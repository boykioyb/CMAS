use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn find_vscode_path() -> Option<String> {
    let candidates = [
        "/usr/local/bin/code",
        "/opt/homebrew/bin/code",
        "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code",
    ];

    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }

    // Try `which code`
    if let Ok(output) = Command::new("which").arg("code").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }

    None
}

/// Ensure the per-account VSCode data directory exists with user settings.
fn ensure_session_data_dir(data_dir: &PathBuf) {
    let user_dir = data_dir.join("User");
    let needs_settings = !user_dir.join("settings.json").exists();

    if needs_settings {
        let _ = std::fs::create_dir_all(&user_dir);
        let home = dirs::home_dir().unwrap_or_default();
        let default_user_dir = home.join("Library/Application Support/Code/User");

        for file in &["settings.json", "keybindings.json"] {
            let src = default_user_dir.join(file);
            if src.exists() {
                let _ = std::fs::copy(&src, user_dir.join(file));
            }
        }
    }
}

/// Set up an isolated HOME directory for a VSCode session.
///
/// Structure:
///   ~/.claude-switcher/homes/{session_key}/
///     ├── Library/Keychains → {real_home}/Library/Keychains
///     ├── .claude → {real_home}/.claude
///     └── .claude.json  (per-account, written separately)
///
/// The symlinked Keychains ensures macOS Security.framework can find the login
/// keychain.  The symlinked .claude shares projects/debug/stats data across
/// sessions.  The per-account .claude.json at HOME root provides isolated
/// oauthAccount config (Claude Code reads $HOME/.claude.json).
fn setup_session_home(real_home: &Path, session_key: &str) -> Result<PathBuf> {
    let session_home = real_home
        .join(".claude-switcher")
        .join("homes")
        .join(session_key);

    std::fs::create_dir_all(&session_home)?;

    // Symlink Library/Keychains → real Keychains
    // This allows Security.framework to find the login keychain even when HOME
    // is overridden, since it resolves $HOME/Library/Keychains/login.keychain-db.
    let lib_dir = session_home.join("Library");
    std::fs::create_dir_all(&lib_dir)?;
    let keychains_link = lib_dir.join("Keychains");
    if !keychains_link.exists() {
        std::os::unix::fs::symlink(
            real_home.join("Library").join("Keychains"),
            &keychains_link,
        )?;
    }

    // Symlink .claude → real .claude (shared projects, debug, stats-cache)
    // This keeps usage tracking and session data unified across all accounts.
    let claude_link = session_home.join(".claude");
    if !claude_link.exists() {
        std::os::unix::fs::symlink(real_home.join(".claude"), &claude_link)?;
    }

    Ok(session_home)
}

/// Write per-session .claude.json with the target account's oauthAccount.
///
/// Placed at $SESSION_HOME/.claude.json so Claude Code extension reads it
/// when HOME is set to the session home directory.
fn write_session_claude_config(
    session_home: &Path,
    oauth_value: &serde_json::Value,
) -> Result<()> {
    let config_path = session_home.join(".claude.json");

    // Start from global config to preserve non-auth fields (hasCompletedOnboarding, etc.)
    let mut config = crate::services::claude_config::read_claude_config()
        .unwrap_or_else(|_| serde_json::json!({}));

    if let Some(obj) = config.as_object_mut() {
        obj.insert("oauthAccount".to_string(), oauth_value.clone());
    }

    // Atomic write: temp file + rename
    let temp_path = config_path.with_extension("json.tmp");
    let json_string = serde_json::to_string_pretty(&config)?;
    std::fs::write(&temp_path, &json_string)?;
    std::fs::rename(&temp_path, &config_path)?;

    Ok(())
}

/// Open VSCode in an isolated session for a specific account.
///
/// Isolation strategy (3 layers):
/// 1. `--user-data-dir` → forces a **separate VSCode Electron process**
///    (the `code` CLI normally IPCs to the running instance; a different
///    data-dir makes it spawn a new Electron process instead)
/// 2. `HOME` override → per-session home with isolated `.claude.json` and
///    symlinked Keychains (so Security.framework still finds the login keychain)
/// 3. `USER` env var → Claude Code extension reads a per-account keychain
///    entry (`-a cmas-{id}`)
///
/// This means switching accounts in CMAS or opening another VSCode window does
/// NOT affect existing sessions — each reads from its own config and keychain.
pub fn open_vscode(
    vscode_path: &str,
    folder: Option<&str>,
    session_user: Option<&str>,
    oauth_config: Option<&serde_json::Value>,
) -> Result<()> {
    let mut cmd = Command::new(vscode_path);
    cmd.arg("--new-window");

    if let Some(user) = session_user {
        let home = dirs::home_dir().unwrap_or_default();

        // Layer 1: Per-session HOME with symlinked Keychains + isolated .claude.json
        let session_home = setup_session_home(&home, user)?;
        if let Some(oauth) = oauth_config {
            write_session_claude_config(&session_home, oauth)?;
        }
        cmd.env("HOME", &session_home);

        // Layer 2: Separate VSCode Electron process
        let session_dir = home
            .join(".claude-switcher")
            .join("vscode-sessions")
            .join(user);
        ensure_session_data_dir(&session_dir);
        cmd.arg("--user-data-dir").arg(&session_dir);

        // Layer 3: Per-account keychain entry
        cmd.env("USER", user);

        // Share extensions from the default location
        let extensions_dir = home.join(".vscode/extensions");
        if extensions_dir.exists() {
            cmd.arg("--extensions-dir").arg(&extensions_dir);
        }
    }

    if let Some(folder_path) = folder {
        cmd.arg(folder_path);
    }

    cmd.spawn()
        .map_err(|e| anyhow::anyhow!("Failed to open VSCode: {}", e))?;
    Ok(())
}
