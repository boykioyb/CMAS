use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub language: String,       // "vi" | "en"
    pub theme: String,          // "light" | "dark" | "system"
    pub vscode_path: String,
    pub quota_warning_threshold: u32,
    pub auto_switch_on_empty: bool,
    pub launch_at_login: bool,
    pub claude_config_path: String,
    #[serde(default)]
    pub claude_cli_path: String, // empty = auto-detect
    pub backup_dir: String,
    #[serde(default = "default_usage_refresh_interval")]
    pub usage_refresh_interval: u32, // seconds, 0 = disabled
}

fn default_usage_refresh_interval() -> u32 {
    300 // 5 minutes
}

impl Default for AppConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        Self {
            language: "vi".to_string(),
            theme: "system".to_string(),
            #[cfg(target_os = "windows")]
            vscode_path: "code.cmd".to_string(),
            #[cfg(not(target_os = "windows"))]
            vscode_path: "/usr/local/bin/code".to_string(),
            quota_warning_threshold: 20,
            auto_switch_on_empty: false,
            launch_at_login: false,
            claude_config_path: home
                .join(".claude")
                .join(".claude.json")
                .to_string_lossy()
                .to_string(),
            claude_cli_path: String::new(), // empty = auto-detect
            backup_dir: home.join(".claude-switcher").to_string_lossy().to_string(),
            usage_refresh_interval: default_usage_refresh_interval(),
        }
    }
}
