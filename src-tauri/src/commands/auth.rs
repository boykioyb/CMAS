use crate::models::{Account, AccountPlan, AccountStatus, UsageInfo};
use crate::services::{claude_auth, claude_config, keychain};

/// Step 1: Backup current credentials before starting OAuth for new account
#[tauri::command]
pub fn auth_backup_current() -> Result<Option<String>, String> {
    // Read current account email (if any)
    let current_email = claude_config::get_active_oauth_account()
        .ok()
        .map(|a| a.email_address);

    // Backup current credentials to a temp slot.
    // After reading, re-write with -A ACL so future reads are prompt-free.
    if let Ok(creds) = keychain::read_active_credentials() {
        keychain::backup_credentials("__temp_current__", &creds)
            .map_err(|e| format!("Failed to backup credentials: {}", e))?;
        let _ = keychain::write_active_credentials(&creds);
    }

    Ok(current_email)
}

/// Step 2: Logout current account and open OAuth login in Terminal
#[tauri::command]
pub fn auth_start_login() -> Result<(), String> {
    // Check if claude CLI is available
    if claude_auth::find_claude_cli().is_none() {
        return Err("Claude CLI not found. Please install Claude Code first.".to_string());
    }

    // Logout current account
    claude_auth::logout().map_err(|e| format!("Logout failed: {}", e))?;

    // Start OAuth login in Terminal
    claude_auth::start_login().map_err(|e| format!("Failed to open Terminal: {}", e))?;

    Ok(())
}

/// Step 3: Poll to check if new OAuth login is complete
#[tauri::command]
pub fn auth_check_login_status() -> Result<AuthCheckResult, String> {
    // Check auth status via CLI
    match claude_auth::get_auth_status() {
        Ok(status) => {
            if status.logged_in {
                // Read the new account info
                let oauth = claude_config::get_active_oauth_account()
                    .map_err(|e| e.to_string())?;

                Ok(AuthCheckResult {
                    logged_in: true,
                    email: Some(oauth.email_address),
                    account_uuid: Some(oauth.account_uuid),
                    org_name: status.org_name,
                    subscription_type: status.subscription_type,
                })
            } else {
                Ok(AuthCheckResult {
                    logged_in: false,
                    email: None,
                    account_uuid: None,
                    org_name: None,
                    subscription_type: None,
                })
            }
        }
        Err(_) => Ok(AuthCheckResult {
            logged_in: false,
            email: None,
            account_uuid: None,
            org_name: None,
            subscription_type: None,
        }),
    }
}

/// Step 4: Confirm and save the new account after successful OAuth
#[tauri::command]
pub fn auth_confirm_new_account(label: Option<String>) -> Result<Account, String> {
    // Read the newly logged-in account
    let oauth = claude_config::get_active_oauth_account()
        .map_err(|e| format!("Failed to read new account: {}", e))?;

    let mut accounts = super::account::load_accounts();

    // Read credentials ONCE from active keychain (may prompt on macOS),
    // then re-write with -A ACL so future reads are prompt-free.
    let active_creds = keychain::read_active_credentials().ok();
    if let Some(ref creds) = active_creds {
        let _ = keychain::write_active_credentials(creds);
    }

    // Check if already exists
    if let Some(existing) = accounts.iter().find(|a| a.account_uuid == oauth.account_uuid).cloned() {
        // Account exists - update credentials backup and oauth_config
        if let Some(ref creds) = active_creds {
            let _ = keychain::backup_credentials(&existing.id, creds);
        }
        // Update oauth_config for existing account
        if let Ok(oauth_cfg) = claude_config::read_full_oauth_account() {
            if let Some(acc) = accounts.iter_mut().find(|a| a.id == existing.id) {
                acc.oauth_config = Some(oauth_cfg);
                let _ = super::account::save_accounts(&accounts);
            }
        }
        return Err(format!("Account {} already exists. Credentials updated.", oauth.email_address));
    }

    // Save new account
    let id = uuid::Uuid::new_v4().to_string();

    // Backup the new account's credentials
    if let Some(ref creds) = active_creds {
        keychain::backup_credentials(&id, creds)
            .map_err(|e| format!("Failed to backup new credentials: {}", e))?;
    }

    // Save the full oauthAccount config blob
    let oauth_config = claude_config::read_full_oauth_account().ok();

    // Detect plan from already-read credentials (no second keychain read)
    let plan = if let Some(ref creds_str) = active_creds {
        if let Ok(creds) = serde_json::from_str::<serde_json::Value>(creds_str) {
            let sub_type = creds
                .get("claudeAiOauth")
                .and_then(|o| o.get("subscriptionType"))
                .and_then(|v| v.as_str());
            match sub_type {
                Some("free") => AccountPlan::Free,
                _ => AccountPlan::Pro,
            }
        } else {
            AccountPlan::Pro
        }
    } else {
        AccountPlan::Pro
    };

    let now = chrono::Utc::now().to_rfc3339();
    let account = Account {
        id: id.clone(),
        email: oauth.email_address.clone(),
        label,
        account_uuid: oauth.account_uuid.clone(),
        plan,
        added_at: now.clone(),
        last_used_at: Some(now.clone()),
        last_switched_at: None,
        is_active: false, // Not active yet - will switch explicitly
        status: AccountStatus::Ok,
        usage: UsageInfo::default(),
        projects: Vec::new(),
        selected_project: None,
        oauth_config,
    };

    accounts.push(account.clone());
    super::account::save_accounts(&accounts)?;

    Ok(account)
}

/// Step 5: Restore original account after adding new one
#[tauri::command]
pub fn auth_restore_original() -> Result<(), String> {
    // Restore credentials from temp backup
    if let Ok(creds) = keychain::restore_credentials("__temp_current__") {
        keychain::write_active_credentials(&creds)
            .map_err(|e| format!("Failed to restore credentials: {}", e))?;
    }

    // Clean up temp backup
    let _ = keychain::delete_credentials("__temp_current__");

    // Also need to restore the original oauth config
    let accounts = super::account::load_accounts();
    if let Some(active) = accounts.iter().find(|a| a.is_active) {
        if let Some(ref oauth_cfg) = active.oauth_config {
            let _ = claude_config::write_full_oauth_account(oauth_cfg);
        } else {
            let oauth = crate::models::OAuthAccount {
                email_address: active.email.clone(),
                account_uuid: active.account_uuid.clone(),
                extra: serde_json::Map::new(),
            };
            let _ = claude_config::write_oauth_account(&oauth);
        }
    }

    Ok(())
}

/// Get current auth status
#[tauri::command]
pub fn auth_get_status() -> Result<AuthCheckResult, String> {
    auth_check_login_status()
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct AuthCheckResult {
    pub logged_in: bool,
    pub email: Option<String>,
    pub account_uuid: Option<String>,
    pub org_name: Option<String>,
    pub subscription_type: Option<String>,
}
