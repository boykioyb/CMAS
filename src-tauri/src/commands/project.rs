use crate::commands::account;
use crate::models::{Account, Project, ProjectRegistry, ScanRoot, ScanSummary};
use crate::services::{project_registry, project_scanner};

#[tauri::command]
pub fn list_projects() -> Result<Vec<Project>, String> {
    let registry = project_registry::load();
    Ok(registry.projects)
}

#[tauri::command]
pub fn list_scan_roots() -> Result<Vec<ScanRoot>, String> {
    let registry = project_registry::load();
    Ok(registry.roots)
}

#[tauri::command]
pub fn get_project_registry() -> Result<ProjectRegistry, String> {
    Ok(project_registry::load())
}

#[tauri::command]
pub fn add_scan_root(path: String) -> Result<ScanRoot, String> {
    let mut registry = project_registry::load();
    let root = project_registry::add_root(&mut registry, &path);
    project_registry::save(&registry)?;
    Ok(root)
}

#[tauri::command]
pub fn remove_scan_root(root_id: String) -> Result<(), String> {
    let mut registry = project_registry::load();
    project_registry::remove_root(&mut registry, &root_id);
    project_registry::save(&registry)
}

#[tauri::command]
pub fn update_scan_root(root_id: String, max_depth: Option<u8>, enabled: Option<bool>) -> Result<ScanRoot, String> {
    let mut registry = project_registry::load();
    let root = registry
        .roots
        .iter_mut()
        .find(|r| r.id == root_id)
        .ok_or("Scan root not found")?;
    if let Some(d) = max_depth {
        root.max_depth = d.min(8);
    }
    if let Some(e) = enabled {
        root.enabled = e;
    }
    let updated = root.clone();
    project_registry::save(&registry)?;
    Ok(updated)
}

#[tauri::command]
pub fn scan_projects() -> Result<Vec<ScanSummary>, String> {
    let mut registry = project_registry::load();
    let summaries = project_scanner::scan_all(&mut registry);
    project_registry::save(&registry)?;
    Ok(summaries)
}

#[tauri::command]
pub fn scan_single_root(root_id: String) -> Result<ScanSummary, String> {
    let mut registry = project_registry::load();
    let root = registry
        .roots
        .iter()
        .find(|r| r.id == root_id)
        .cloned()
        .ok_or("Scan root not found")?;
    let summary = project_scanner::scan_root(&mut registry, &root);
    project_registry::save(&registry)?;
    Ok(summary)
}

#[tauri::command]
pub fn add_manual_project(name: String, path: String) -> Result<Project, String> {
    let mut registry = project_registry::load();
    let id = project_registry::upsert_manual(&mut registry, &name, &path);
    project_registry::save(&registry)?;
    let project = registry
        .projects
        .into_iter()
        .find(|p| p.id == id)
        .ok_or("Project not found after insert")?;
    Ok(project)
}

#[tauri::command]
pub fn remove_project(project_id: String) -> Result<(), String> {
    let mut registry = project_registry::load();
    registry.projects.retain(|p| p.id != project_id);
    project_registry::save(&registry)?;

    // Unlink from all accounts as well
    let mut accounts = account::load_accounts();
    let mut changed = false;
    for acc in accounts.iter_mut() {
        let before = acc.project_ids.len();
        acc.project_ids.retain(|id| id != &project_id);
        if acc.project_ids.len() != before {
            changed = true;
        }
        if acc.selected_project_id.as_deref() == Some(project_id.as_str()) {
            acc.selected_project_id = acc.project_ids.first().cloned();
            changed = true;
        }
    }
    if changed {
        account::save_accounts(&accounts)?;
    }
    Ok(())
}

#[tauri::command]
pub fn toggle_project_favorite(project_id: String) -> Result<Project, String> {
    let mut registry = project_registry::load();
    let project = registry
        .projects
        .iter_mut()
        .find(|p| p.id == project_id)
        .ok_or("Project not found")?;
    project.favorite = !project.favorite;
    let updated = project.clone();
    project_registry::save(&registry)?;
    Ok(updated)
}

#[tauri::command]
pub fn link_project_to_account(account_id: String, project_id: String) -> Result<Account, String> {
    let registry = project_registry::load();
    if !registry.projects.iter().any(|p| p.id == project_id) {
        return Err("Project not in registry".to_string());
    }

    let mut accounts = account::load_accounts();
    let acc = accounts
        .iter_mut()
        .find(|a| a.id == account_id)
        .ok_or("Account not found")?;

    if !acc.project_ids.contains(&project_id) {
        acc.project_ids.push(project_id.clone());
    }
    if acc.selected_project_id.is_none() {
        acc.selected_project_id = Some(project_id);
    }

    let updated = acc.clone();
    account::save_accounts(&accounts)?;
    Ok(updated)
}

#[tauri::command]
pub fn unlink_project_from_account(account_id: String, project_id: String) -> Result<Account, String> {
    let mut accounts = account::load_accounts();
    let acc = accounts
        .iter_mut()
        .find(|a| a.id == account_id)
        .ok_or("Account not found")?;

    acc.project_ids.retain(|id| id != &project_id);
    if acc.selected_project_id.as_deref() == Some(project_id.as_str()) {
        acc.selected_project_id = acc.project_ids.first().cloned();
    }

    let updated = acc.clone();
    account::save_accounts(&accounts)?;
    Ok(updated)
}

#[tauri::command]
pub fn set_selected_project_id(account_id: String, project_id: Option<String>) -> Result<(), String> {
    let mut accounts = account::load_accounts();
    let acc = accounts
        .iter_mut()
        .find(|a| a.id == account_id)
        .ok_or("Account not found")?;

    if let Some(ref pid) = project_id {
        if !acc.project_ids.contains(pid) {
            return Err("Project not linked to account".to_string());
        }
    }

    acc.selected_project_id = project_id;
    account::save_accounts(&accounts)?;
    Ok(())
}
