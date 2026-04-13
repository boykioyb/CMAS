use anyhow::{Context, Result};

const ACTIVE_SERVICE: &str = "Claude Code-credentials";

fn backup_service(account_id: &str) -> String {
    format!("CMAS-Account-{}", account_id)
}

/// Get the OS username, matching how Claude Code CLI/extension reads keychain.
/// The extension uses `process.env.USER || os.userInfo().username` as the
/// keychain account name, NOT "claude-code".
fn get_os_username() -> String {
    #[cfg(target_os = "windows")]
    let var = "USERNAME";
    #[cfg(not(target_os = "windows"))]
    let var = "USER";

    std::env::var(var).unwrap_or_else(|_| {
        // Fallback: call `whoami` command (works on both macOS and Windows)
        std::process::Command::new("whoami")
            .output()
            .map(|o| {
                let raw = String::from_utf8_lossy(&o.stdout).trim().to_string();
                // On Windows, whoami may return DOMAIN\username — extract just username
                #[cfg(target_os = "windows")]
                {
                    raw.rsplit('\\').next().unwrap_or(&raw).to_string()
                }
                #[cfg(not(target_os = "windows"))]
                {
                    raw
                }
            })
            .unwrap_or_else(|_| "claude-code-user".to_string())
    })
}

pub fn read_active_credentials() -> Result<String> {
    let username = get_os_username();
    let entry = keyring::Entry::new(ACTIVE_SERVICE, &username)
        .context("Failed to create keyring entry")?;

    entry
        .get_password()
        .map_err(|_| anyhow::anyhow!("No active credentials found in Keychain"))
}

/// Core function: write credentials to keychain for a specific account name.
/// The account name determines which keychain entry the Claude Code
/// extension will read.  The extension uses `process.env.USER` as the account
/// name, so by varying the account name we can isolate per-VSCode sessions.
fn write_credentials_for_user(account_name: &str, creds: &str) -> Result<()> {
    let entry = keyring::Entry::new(ACTIVE_SERVICE, account_name)
        .context("Failed to create keyring entry")?;

    // Delete existing entry first (ignore errors)
    let _ = entry.delete_credential();

    entry
        .set_password(creds)
        .map_err(|e| anyhow::anyhow!("Failed to write credentials to Keychain: {}", e))
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
    let entry = keyring::Entry::new(&service, "claude-code")
        .context("Failed to create keyring entry")?;

    // Delete existing entry first (ignore errors)
    let _ = entry.delete_credential();

    entry
        .set_password(creds)
        .map_err(|e| anyhow::anyhow!("Failed to backup credentials: {}", e))
}

pub fn restore_credentials(account_id: &str) -> Result<String> {
    let service = backup_service(account_id);
    let entry = keyring::Entry::new(&service, "claude-code")
        .context("Failed to create keyring entry")?;

    entry.get_password().map_err(|_| {
        anyhow::anyhow!(
            "No backup credentials found for account {}",
            account_id
        )
    })
}

pub fn delete_credentials(account_id: &str) -> Result<()> {
    let service = backup_service(account_id);
    let entry = keyring::Entry::new(&service, "claude-code")
        .context("Failed to create keyring entry")?;

    let _ = entry.delete_credential();
    Ok(())
}

/// Sync active keychain credentials to the active account's backup.
/// The Claude CLI refreshes OAuth tokens automatically in the active keychain
/// entry, but CMAS backup copies become stale. Call this on startup to keep
/// the active account's backup in sync with the CLI-refreshed token.
pub fn sync_active_credentials_to_backup(active_account_id: &str) {
    if let Ok(current_creds) = read_active_credentials() {
        if !current_creds.is_empty() {
            let _ = backup_credentials(active_account_id, &current_creds);
        }
    }
}

/// One-time migration: if the old keychain entry (account="claude-code") exists
/// but the correct entry (account=OS_USERNAME) does not, copy it over.
/// Also cleans up the stale old entry.
///
/// Additionally handles migration from the old `security` CLI hex-encoded format
/// to the new `keyring` crate format on macOS.
pub fn migrate_keychain_account_name() {
    let username = get_os_username();
    if username == "claude-code" {
        return; // Nothing to migrate
    }

    // Check if the correct entry already exists
    let correct_entry = match keyring::Entry::new(ACTIVE_SERVICE, &username) {
        Ok(e) => e,
        Err(_) => return,
    };

    if correct_entry.get_password().is_ok() {
        // Correct entry exists; clean up old entry if present
        if let Ok(old_entry) = keyring::Entry::new(ACTIVE_SERVICE, "claude-code") {
            let _ = old_entry.delete_credential();
        }
        return;
    }

    // Try reading from the old "claude-code" account entry
    if let Ok(old_entry) = keyring::Entry::new(ACTIVE_SERVICE, "claude-code") {
        if let Ok(creds) = old_entry.get_password() {
            if !creds.is_empty() {
                // Write to the correct entry
                let _ = write_active_credentials(&creds);
                // Delete old entry
                let _ = old_entry.delete_credential();
                return;
            }
        }
    }

    // macOS fallback: try reading via the old `security` CLI in case credentials
    // were stored with hex encoding that the keyring crate can't read directly.
    #[cfg(target_os = "macos")]
    {
        let old_creds = std::process::Command::new("security")
            .args([
                "find-generic-password",
                "-s",
                ACTIVE_SERVICE,
                "-a",
                "claude-code",
                "-w",
            ])
            .output();

        if let Ok(output) = old_creds {
            if output.status.success() {
                let creds = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !creds.is_empty() {
                    let _ = write_active_credentials(&creds);
                    let _ = std::process::Command::new("security")
                        .args([
                            "delete-generic-password",
                            "-s",
                            ACTIVE_SERVICE,
                            "-a",
                            "claude-code",
                        ])
                        .output();
                }
            }
        }
    }
}
