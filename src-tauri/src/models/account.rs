use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub email: String,
    pub label: Option<String>,
    pub account_uuid: String,
    pub plan: AccountPlan,
    pub added_at: String,
    pub last_used_at: Option<String>,
    pub last_switched_at: Option<String>,
    pub is_active: bool,
    pub status: AccountStatus,
    pub usage: UsageInfo,
    /// List of project folders assigned to this account
    #[serde(default)]
    pub projects: Vec<ProjectFolder>,
    /// Currently selected project index (for quick open)
    #[serde(default)]
    pub selected_project: Option<usize>,
    /// Full oauthAccount config blob (saved from ~/.claude.json when adding)
    #[serde(default)]
    pub oauth_config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFolder {
    pub path: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AccountPlan {
    Pro,
    Free,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AccountStatus {
    Ok,
    Error,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelUsage {
    pub messages_5h: u64,
    pub messages_today: u64,
    pub messages_week: u64,
    pub output_tokens_5h: u64,
    pub output_tokens_today: u64,
    pub output_tokens_week: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageInfo {
    /// Messages sent today
    pub messages_today: u64,
    /// Messages in current 5h window
    pub messages_5h_window: u64,
    /// Total sessions today
    pub sessions_today: u64,
    /// Tool calls today
    pub tool_calls_today: u64,
    /// Messages this week
    pub messages_week: u64,
    /// Output tokens in current 5h window
    pub output_tokens_5h: u64,
    /// Output tokens today
    pub output_tokens_today: u64,
    /// Output tokens this week
    pub output_tokens_week: u64,
    /// Last rate limit hit (ISO timestamp or null)
    pub last_rate_limit_at: Option<String>,
    /// Whether currently rate limited (based on last hit + 5h window)
    pub is_rate_limited: bool,
    /// Estimated reset time (ISO timestamp or null)
    pub estimated_reset_at: Option<String>,
    /// Subscription type (team, pro, free)
    pub subscription_type: Option<String>,
    /// Rate limit tier from credential (e.g., "default_claude_max_5x")
    pub rate_limit_tier: Option<String>,
    /// When the 5h session window resets (oldest message in window drops off)
    #[serde(default)]
    pub session_reset_at: Option<String>,
    /// When the weekly usage counters reset (next Monday 00:00 UTC)
    #[serde(default)]
    pub weekly_reset_at: Option<String>,
    /// Last checked timestamp
    pub last_checked_at: String,
    /// Per-model usage breakdown
    #[serde(default)]
    pub opus_usage: ModelUsage,
    #[serde(default)]
    pub sonnet_usage: ModelUsage,
}

impl Default for UsageInfo {
    fn default() -> Self {
        Self {
            messages_today: 0,
            messages_5h_window: 0,
            sessions_today: 0,
            tool_calls_today: 0,
            messages_week: 0,
            output_tokens_5h: 0,
            output_tokens_today: 0,
            output_tokens_week: 0,
            last_rate_limit_at: None,
            is_rate_limited: false,
            estimated_reset_at: None,
            subscription_type: None,
            rate_limit_tier: None,
            session_reset_at: None,
            weekly_reset_at: None,
            last_checked_at: chrono::Utc::now().to_rfc3339(),
            opus_usage: ModelUsage::default(),
            sonnet_usage: ModelUsage::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountUpdate {
    pub label: Option<String>,
    pub plan: Option<AccountPlan>,
    pub status: Option<AccountStatus>,
    pub usage: Option<UsageInfo>,
}

// Represents the oauthAccount section from ~/.claude/.claude.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthAccount {
    #[serde(rename = "emailAddress")]
    pub email_address: String,
    #[serde(rename = "accountUuid")]
    pub account_uuid: String,
    // Keep other fields as dynamic
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
