use crate::models::UsageInfo;
use crate::services::{keychain, token_refresh, usage_tracker};

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

/// Sync active credentials + check all tokens + auto-refresh expired ones.
/// Returns a summary of each account's token status.
#[tauri::command]
pub fn sync_and_check_all_tokens() -> Result<Vec<TokenSyncResult>, String> {
    let accounts = super::account::load_accounts();

    // 1. Sync active account's credentials from keychain to backup
    if let Some(active) = accounts.iter().find(|a| a.is_active) {
        keychain::sync_active_credentials_to_backup(&active.id);
    }

    // 2. Check each account and auto-refresh if needed
    let mut results = Vec::new();

    for account in &accounts {
        let result = check_and_refresh_single_account(account);
        results.push(result);
    }

    // 3. Persist updated statuses
    let mut accounts = super::account::load_accounts();
    for result in &results {
        if let Some(acc) = accounts.iter_mut().find(|a| a.id == result.account_id) {
            acc.status = match result.status.as_str() {
                "ok" => crate::models::AccountStatus::Ok,
                "expired" => crate::models::AccountStatus::Expired,
                _ => crate::models::AccountStatus::Error,
            };
        }
    }
    let _ = super::account::save_accounts(&accounts);

    Ok(results)
}

fn check_and_refresh_single_account(account: &crate::models::Account) -> TokenSyncResult {
    let account_id = &account.id;

    // Try to read backup credentials
    let creds = match keychain::restore_credentials(account_id) {
        Ok(c) => c,
        Err(_) => {
            return TokenSyncResult {
                account_id: account_id.clone(),
                status: "no_credentials".to_string(),
                refreshed: false,
                message: "No backup credentials found".to_string(),
            };
        }
    };

    // Check if token is expired via expiresAt field
    let token_info = token_refresh::extract_token_info(&creds);
    let is_expired = token_info
        .as_ref()
        .map(|ti| token_refresh::is_token_expired(ti))
        .unwrap_or(false);

    if is_expired {
        log::info!("Token expired for account {}, attempting refresh...", account_id);

        // Attempt to refresh
        match token_refresh::refresh_account_credentials(account_id, account.is_active) {
            Ok(_) => {
                log::info!("Token refreshed successfully for {}", account_id);
                return TokenSyncResult {
                    account_id: account_id.clone(),
                    status: "ok".to_string(),
                    refreshed: true,
                    message: "Token refreshed successfully".to_string(),
                };
            }
            Err(e) => {
                log::warn!("Token refresh failed for {}: {}", account_id, e);
                // Refresh failed — verify via API as fallback
            }
        }
    }

    // Verify token via API call (health check)
    let token = token_info
        .as_ref()
        .map(|ti| ti.access_token.clone())
        .or_else(|| extract_access_token_from_creds(&creds));

    let token = match token {
        Some(t) if !t.is_empty() => t,
        _ => {
            return TokenSyncResult {
                account_id: account_id.clone(),
                status: "invalid_credentials".to_string(),
                refreshed: false,
                message: "No access token in credentials".to_string(),
            };
        }
    };

    // Quick API health check
    match verify_token_via_api(&token) {
        TokenVerifyResult::Valid => TokenSyncResult {
            account_id: account_id.clone(),
            status: "ok".to_string(),
            refreshed: false,
            message: "Token is valid".to_string(),
        },
        TokenVerifyResult::Expired => {
            // Token is expired by API — try refresh if we haven't already
            if !is_expired {
                log::info!("API reports expired token for {}, attempting refresh...", account_id);
                match token_refresh::refresh_account_credentials(account_id, account.is_active) {
                    Ok(_) => {
                        return TokenSyncResult {
                            account_id: account_id.clone(),
                            status: "ok".to_string(),
                            refreshed: true,
                            message: "Token refreshed after API expiry detection".to_string(),
                        };
                    }
                    Err(e) => {
                        log::warn!("Token refresh failed for {}: {}", account_id, e);
                    }
                }
            }
            TokenSyncResult {
                account_id: account_id.clone(),
                status: "expired".to_string(),
                refreshed: false,
                message: "Token expired, refresh failed".to_string(),
            }
        }
        TokenVerifyResult::Error(msg) => TokenSyncResult {
            account_id: account_id.clone(),
            status: "error".to_string(),
            refreshed: false,
            message: msg,
        },
        TokenVerifyResult::NetworkError => TokenSyncResult {
            account_id: account_id.clone(),
            // Don't mark as error if it's just a network issue
            status: if account.status == crate::models::AccountStatus::Ok {
                "ok".to_string()
            } else {
                "error".to_string()
            },
            refreshed: false,
            message: "Network error during health check".to_string(),
        },
    }
}

fn extract_access_token_from_creds(creds: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(creds).ok()?;
    v.get("claudeAiOauth")
        .and_then(|o| o.get("accessToken"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
}

enum TokenVerifyResult {
    Valid,
    Expired,
    Error(String),
    NetworkError,
}

fn verify_token_via_api(token: &str) -> TokenVerifyResult {
    let output = match std::process::Command::new("curl")
        .args([
            "-s",
            "-m", "10",
            "-H",
            &format!("Authorization: Bearer {}", token),
            "https://api.anthropic.com/api/oauth/claude_cli/roles",
        ])
        .output()
    {
        Ok(o) => o,
        Err(_) => return TokenVerifyResult::NetworkError,
    };

    let body = String::from_utf8_lossy(&output.stdout).to_string();

    match serde_json::from_str::<serde_json::Value>(&body) {
        Ok(json) => {
            if let Some(err) = json.get("error") {
                let err_msg = err
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error");
                let err_type = err
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("unknown");

                if err_msg.contains("expired") || err_type == "authentication_error" {
                    TokenVerifyResult::Expired
                } else {
                    TokenVerifyResult::Error(err_msg.to_string())
                }
            } else {
                TokenVerifyResult::Valid
            }
        }
        Err(_) => {
            if body.is_empty() {
                TokenVerifyResult::NetworkError
            } else {
                TokenVerifyResult::Error("Invalid API response".to_string())
            }
        }
    }
}

/// Manually refresh a specific account's token.
#[tauri::command]
pub fn refresh_account_token(account_id: String) -> Result<TokenSyncResult, String> {
    let accounts = super::account::load_accounts();
    let account = accounts
        .iter()
        .find(|a| a.id == account_id)
        .ok_or("Account not found")?;

    match token_refresh::refresh_account_credentials(&account_id, account.is_active) {
        Ok(_) => {
            // Update status to Ok
            let mut accounts = super::account::load_accounts();
            if let Some(acc) = accounts.iter_mut().find(|a| a.id == account_id) {
                acc.status = crate::models::AccountStatus::Ok;
                let _ = super::account::save_accounts(&accounts);
            }

            Ok(TokenSyncResult {
                account_id,
                status: "ok".to_string(),
                refreshed: true,
                message: "Token refreshed successfully".to_string(),
            })
        }
        Err(e) => Ok(TokenSyncResult {
            account_id,
            status: "error".to_string(),
            refreshed: false,
            message: format!("Refresh failed: {}", e),
        }),
    }
}

/// Sync active credentials to backup only (lightweight, no API calls).
#[tauri::command]
pub fn sync_active_credentials() -> Result<(), String> {
    let accounts = super::account::load_accounts();
    if let Some(active) = accounts.iter().find(|a| a.is_active) {
        keychain::sync_active_credentials_to_backup(&active.id);
    }
    Ok(())
}

#[derive(serde::Serialize, Clone)]
pub struct TokenSyncResult {
    pub account_id: String,
    pub status: String,
    pub refreshed: bool,
    pub message: String,
}
