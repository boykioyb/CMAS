//! Per-account credential file store.
//!
//! Replaces the legacy `CMAS-Account-{id}` keychain entries with flat files
//! under `~/.claude-switcher/credentials/<id>.json` (mode 0600 on Unix).
//!
//! The active keychain entry (`Claude Code-credentials` + OS username) is
//! still managed by `keychain.rs` — that one is the contract with Claude CLI
//! and cannot be replaced.

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

fn credentials_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Cannot resolve home directory")?;
    let dir = home.join(".claude-switcher").join("credentials");
    if !dir.exists() {
        fs::create_dir_all(&dir).with_context(|| {
            format!("Failed to create credentials dir: {}", dir.display())
        })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o700));
        }
    }
    Ok(dir)
}

fn credential_path(account_id: &str) -> Result<PathBuf> {
    if account_id.is_empty() || account_id.contains('/') || account_id.contains('\\') {
        return Err(anyhow::anyhow!("Invalid account_id: {}", account_id));
    }
    Ok(credentials_dir()?.join(format!("{}.json", account_id)))
}

/// Store credentials for an account. Validates JSON before writing.
pub fn store(account_id: &str, creds: &str) -> Result<()> {
    serde_json::from_str::<serde_json::Value>(creds)
        .context("Refusing to store: credentials are not valid JSON")?;

    let path = credential_path(account_id)?;
    let tmp = path.with_extension("json.tmp");

    fs::write(&tmp, creds)
        .with_context(|| format!("Failed to write {}", tmp.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))
            .with_context(|| format!("Failed to chmod {}", tmp.display()))?;
    }

    fs::rename(&tmp, &path)
        .with_context(|| format!("Failed to rename {} -> {}", tmp.display(), path.display()))?;

    Ok(())
}

/// Load credentials for an account.
pub fn load(account_id: &str) -> Result<String> {
    let path = credential_path(account_id)?;
    fs::read_to_string(&path)
        .with_context(|| format!("No credentials found for account {}", account_id))
}

/// Delete credentials for an account. Idempotent — succeeds if file is absent.
pub fn delete(account_id: &str) -> Result<()> {
    let path = credential_path(account_id)?;
    match fs::remove_file(&path) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).with_context(|| format!("Failed to delete {}", path.display())),
    }
}

/// Check if credentials exist for an account.
pub fn exists(account_id: &str) -> bool {
    credential_path(account_id)
        .map(|p| p.exists())
        .unwrap_or(false)
}
