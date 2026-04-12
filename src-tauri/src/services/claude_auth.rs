use anyhow::{Context, Result};
use std::process::Command;

/// Get auth status via `claude auth status`
pub fn get_auth_status() -> Result<AuthStatus> {
    let output = Command::new("claude")
        .args(["auth", "status", "--json"])
        .output()
        .context("Failed to run claude auth status. Is Claude CLI installed?")?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Try parsing JSON output
        if let Ok(status) = serde_json::from_str::<AuthStatus>(&stdout) {
            return Ok(status);
        }
    }

    // Fallback: try without --json
    let output = Command::new("claude")
        .args(["auth", "status"])
        .output()
        .context("Failed to run claude auth status")?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if let Ok(status) = serde_json::from_str::<AuthStatus>(&stdout) {
        Ok(status)
    } else {
        // Parse text output
        Ok(AuthStatus {
            logged_in: stdout.contains("true") || stdout.contains("loggedIn"),
            email: None,
            org_name: None,
            subscription_type: None,
        })
    }
}

/// Run `claude auth logout`
pub fn logout() -> Result<()> {
    let output = Command::new("claude")
        .args(["auth", "logout"])
        .output()
        .context("Failed to run claude auth logout")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::warn!("claude auth logout stderr: {}", stderr);
        // Not a hard error - might already be logged out
    }
    Ok(())
}

/// Start OAuth login silently in background
/// `claude auth login` opens the browser for OAuth automatically,
/// no terminal window needed - the process runs in background
pub fn start_login() -> Result<()> {
    let claude_path = find_claude_cli()
        .unwrap_or_else(|| "claude".to_string());

    // Run claude auth login as a background process
    // It will open the browser for OAuth, and write credentials when done
    Command::new(&claude_path)
        .args(["auth", "login", "--claudeai"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("Failed to start OAuth login. Ensure Claude CLI is installed.")?;

    Ok(())
}

/// Find the claude CLI path
pub fn find_claude_cli() -> Option<String> {
    let candidates = [
        "/usr/local/bin/claude",
        "/opt/homebrew/bin/claude",
    ];

    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }

    // Try `which claude`
    if let Ok(output) = Command::new("which").arg("claude").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }

    None
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthStatus {
    #[serde(rename = "loggedIn", alias = "logged_in")]
    pub logged_in: bool,
    pub email: Option<String>,
    #[serde(rename = "orgName", alias = "org_name")]
    pub org_name: Option<String>,
    #[serde(rename = "subscriptionType", alias = "subscription_type")]
    pub subscription_type: Option<String>,
}
