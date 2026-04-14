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

// ── Platform-specific keychain primitives ────────────────────────────────
//
// macOS: use `/usr/bin/security` CLI so the ACL is tied to a stable system
//        binary instead of the CMAS app binary.  This prevents the "wants to
//        use your confidential information" prompt after every app update.
//
// Windows / Linux: use the `keyring` crate (Credential Manager / Secret
//                  Service) which doesn't have the same ACL issue.

#[cfg(target_os = "macos")]
fn kc_read(service: &str, account: &str) -> Result<String> {
    let output = std::process::Command::new("security")
        .args(["find-generic-password", "-s", service, "-a", account, "-w"])
        .output()
        .context("Failed to run security command")?;

    if output.status.success() {
        let password = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !password.is_empty() {
            return Ok(password);
        }
    }
    Err(anyhow::anyhow!("No credentials found for {}/{}", service, account))
}

#[cfg(target_os = "macos")]
fn kc_write(service: &str, account: &str, password: &str) -> Result<()> {
    // Delete existing entry first (ignore errors).
    // This also removes any restrictive ACL from entries created by other apps.
    let _ = std::process::Command::new("security")
        .args(["delete-generic-password", "-s", service, "-a", account])
        .output();

    // -A = allow ANY application to access without confirmation dialog.
    // Without this, each `security find-generic-password` call triggers a
    // macOS keychain prompt asking for the login password.
    let output = std::process::Command::new("security")
        .args([
            "add-generic-password",
            "-A",
            "-s", service,
            "-a", account,
            "-w", password,
        ])
        .output()
        .context("Failed to run security command")?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("Failed to write to keychain: {}", stderr))
    }
}

#[cfg(target_os = "macos")]
fn kc_delete(service: &str, account: &str) {
    let _ = std::process::Command::new("security")
        .args(["delete-generic-password", "-s", service, "-a", account])
        .output();
}

#[cfg(not(target_os = "macos"))]
fn kc_read(service: &str, account: &str) -> Result<String> {
    let entry = keyring::Entry::new(service, account)
        .context("Failed to create keyring entry")?;
    entry
        .get_password()
        .map_err(|_| anyhow::anyhow!("No credentials found for {}/{}", service, account))
}

#[cfg(not(target_os = "macos"))]
fn kc_write(service: &str, account: &str, password: &str) -> Result<()> {
    let entry = keyring::Entry::new(service, account)
        .context("Failed to create keyring entry")?;
    let _ = entry.delete_credential();
    entry
        .set_password(password)
        .map_err(|e| anyhow::anyhow!("Failed to write credentials: {}", e))
}

#[cfg(not(target_os = "macos"))]
fn kc_delete(service: &str, account: &str) {
    if let Ok(entry) = keyring::Entry::new(service, account) {
        let _ = entry.delete_credential();
    }
}

// ── Public API (platform-agnostic) ───────────────────────────────────────

pub fn read_active_credentials() -> Result<String> {
    let username = get_os_username();
    kc_read(ACTIVE_SERVICE, &username)
}

/// Core function: write credentials to keychain for a specific account name.
fn write_credentials_for_user(account_name: &str, creds: &str) -> Result<()> {
    kc_write(ACTIVE_SERVICE, account_name, creds)
}

/// Write credentials to the global keychain entry (used by CLI and default
/// VSCode instances).  Uses the OS username as account name.
pub fn write_active_credentials(creds: &str) -> Result<()> {
    let username = get_os_username();
    write_credentials_for_user(&username, creds)
}

/// Write credentials to an isolated per-session keychain entry.
pub fn write_session_credentials(session_key: &str, creds: &str) -> Result<()> {
    write_credentials_for_user(session_key, creds)
}

pub fn backup_credentials(account_id: &str, creds: &str) -> Result<()> {
    let service = backup_service(account_id);
    kc_write(&service, "claude-code", creds)
}

pub fn restore_credentials(account_id: &str) -> Result<String> {
    let service = backup_service(account_id);
    kc_read(&service, "claude-code")
}

pub fn delete_credentials(account_id: &str) -> Result<()> {
    let service = backup_service(account_id);
    kc_delete(&service, "claude-code");
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
    if kc_read(ACTIVE_SERVICE, &username).is_ok() {
        // Correct entry exists; clean up old entry if present
        kc_delete(ACTIVE_SERVICE, "claude-code");
        return;
    }

    // Try reading from the old "claude-code" account entry
    if let Ok(creds) = kc_read(ACTIVE_SERVICE, "claude-code") {
        if !creds.is_empty() {
            let _ = write_active_credentials(&creds);
            kc_delete(ACTIVE_SERVICE, "claude-code");
        }
    }
}
