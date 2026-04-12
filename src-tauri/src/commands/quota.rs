use crate::models::UsageInfo;
use crate::services::{keychain, usage_tracker};

#[tauri::command]
pub fn get_usage_info() -> Result<UsageInfo, String> {
    Ok(usage_tracker::get_usage_info())
}

/// Check token health for a specific account by calling the Claude API roles endpoint.
/// Returns org info if token is valid, or error status if expired/invalid.
#[tauri::command]
pub fn check_account_token(account_id: String) -> Result<TokenHealthResult, String> {
    let accounts = super::account::load_accounts();
    let account = accounts
        .iter()
        .find(|a| a.id == account_id)
        .ok_or("Account not found")?;

    // Get credentials from backup
    let creds = match keychain::restore_credentials(&account.id) {
        Ok(c) => c,
        Err(_) => {
            return Ok(TokenHealthResult {
                valid: false,
                status: "no_credentials".to_string(),
                organization_name: None,
                organization_role: None,
                error_message: Some("Credentials not found".to_string()),
            });
        }
    };

    // Extract OAuth access token
    let token = match serde_json::from_str::<serde_json::Value>(&creds) {
        Ok(v) => v
            .get("claudeAiOauth")
            .and_then(|o| o.get("accessToken"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string()),
        Err(_) => None,
    };

    let token = match token {
        Some(t) => t,
        None => {
            return Ok(TokenHealthResult {
                valid: false,
                status: "invalid_credentials".to_string(),
                organization_name: None,
                organization_role: None,
                error_message: Some("Invalid credentials".to_string()),
            });
        }
    };

    // Call /api/oauth/claude_cli/roles to check token health
    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "-m", "10",
            "-H", &format!("Authorization: Bearer {}", token),
            "https://api.anthropic.com/api/oauth/claude_cli/roles",
        ])
        .output()
        .map_err(|e| format!("Failed to check token: {}", e))?;

    let body = String::from_utf8_lossy(&output.stdout).to_string();

    match serde_json::from_str::<serde_json::Value>(&body) {
        Ok(json) => {
            // Check for error response
            if let Some(err) = json.get("error") {
                let err_type = err
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("unknown");
                let err_msg = err
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error");

                let status = if err_msg.contains("expired") {
                    "expired"
                } else if err_type == "authentication_error" {
                    "auth_error"
                } else {
                    "error"
                };

                // Update account status in storage
                let mut accounts = super::account::load_accounts();
                if let Some(acc) = accounts.iter_mut().find(|a| a.id == account_id) {
                    acc.status = if status == "expired" {
                        crate::models::AccountStatus::Expired
                    } else {
                        crate::models::AccountStatus::Error
                    };
                    let _ = super::account::save_accounts(&accounts);
                }

                return Ok(TokenHealthResult {
                    valid: false,
                    status: status.to_string(),
                    organization_name: None,
                    organization_role: None,
                    error_message: Some(err_msg.to_string()),
                });
            }

            // Success — extract org info
            let org_name = json
                .get("organization_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let org_role = json
                .get("organization_role")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Update account status to Ok
            let mut accounts = super::account::load_accounts();
            if let Some(acc) = accounts.iter_mut().find(|a| a.id == account_id) {
                acc.status = crate::models::AccountStatus::Ok;
                let _ = super::account::save_accounts(&accounts);
            }

            Ok(TokenHealthResult {
                valid: true,
                status: "ok".to_string(),
                organization_name: org_name,
                organization_role: org_role,
                error_message: None,
            })
        }
        Err(_) => Ok(TokenHealthResult {
            valid: false,
            status: "network_error".to_string(),
            organization_name: None,
            organization_role: None,
            error_message: Some("Failed to connect to API".to_string()),
        }),
    }
}

#[derive(serde::Serialize)]
pub struct TokenHealthResult {
    pub valid: bool,
    pub status: String,
    pub organization_name: Option<String>,
    pub organization_role: Option<String>,
    pub error_message: Option<String>,
}

#[tauri::command]
pub fn get_daily_activity(days: Option<u32>) -> Result<Vec<usage_tracker::DailyActivity>, String> {
    let all = usage_tracker::read_stats_cache().map_err(|e| e.to_string())?;
    let days = days.unwrap_or(7) as usize;
    let start = if all.len() > days { all.len() - days } else { 0 };
    Ok(all[start..].to_vec())
}

#[tauri::command]
pub fn get_quota_summary() -> Result<QuotaSummary, String> {
    let usage = usage_tracker::get_usage_info();

    Ok(QuotaSummary {
        messages_today: usage.messages_today,
        messages_week: usage.messages_week,
        sessions_today: usage.sessions_today,
        is_rate_limited: usage.is_rate_limited,
        estimated_reset_at: usage.estimated_reset_at,
        subscription_type: usage.subscription_type,
    })
}

#[derive(serde::Serialize)]
pub struct QuotaSummary {
    pub messages_today: u64,
    pub messages_week: u64,
    pub sessions_today: u64,
    pub is_rate_limited: bool,
    pub estimated_reset_at: Option<String>,
    pub subscription_type: Option<String>,
}
