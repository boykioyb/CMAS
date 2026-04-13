use anyhow::{Context, Result};
use std::process::Command;

/// Resolve the claude CLI binary path.
/// Priority: user config > auto-detect > bare "claude".
fn cli_path() -> String {
    // Check user-configured path first
    if let Some(app_cfg) = read_app_config() {
        if !app_cfg.claude_cli_path.is_empty()
            && std::path::Path::new(&app_cfg.claude_cli_path).exists()
        {
            return app_cfg.claude_cli_path;
        }
    }
    find_claude_cli().unwrap_or_else(|| "claude".to_string())
}

/// Read the user's CMAS app config.
fn read_app_config() -> Option<crate::models::AppConfig> {
    let home = dirs::home_dir()?;
    let path = home.join(".claude-switcher").join("config.json");
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Get auth status via `claude auth status`
pub fn get_auth_status() -> Result<AuthStatus> {
    let claude = cli_path();

    let output = Command::new(&claude)
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
    let output = Command::new(&claude)
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
    let claude = cli_path();

    let output = Command::new(&claude)
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
    #[cfg(target_os = "macos")]
    {
        // Check well-known install locations first
        let mut candidates = vec![
            "/usr/local/bin/claude".to_string(),
            "/opt/homebrew/bin/claude".to_string(),
        ];

        // ~/.local/bin/claude (official Claude Code installer)
        if let Some(home) = dirs::home_dir() {
            candidates.push(home.join(".local/bin/claude").to_string_lossy().to_string());
        }

        for path in &candidates {
            if std::path::Path::new(path).exists() {
                return Some(path.clone());
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
            let candidate = format!("{}\\Programs\\claude\\claude.exe", local_app);
            if std::path::Path::new(&candidate).exists() {
                return Some(candidate);
            }
        }
        if let Ok(program_files) = std::env::var("ProgramFiles") {
            let candidate = format!("{}\\claude\\claude.exe", program_files);
            if std::path::Path::new(&candidate).exists() {
                return Some(candidate);
            }
        }
        // ~/.local/bin/claude.exe (official installer)
        if let Some(home) = dirs::home_dir() {
            let candidate = home.join(".local\\bin\\claude.exe");
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }

    // Linux: check ~/.local/bin/claude
    #[cfg(target_os = "linux")]
    {
        if let Some(home) = dirs::home_dir() {
            let candidate = home.join(".local/bin/claude");
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }

    // Fallback: which (macOS/Linux) / where (Windows)
    #[cfg(target_os = "windows")]
    let lookup_cmd = "where";
    #[cfg(not(target_os = "windows"))]
    let lookup_cmd = "which";

    if let Ok(output) = Command::new(lookup_cmd).arg("claude").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
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
