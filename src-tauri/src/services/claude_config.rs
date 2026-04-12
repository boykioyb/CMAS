use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::models::OAuthAccount;

pub fn get_config_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_default();
    let primary = home.join(".claude").join(".claude.json");
    if primary.exists() {
        primary
    } else {
        home.join(".claude.json")
    }
}

pub fn read_claude_config() -> Result<serde_json::Value> {
    let path = get_config_path();
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read Claude config at {:?}", path))?;
    serde_json::from_str(&content).context("Failed to parse Claude config JSON")
}

pub fn get_active_oauth_account() -> Result<OAuthAccount> {
    let config = read_claude_config()?;
    let oauth = config
        .get("oauthAccount")
        .context("No oauthAccount found in Claude config")?;
    serde_json::from_value(oauth.clone()).context("Failed to parse oauthAccount")
}

pub fn write_oauth_account(oauth: &OAuthAccount) -> Result<()> {
    let oauth_value = serde_json::to_value(oauth).context("Failed to serialize oauthAccount")?;
    write_full_oauth_account(&oauth_value)
}

/// Write a full oauthAccount JSON blob to the config (preserving all fields)
pub fn write_full_oauth_account(oauth_value: &serde_json::Value) -> Result<()> {
    let path = get_config_path();
    let mut config = read_claude_config()?;

    if let Some(obj) = config.as_object_mut() {
        obj.insert("oauthAccount".to_string(), oauth_value.clone());
    }

    // Atomic write: write to temp file, then rename
    let temp_path = path.with_extension("json.tmp");
    let json_string = serde_json::to_string_pretty(&config)?;
    fs::write(&temp_path, &json_string).context("Failed to write temp config")?;
    fs::rename(&temp_path, &path).context("Failed to rename temp config")?;

    Ok(())
}

/// Read the full oauthAccount JSON blob from config
pub fn read_full_oauth_account() -> Result<serde_json::Value> {
    let config = read_claude_config()?;
    config
        .get("oauthAccount")
        .cloned()
        .context("No oauthAccount found in Claude config")
}
