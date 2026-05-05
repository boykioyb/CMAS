use crate::models::{Account, AccountPlan, AccountStatus, AccountUpdate, OAuthAccount, UsageInfo};
use crate::services::{claude_config, keychain, project_registry, usage_tracker};
use std::fs;
use std::path::PathBuf;

fn get_accounts_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_default();
    home.join(".claude-switcher").join("accounts.json")
}

fn ensure_backup_dir() {
    let home = dirs::home_dir().unwrap_or_default();
    let dir = home.join(".claude-switcher");
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }
}

pub fn load_accounts() -> Vec<Account> {
    let path = get_accounts_path();
    let mut accounts: Vec<Account> = if let Ok(content) = fs::read_to_string(&path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    if migrate_legacy_projects(&mut accounts) {
        let _ = save_accounts(&accounts);
    }

    accounts
}

/// One-shot migration: move embedded `projects` into the registry and
/// populate `project_ids` / `selected_project_id`. Returns true if anything changed.
fn migrate_legacy_projects(accounts: &mut [Account]) -> bool {
    let needs_migration = accounts
        .iter()
        .any(|a| !a.projects.is_empty() && a.project_ids.is_empty());
    if !needs_migration {
        return false;
    }

    let mut registry = project_registry::load();
    let mut changed = false;

    for acc in accounts.iter_mut() {
        if acc.projects.is_empty() || !acc.project_ids.is_empty() {
            continue;
        }
        let mut new_ids = Vec::with_capacity(acc.projects.len());
        for proj in &acc.projects {
            let id = project_registry::upsert_manual(&mut registry, &proj.name, &proj.path);
            if !new_ids.contains(&id) {
                new_ids.push(id);
            }
        }
        if let Some(idx) = acc.selected_project {
            if let Some(picked_path) = acc.projects.get(idx).map(|p| project_registry::canonicalize(&p.path)) {
                if let Some(p) = registry.projects.iter().find(|p| p.path == picked_path) {
                    acc.selected_project_id = Some(p.id.clone());
                }
            }
        }
        if acc.selected_project_id.is_none() {
            acc.selected_project_id = new_ids.first().cloned();
        }
        acc.project_ids = new_ids;
        // Keep `projects` populated for one release as a safety net
        changed = true;
    }

    if changed {
        let _ = project_registry::save(&registry);
    }
    changed
}


pub fn save_accounts(accounts: &[Account]) -> Result<(), String> {
    ensure_backup_dir();
    let path = get_accounts_path();
    let json = serde_json::to_string_pretty(accounts).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_accounts() -> Result<Vec<Account>, String> {
    let accounts = load_accounts();
    Ok(accounts)
}

/// Refresh usage for all accounts in a single pass (avoids redundant JSONL scanning).
/// Returns updated account list with fresh usage data.
#[tauri::command]
pub fn refresh_all_usage() -> Result<Vec<Account>, String> {
    let mut accounts = load_accounts();

    // Scan rate limit info once
    let rate_limit_at = usage_tracker::find_last_rate_limit();
    let (subscription_type, rate_limit_tier) = usage_tracker::read_credential_metadata_pub();

    let registry = project_registry::load();
    for acc in accounts.iter_mut() {
        let project_paths: Vec<String> = acc
            .project_ids
            .iter()
            .filter_map(|id| registry.projects.iter().find(|p| &p.id == id))
            .map(|p| p.path.clone())
            .collect();
        if !project_paths.is_empty() {
            acc.usage = usage_tracker::get_usage_info_for_projects(&project_paths);
        } else if acc.is_active {
            acc.usage = usage_tracker::get_usage_info();
        }
        // Share common metadata to avoid redundant lookups
        if acc.is_active {
            acc.usage.last_rate_limit_at = rate_limit_at.clone();
            acc.usage.subscription_type = subscription_type.clone();
            acc.usage.rate_limit_tier = rate_limit_tier.clone();
        }
    }

    Ok(accounts)
}

#[tauri::command]
pub fn add_current_account(label: Option<String>) -> Result<Account, String> {
    // Read the currently logged-in account from Claude config
    let oauth = claude_config::get_active_oauth_account().map_err(|e| e.to_string())?;

    let mut accounts = load_accounts();

    // Check if already exists
    if accounts
        .iter()
        .any(|a| a.account_uuid == oauth.account_uuid)
    {
        return Err("Account already exists".to_string());
    }

    // Save the full oauthAccount config blob (all fields)
    let oauth_config = claude_config::read_full_oauth_account().ok();

    // Read credentials ONCE from active keychain (may prompt on macOS),
    // then re-write with -A ACL so future reads are prompt-free.
    let id = uuid::Uuid::new_v4().to_string();
    let active_creds = keychain::read_active_credentials().ok();

    if let Some(ref creds) = active_creds {
        keychain::backup_credentials(&id, creds).map_err(|e| e.to_string())?;
        // Re-write to active keychain with -A ACL to prevent future prompts
        let _ = keychain::write_active_credentials(creds);
    }

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
        is_active: accounts.is_empty(), // First account is active
        status: AccountStatus::Ok,
        usage: UsageInfo::default(),
        projects: Vec::new(),
        project_ids: Vec::new(),
        selected_project_id: None,
        selected_project: None,
        oauth_config,
    };

    accounts.push(account.clone());
    save_accounts(&accounts)?;

    Ok(account)
}

/// Legacy command — accepts a folder path, upserts into the registry as a manual project,
/// then links it to the account. Kept so the old AddProject UX still works.
#[tauri::command]
pub fn add_project_to_account(account_id: String, path: String) -> Result<Account, String> {
    let name = std::path::Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.clone());

    let mut registry = project_registry::load();
    let project_id = project_registry::upsert_manual(&mut registry, &name, &path);
    project_registry::save(&registry)?;

    let mut accounts = load_accounts();
    let account = accounts
        .iter_mut()
        .find(|a| a.id == account_id)
        .ok_or("Account not found")?;

    if !account.project_ids.contains(&project_id) {
        account.project_ids.push(project_id.clone());
    }
    if account.selected_project_id.is_none() {
        account.selected_project_id = Some(project_id);
    }

    let updated = account.clone();
    save_accounts(&accounts)?;
    Ok(updated)
}

/// Legacy command — kept so the old UI can still call by index.
/// Maps the index into `project_ids` and unlinks it.
#[tauri::command]
pub fn remove_project_from_account(account_id: String, project_index: usize) -> Result<Account, String> {
    let mut accounts = load_accounts();
    let account = accounts
        .iter_mut()
        .find(|a| a.id == account_id)
        .ok_or("Account not found")?;

    if project_index >= account.project_ids.len() {
        return Err("Invalid project index".to_string());
    }

    let removed_id = account.project_ids.remove(project_index);
    if account.selected_project_id.as_deref() == Some(removed_id.as_str()) {
        account.selected_project_id = account.project_ids.first().cloned();
    }

    let updated = account.clone();
    save_accounts(&accounts)?;
    Ok(updated)
}

/// Legacy command — kept for old UI; index refers to `project_ids`.
#[tauri::command]
pub fn set_selected_project(account_id: String, project_index: Option<usize>) -> Result<(), String> {
    let mut accounts = load_accounts();
    let account = accounts
        .iter_mut()
        .find(|a| a.id == account_id)
        .ok_or("Account not found")?;

    let new_selected = match project_index {
        Some(idx) => {
            let pid = account
                .project_ids
                .get(idx)
                .ok_or("Invalid project index")?
                .clone();
            Some(pid)
        }
        None => None,
    };

    account.selected_project_id = new_selected;
    save_accounts(&accounts)?;
    Ok(())
}

#[tauri::command]
pub fn update_account(id: String, update: AccountUpdate) -> Result<Account, String> {
    let mut accounts = load_accounts();
    let account = accounts
        .iter_mut()
        .find(|a| a.id == id)
        .ok_or("Account not found")?;

    if let Some(label) = update.label {
        account.label = Some(label);
    }
    if let Some(plan) = update.plan {
        account.plan = plan;
    }
    if let Some(status) = update.status {
        account.status = status;
    }
    if let Some(usage) = update.usage {
        account.usage = usage;
    }

    let updated = account.clone();
    save_accounts(&accounts)?;
    Ok(updated)
}

#[tauri::command]
pub fn remove_account(id: String) -> Result<(), String> {
    let mut accounts = load_accounts();
    let was_active = accounts
        .iter()
        .find(|a| a.id == id)
        .map(|a| a.is_active)
        .unwrap_or(false);

    // Delete keychain backup
    let _ = keychain::delete_credentials(&id);

    accounts.retain(|a| a.id != id);

    // If removed account was active and there are others, make first one active
    if was_active && !accounts.is_empty() {
        accounts[0].is_active = true;
    }

    save_accounts(&accounts)?;
    Ok(())
}

#[tauri::command]
pub fn get_active_account() -> Result<Option<Account>, String> {
    let accounts = load_accounts();
    Ok(accounts.into_iter().find(|a| a.is_active))
}

#[tauri::command]
pub fn detect_current_account() -> Result<OAuthAccount, String> {
    claude_config::get_active_oauth_account().map_err(|e| e.to_string())
}
