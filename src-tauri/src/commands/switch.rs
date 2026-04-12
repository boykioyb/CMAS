use crate::models::OAuthAccount;
use crate::services::{claude_config, keychain, vscode};

#[derive(serde::Serialize)]
pub struct SwitchResult {
    pub success: bool,
    pub from_email: Option<String>,
    pub to_email: String,
    pub message: String,
}

/// Swap credentials from current active to target account
/// Updates keychain + claude config, returns the previous active account info
fn swap_credentials(target: &crate::models::Account, current_active: Option<&crate::models::Account>) -> Result<(), String> {
    // 1. Backup current credentials — only if no backup exists yet.
    //    The global keychain may have been temporarily overwritten by
    //    switch_and_open_vscode for a different account, so blindly
    //    backing up from global would corrupt the backup.
    if let Some(active) = current_active {
        if keychain::restore_credentials(&active.id).is_err() {
            if let Ok(creds) = keychain::read_active_credentials() {
                let _ = keychain::backup_credentials(&active.id, &creds);
            }
        }
        // Also save current oauthAccount config if the account doesn't have one yet
        if active.oauth_config.is_none() {
            if let Ok(oauth_cfg) = claude_config::read_full_oauth_account() {
                let mut accounts = super::account::load_accounts();
                if let Some(acc) = accounts.iter_mut().find(|a| a.id == active.id) {
                    acc.oauth_config = Some(oauth_cfg);
                    let _ = super::account::save_accounts(&accounts);
                }
            }
        }
    }

    // 2. Restore target credentials from backup
    let target_creds = keychain::restore_credentials(&target.id)
        .map_err(|e| format!("Failed to restore credentials: {}", e))?;

    // 3. Write target credentials to active keychain slot
    keychain::write_active_credentials(&target_creds)
        .map_err(|e| format!("Failed to write credentials: {}", e))?;

    // 4. Write full oauthAccount config (with all fields)
    if let Some(ref oauth_cfg) = target.oauth_config {
        claude_config::write_full_oauth_account(oauth_cfg)
            .map_err(|e| format!("Failed to update config: {}", e))?;
    } else {
        // Fallback: write minimal oauthAccount
        let oauth = OAuthAccount {
            email_address: target.email.clone(),
            account_uuid: target.account_uuid.clone(),
            extra: serde_json::Map::new(),
        };
        claude_config::write_oauth_account(&oauth)
            .map_err(|e| format!("Failed to update config: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub fn switch_account(target_id: String) -> Result<SwitchResult, String> {
    let accounts = super::account::load_accounts();

    let target = accounts
        .iter()
        .find(|a| a.id == target_id)
        .ok_or("Target account not found")?
        .clone();

    let current_active = accounts.iter().find(|a| a.is_active).cloned();

    // Swap credentials (keychain + config)
    swap_credentials(&target, current_active.as_ref())?;

    // Update account states in storage
    let mut accounts = super::account::load_accounts();
    let now = chrono::Utc::now().to_rfc3339();
    for acc in accounts.iter_mut() {
        if acc.id == target_id {
            acc.is_active = true;
            acc.last_switched_at = Some(now.clone());
            acc.last_used_at = Some(now.clone());
        } else {
            acc.is_active = false;
        }
    }
    // Save updated accounts
    super::account::save_accounts(&accounts)?;

    Ok(SwitchResult {
        success: true,
        from_email: current_active.map(|a| a.email),
        to_email: target.email,
        message: "Account switched successfully".to_string(),
    })
}

/// Open VSCode with a specific account's credentials in a **fully isolated session**.
///
/// Isolation strategy (3 layers):
/// 1. `--user-data-dir` → separate Electron process per account
/// 2. `HOME` override → per-session home dir with:
///    - `Library/Keychains` → symlink to real keychains (Security.framework works)
///    - `.claude` → symlink to real .claude (shared projects/debug/stats)
///    - `.claude.json` → per-account file (isolated oauthAccount)
/// 3. `USER` env var → per-account keychain entry (`-a cmas-{id}`)
///
/// This means:
/// - Multiple VSCode windows can run different accounts simultaneously
/// - Switching accounts in CMAS (via `switch_account`) does NOT affect
///   existing VSCode windows
/// - The global active account and keychain are NOT touched
#[tauri::command]
pub fn switch_and_open_vscode(
    target_id: String,
    vscode_path: Option<String>,
    folder_path: Option<String>,
) -> Result<SwitchResult, String> {
    let accounts = super::account::load_accounts();

    let target = accounts
        .iter()
        .find(|a| a.id == target_id)
        .ok_or("Target account not found")?
        .clone();

    // Get the target account's selected project if no explicit folder
    let project_folder = folder_path.or_else(|| {
        target.selected_project.and_then(|idx| {
            target.projects.get(idx).map(|p| p.path.clone())
        })
    });

    // 1. Get target credentials from backup
    let target_creds = keychain::restore_credentials(&target.id)
        .map_err(|e| format!("Failed to restore credentials: {}", e))?;

    // 2. Write credentials to global keychain (extension reads `-a {os_user}`)
    //    AND per-session keychain (for future isolation).
    //    The extension does NOT respect $USER for keychain reads — it uses the
    //    real OS username.  So we must write to global for the extension to work.
    let session_key = format!("cmas-{}", &target.id[..8.min(target.id.len())]);
    keychain::write_active_credentials(&target_creds)
        .map_err(|e| format!("Failed to write credentials: {}", e))?;
    let _ = keychain::write_session_credentials(&session_key, &target_creds);

    // 3. Write oauthAccount to BOTH global and per-session .claude.json.
    //    The extension reads account display info (email, org) from the real
    //    ~/.claude.json at startup — it does NOT respect $HOME override for this.
    //    The per-session copy is written by open_vscode via session HOME.
    let oauth_for_session = target.oauth_config.clone().unwrap_or_else(|| {
        serde_json::json!({
            "emailAddress": target.email,
            "accountUuid": target.account_uuid
        })
    });
    claude_config::write_full_oauth_account(&oauth_for_session)
        .map_err(|e| format!("Failed to update config: {}", e))?;

    // 4. Open VSCode with full isolation (HOME + USER + --user-data-dir)
    let path = vscode_path
        .or_else(|| vscode::find_vscode_path())
        .unwrap_or_else(|| "/usr/local/bin/code".to_string());

    vscode::open_vscode(&path, project_folder.as_deref(), Some(&session_key), Some(&oauth_for_session))
        .map_err(|e| e.to_string())?;

    // 5. Only update last_used_at — do NOT change is_active or global state
    let mut accounts = super::account::load_accounts();
    let now = chrono::Utc::now().to_rfc3339();
    if let Some(acc) = accounts.iter_mut().find(|a| a.id == target_id) {
        acc.last_used_at = Some(now);
    }
    super::account::save_accounts(&accounts)?;

    Ok(SwitchResult {
        success: true,
        from_email: None,
        to_email: target.email,
        message: "Opened VSCode with isolated session".to_string(),
    })
}

#[tauri::command]
pub fn switch_to_best_account() -> Result<SwitchResult, String> {
    let accounts = super::account::load_accounts();

    // Find best account: prefer non-rate-limited with fewest messages today
    let best = accounts
        .iter()
        .filter(|a| !a.is_active && a.status == crate::models::AccountStatus::Ok)
        .min_by(|a, b| {
            // Rate-limited accounts sort last
            let a_limited = a.usage.is_rate_limited as u8;
            let b_limited = b.usage.is_rate_limited as u8;
            a_limited.cmp(&b_limited).then_with(|| {
                // Among non-limited, prefer fewer messages today
                a.usage.messages_today.cmp(&b.usage.messages_today)
            })
        })
        .ok_or("No suitable account found")?;

    switch_account(best.id.clone())
}
