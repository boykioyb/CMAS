use crate::models::AppConfig;
use std::fs;
use std::path::PathBuf;

fn get_config_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_default();
    home.join(".claude-switcher").join("config.json")
}

#[tauri::command]
pub fn get_app_config() -> Result<AppConfig, String> {
    let path = get_config_path();
    if let Ok(content) = fs::read_to_string(&path) {
        serde_json::from_str(&content).map_err(|e| e.to_string())
    } else {
        Ok(AppConfig::default())
    }
}

#[tauri::command]
pub fn save_app_config(config: AppConfig) -> Result<(), String> {
    let home = dirs::home_dir().unwrap_or_default();
    let dir = home.join(".claude-switcher");
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    let path = dir.join("config.json");
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn find_vscode() -> Result<Option<String>, String> {
    Ok(crate::services::vscode::find_vscode_path())
}

#[tauri::command]
pub fn find_claude_cli() -> Result<Option<String>, String> {
    Ok(crate::services::claude_auth::find_claude_cli())
}
