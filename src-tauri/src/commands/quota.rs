use crate::models::UsageInfo;
use crate::services::{credential_store, token_refresh, usage_tracker};

#[tauri::command]
pub fn get_usage_info() -> Result<UsageInfo, String> {
    Ok(usage_tracker::get_usage_info())
}

/// Check token health for a specific account by calling the Claude API roles endpoint.
/// Returns org info if token is valid, or error status if expired/invalid.
///
/// Tries `refresh_token` automatically before declaring the account expired:
/// 1. If `expiresAt` says token is already expired/expiring → refresh proactively first.
/// 2. If API call returns "expired" → fall back to refresh + retry once.
/// Only marks the account as Expired when the refresh_token itself is no longer valid.
#[tauri::command]
pub fn check_account_token(account_id: String) -> Result<TokenHealthResult, String> {
    let accounts = super::account::load_accounts();
    let account = accounts
        .iter()
        .find(|a| a.id == account_id)
        .cloned()
        .ok_or("Account not found")?;

    // Get credentials from file store
    let mut creds = match credential_store::load(&account.id) {
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

    // Proactive refresh: if expiresAt says we're already expired/about-to-expire,
    // try refresh_token before hitting the API.
    let mut refreshed = false;
    let token_info = token_refresh::extract_token_info(&creds);
    let proactively_expired = token_info
        .as_ref()
        .map(|ti| token_refresh::is_token_expired(ti))
        .unwrap_or(false);
    if proactively_expired {
        log::info!("Manual check: token expired by expiresAt for {}, refreshing first", account_id);
        if let Ok(_) = token_refresh::refresh_account_credentials(&account_id, account.is_active) {
            if let Ok(new_creds) = credential_store::load(&account.id) {
                creds = new_creds;
                refreshed = true;
            }
        }
    }

    // Verify with API (may attempt one more refresh fallback if API still says expired)
    let mut attempt = 0u8;
    loop {
        attempt += 1;

        let token = serde_json::from_str::<serde_json::Value>(&creds)
            .ok()
            .and_then(|v| {
                v.get("claudeAiOauth")
                    .and_then(|o| o.get("accessToken"))
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string())
            });
        let token = match token {
            Some(t) if !t.is_empty() => t,
            _ => {
                return Ok(TokenHealthResult {
                    valid: false,
                    status: "invalid_credentials".to_string(),
                    organization_name: None,
                    organization_role: None,
                    error_message: Some("Invalid credentials".to_string()),
                });
            }
        };

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

        let json = match serde_json::from_str::<serde_json::Value>(&body) {
            Ok(j) => j,
            Err(_) => {
                return Ok(TokenHealthResult {
                    valid: false,
                    status: "network_error".to_string(),
                    organization_name: None,
                    organization_role: None,
                    error_message: Some("Failed to connect to API".to_string()),
                });
            }
        };

        if let Some(err) = json.get("error") {
            let err_type = err.get("type").and_then(|t| t.as_str()).unwrap_or("unknown");
            let err_msg = err.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error");
            let needs_refresh = err_type == "authentication_error"
                || err_msg.to_ascii_lowercase().contains("expired");

            // Transient infrastructure errors — don't touch account status.
            if err_type == "rate_limit_error"
                || err_type == "overloaded_error"
                || err_type == "api_error"
            {
                return Ok(TokenHealthResult {
                    valid: false,
                    status: "transient_error".to_string(),
                    organization_name: None,
                    organization_role: None,
                    error_message: Some(err_msg.to_string()),
                });
            }

            // Reactive refresh: API says auth-failed and we haven't tried refresh yet
            if needs_refresh && !refreshed && attempt == 1 {
                log::info!("Manual check: API reports auth error for {}, attempting refresh", account_id);
                if let Ok(_) = token_refresh::refresh_account_credentials(&account_id, account.is_active) {
                    if let Ok(new_creds) = credential_store::load(&account.id) {
                        creds = new_creds;
                        refreshed = true;
                        continue;
                    }
                }
                // Refresh failed — was the failure permanent?
                if !token_refresh::is_in_cooldown(&account_id) {
                    // Transient (network glitch, 5xx). Don't change status.
                    return Ok(TokenHealthResult {
                        valid: false,
                        status: "transient_error".to_string(),
                        organization_name: None,
                        organization_role: None,
                        error_message: Some(format!("Refresh transiently failed: {}", err_msg)),
                    });
                }
            }

            let status = if needs_refresh { "expired" } else { "error" };

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

        // Success
        let org_name = json
            .get("organization_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let org_role = json
            .get("organization_role")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut accounts = super::account::load_accounts();
        if let Some(acc) = accounts.iter_mut().find(|a| a.id == account_id) {
            acc.status = crate::models::AccountStatus::Ok;
            let _ = super::account::save_accounts(&accounts);
        }

        // Clear any cooldown if we just successfully refreshed
        if refreshed {
            token_refresh::clear_cooldown_for(&account_id);
        }

        return Ok(TokenHealthResult {
            valid: true,
            status: if refreshed { "refreshed".to_string() } else { "ok".to_string() },
            organization_name: org_name,
            organization_role: org_role,
            error_message: None,
        });
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

/// Check all tokens + auto-refresh expired ones.
/// Reads only from CMAS backup entries (never from the global active keychain)
/// to avoid macOS keychain password prompts.
#[tauri::command]
pub fn sync_and_check_all_tokens() -> Result<Vec<TokenSyncResult>, String> {
    let accounts = super::account::load_accounts();

    // Check each account and auto-refresh if needed
    let mut results = Vec::new();

    for account in &accounts {
        let result = check_and_refresh_single_account(account);
        results.push(result);
    }

    // 3. Persist updated statuses. `transient_error` means we couldn't
    //    determine the truth (network/5xx) — leave the existing status alone.
    let mut accounts = super::account::load_accounts();
    for result in &results {
        if let Some(acc) = accounts.iter_mut().find(|a| a.id == result.account_id) {
            match result.status.as_str() {
                "ok" => acc.status = crate::models::AccountStatus::Ok,
                "expired" => acc.status = crate::models::AccountStatus::Expired,
                "transient_error" => { /* keep existing status */ }
                _ => acc.status = crate::models::AccountStatus::Error,
            }
        }
    }
    let _ = super::account::save_accounts(&accounts);

    Ok(results)
}

fn check_and_refresh_single_account(account: &crate::models::Account) -> TokenSyncResult {
    let account_id = &account.id;

    // Try to read credentials from file store
    let creds = match credential_store::load(account_id) {
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
                // Transient failure (network/5xx) — bail out without touching
                // status. The account isn't broken, the network is.
                if !token_refresh::is_in_cooldown(account_id) {
                    return TokenSyncResult {
                        account_id: account_id.clone(),
                        status: "transient_error".to_string(),
                        refreshed: false,
                        message: format!("Transient refresh error: {}", e),
                    };
                }
                // Permanent — fall through to API verify so we end up
                // returning status="expired" with confidence.
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
                        if !token_refresh::is_in_cooldown(account_id) {
                            return TokenSyncResult {
                                account_id: account_id.clone(),
                                status: "transient_error".to_string(),
                                refreshed: false,
                                message: format!("Transient refresh error: {}", e),
                            };
                        }
                    }
                }
            }
            TokenSyncResult {
                account_id: account_id.clone(),
                status: "expired".to_string(),
                refreshed: false,
                message: "Token expired, refresh_token is no longer valid".to_string(),
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
            // Network blip — never change persisted status. Caller treats
            // "transient_error" as "leave the account alone".
            status: "transient_error".to_string(),
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

                // Map by error type rather than fuzzy-matching the message.
                // Rate-limited / overloaded responses do NOT mean the token
                // is bad; treating them as "Expired" used to trigger spurious
                // refresh attempts and ultimately mark healthy accounts as expired.
                match err_type {
                    "authentication_error" => TokenVerifyResult::Expired,
                    "rate_limit_error" => TokenVerifyResult::Valid,
                    "overloaded_error" | "api_error" => TokenVerifyResult::NetworkError,
                    _ => {
                        if err_msg.to_ascii_lowercase().contains("expired") {
                            TokenVerifyResult::Expired
                        } else {
                            TokenVerifyResult::Error(err_msg.to_string())
                        }
                    }
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
        Err(e) => {
            // Permanent failure → mark Expired so user re-auths.
            // Transient → leave status alone, frontend shows retry hint.
            let permanent = token_refresh::is_in_cooldown(&account_id);
            if permanent {
                let mut accounts = super::account::load_accounts();
                if let Some(acc) = accounts.iter_mut().find(|a| a.id == account_id) {
                    acc.status = crate::models::AccountStatus::Expired;
                    let _ = super::account::save_accounts(&accounts);
                }
            }
            Ok(TokenSyncResult {
                account_id,
                status: if permanent { "expired" } else { "transient_error" }.to_string(),
                refreshed: false,
                message: format!("Refresh failed: {}", e),
            })
        }
    }
}


#[derive(serde::Serialize, Clone)]
pub struct TokenSyncResult {
    pub account_id: String,
    pub status: String,
    pub refreshed: bool,
    pub message: String,
}
