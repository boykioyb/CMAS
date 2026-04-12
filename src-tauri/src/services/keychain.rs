use anyhow::{Context, Result};
use std::process::Command;

const ACTIVE_SERVICE: &str = "Claude Code-credentials";

fn backup_service(account_id: &str) -> String {
    format!("CMAS-Account-{}", account_id)
}

/// Get the OS username, matching how Claude Code CLI/extension reads keychain.
/// The extension uses `process.env.USER || os.userInfo().username` as the
/// keychain account name (-a flag), NOT "claude-code".
fn get_os_username() -> String {
    std::env::var("USER").unwrap_or_else(|_| {
        // Fallback: call `whoami` command
        Command::new("whoami")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "claude-code-user".to_string())
    })
}

/// Encode bytes as hex string (matching the extension's Buffer.from(str, "utf-8").toString("hex"))
fn to_hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn read_active_credentials() -> Result<String> {
    let username = get_os_username();
    let output = Command::new("security")
        .args(["find-generic-password", "-s", ACTIVE_SERVICE, "-a", &username, "-w"])
        .output()
        .context("Failed to execute security command")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(anyhow::anyhow!("No active credentials found in Keychain"))
    }
}

/// Core function: write credentials to keychain for a specific account name.
/// The account name (-a) determines which keychain entry the Claude Code
/// extension will read.  The extension uses `process.env.USER` as the account
/// name, so by varying the account name we can isolate per-VSCode sessions.
fn write_credentials_for_user(account_name: &str, creds: &str) -> Result<()> {
    // Delete existing entry first (ignore errors)
    let _ = Command::new("security")
        .args(["delete-generic-password", "-s", ACTIVE_SERVICE, "-a", account_name])
        .output();

    // Write with hex encoding via `security -i`.
    // NOTE: Do NOT quote the hex data — `security -i` treats quotes literally.
    let hex_creds = to_hex(creds.as_bytes());
    let security_input = format!(
        "add-generic-password -U -a \"{}\" -s \"{}\" -X {}\n",
        account_name, ACTIVE_SERVICE, hex_creds
    );

    let output = Command::new("security")
        .arg("-i")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                stdin.write_all(security_input.as_bytes())?;
            }
            child.wait_with_output()
        })
        .context("Failed to write credentials to Keychain")?;

    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Failed to write credentials to Keychain: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// Write credentials to the global keychain entry (used by CLI and default
/// VSCode instances).  Uses the OS username as account name.
pub fn write_active_credentials(creds: &str) -> Result<()> {
    let username = get_os_username();
    write_credentials_for_user(&username, creds)
}

/// Write credentials to an isolated per-session keychain entry.
/// Used when opening VSCode — each VSCode instance gets its own keychain entry
/// so switching accounts elsewhere doesn't affect it.
pub fn write_session_credentials(session_key: &str, creds: &str) -> Result<()> {
    write_credentials_for_user(session_key, creds)
}

pub fn backup_credentials(account_id: &str, creds: &str) -> Result<()> {
    let service = backup_service(account_id);
    let _ = Command::new("security")
        .args(["delete-generic-password", "-s", &service])
        .output();

    let output = Command::new("security")
        .args([
            "add-generic-password",
            "-s",
            &service,
            "-a",
            "claude-code",
            "-w",
            creds,
            "-U",
        ])
        .output()
        .context("Failed to backup credentials")?;

    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to backup credentials"))
    }
}

pub fn restore_credentials(account_id: &str) -> Result<String> {
    let service = backup_service(account_id);
    let output = Command::new("security")
        .args(["find-generic-password", "-s", &service, "-w"])
        .output()
        .context("Failed to read backup credentials")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(anyhow::anyhow!(
            "No backup credentials found for account {}",
            account_id
        ))
    }
}

pub fn delete_credentials(account_id: &str) -> Result<()> {
    let service = backup_service(account_id);
    let _ = Command::new("security")
        .args(["delete-generic-password", "-s", &service])
        .output();
    Ok(())
}

/// One-time migration: if the old keychain entry (account="claude-code") exists
/// but the correct entry (account=OS_USERNAME) does not, copy it over.
/// Also cleans up the stale old entry.
pub fn migrate_keychain_account_name() {
    let username = get_os_username();
    if username == "claude-code" {
        return; // Nothing to migrate
    }

    // Check if the correct entry already exists
    let has_correct = Command::new("security")
        .args(["find-generic-password", "-s", ACTIVE_SERVICE, "-a", &username, "-w"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if has_correct {
        // Correct entry exists; clean up old entry if present
        let _ = Command::new("security")
            .args(["delete-generic-password", "-s", ACTIVE_SERVICE, "-a", "claude-code"])
            .output();
        return;
    }

    // Try reading from the old entry
    let old_creds = Command::new("security")
        .args(["find-generic-password", "-s", ACTIVE_SERVICE, "-a", "claude-code", "-w"])
        .output();

    if let Ok(output) = old_creds {
        if output.status.success() {
            let creds = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !creds.is_empty() {
                // Write to the correct entry
                let _ = write_active_credentials(&creds);
                // Delete old entry
                let _ = Command::new("security")
                    .args(["delete-generic-password", "-s", ACTIVE_SERVICE, "-a", "claude-code"])
                    .output();
            }
        }
    }
}
