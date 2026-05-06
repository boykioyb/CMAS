use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

const OAUTH_TOKEN_ENDPOINT: &str = "https://platform.claude.com/v1/oauth/token";
const OAUTH_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";

/// Cooldown duration after a failed refresh (5 minutes).
const REFRESH_COOLDOWN_SECS: u64 = 300;

lazy_static::lazy_static! {
    /// Tracks last failed refresh time per account to prevent hammering the API.
    static ref REFRESH_COOLDOWNS: Mutex<HashMap<String, Instant>> = Mutex::new(HashMap::new());

    /// Per-account locks to serialize refresh calls. Without this, two concurrent
    /// triggers (hourly polling + manual click + reactive 401 fallback) can both
    /// read the same RT, race against the OAuth server's exactly-once rotation,
    /// and one side gets `invalid_grant` → cooldown → user appears logged out.
    static ref REFRESH_LOCKS: Mutex<HashMap<String, Arc<Mutex<()>>>> = Mutex::new(HashMap::new());
}

/// Acquire (or create) the refresh lock for a given account.
fn get_refresh_lock(account_id: &str) -> Arc<Mutex<()>> {
    let mut locks = REFRESH_LOCKS.lock().expect("refresh locks poisoned");
    locks
        .entry(account_id.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

/// Check if an account is in cooldown (recently failed refresh).
pub fn is_in_cooldown(account_id: &str) -> bool {
    if let Ok(cooldowns) = REFRESH_COOLDOWNS.lock() {
        if let Some(failed_at) = cooldowns.get(account_id) {
            return failed_at.elapsed().as_secs() < REFRESH_COOLDOWN_SECS;
        }
    }
    false
}

/// Record a failed refresh for cooldown tracking.
fn set_cooldown(account_id: &str) {
    if let Ok(mut cooldowns) = REFRESH_COOLDOWNS.lock() {
        cooldowns.insert(account_id.to_string(), Instant::now());
    }
}

/// Clear cooldown after a successful refresh.
fn clear_cooldown(account_id: &str) {
    if let Ok(mut cooldowns) = REFRESH_COOLDOWNS.lock() {
        cooldowns.remove(account_id);
    }
}

/// Public: clear cooldown for re-auth scenarios.
pub fn clear_cooldown_for(account_id: &str) {
    clear_cooldown(account_id);
}

/// True if a refresh error indicates the refresh_token itself is dead.
/// Only permanent errors should set a cooldown — transient errors (network,
/// 5xx, parse failures) must remain retryable, otherwise a single hiccup
/// blocks all refreshes for 5 minutes and the user appears to be logged out.
fn is_permanent_refresh_error(err_msg: &str) -> bool {
    let m = err_msg.to_ascii_lowercase();
    m.contains("invalid_grant")
        || m.contains("invalid_token")
        || m.contains("invalid_request")
        || m.contains("unauthorized_client")
        || m.contains("unsupported_grant_type")
        || m.contains("no refresh token")
}

/// Token info extracted from stored credentials.
#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: Option<String>,
}

/// Response from the OAuth token refresh endpoint.
#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    refresh_token: String,
    #[serde(default)]
    expires_in: Option<u64>,
    #[serde(default)]
    #[allow(dead_code)]
    token_type: Option<String>,
}

/// Result of a token refresh attempt.
#[derive(Debug, Clone, Serialize)]
pub struct RefreshResult {
    pub success: bool,
    pub message: String,
}

/// Extract token info from stored credential JSON.
pub fn extract_token_info(creds: &str) -> Option<TokenInfo> {
    let v: serde_json::Value = serde_json::from_str(creds).ok()?;
    let oauth = v.get("claudeAiOauth")?;

    let access_token = oauth.get("accessToken")?.as_str()?.to_string();
    let refresh_token = oauth.get("refreshToken")?.as_str()?.to_string();
    let expires_at = oauth.get("expiresAt").and_then(|v| {
        if let Some(s) = v.as_str() {
            Some(s.to_string())
        } else if let Some(n) = v.as_i64() {
            Some(n.to_string())
        } else if let Some(n) = v.as_f64() {
            Some((n as i64).to_string())
        } else {
            None
        }
    });

    Some(TokenInfo {
        access_token,
        refresh_token,
        expires_at,
    })
}

/// Check if a token is expired or about to expire (within 5 minutes).
///
/// Fail-safe direction: when we cannot determine the expiry, return `true`
/// so the caller will refresh. Pretending an unknown token is valid (the old
/// behavior) caused refreshes to be skipped indefinitely on parse failures.
pub fn is_token_expired(token_info: &TokenInfo) -> bool {
    let expires_at = match &token_info.expires_at {
        Some(s) => s,
        None => return true,
    };

    let expires = match chrono::DateTime::parse_from_rfc3339(expires_at) {
        Ok(dt) => dt,
        Err(_) => {
            // Numeric timestamp — auto-detect seconds vs milliseconds.
            // Anything below 10^11 (year ~5138 in seconds) is too small to be
            // a millisecond epoch for any plausible token, so treat as seconds.
            let parsed = expires_at.parse::<i64>();
            match parsed {
                Ok(n) => {
                    let dt = if n.abs() < 100_000_000_000 {
                        chrono::DateTime::from_timestamp(n, 0)
                    } else {
                        chrono::DateTime::from_timestamp_millis(n)
                    };
                    match dt {
                        Some(d) => d.fixed_offset(),
                        None => return true,
                    }
                }
                Err(_) => return true,
            }
        }
    };

    let now = chrono::Utc::now();
    let buffer = chrono::Duration::minutes(5);

    // Expired or will expire within 5 minutes
    expires.signed_duration_since(now) < buffer
}

/// Refresh an OAuth token using the refresh_token grant.
/// Returns the new credential JSON string with updated tokens.
/// Fails immediately on rate limit (429) — no retries.
pub fn refresh_oauth_token(current_creds: &str) -> Result<String> {
    let token_info = extract_token_info(current_creds)
        .context("Failed to extract token info from credentials")?;

    if token_info.refresh_token.is_empty() {
        return Err(anyhow::anyhow!("No refresh token available"));
    }

    let body = serde_json::json!({
        "grant_type": "refresh_token",
        "refresh_token": token_info.refresh_token,
        "client_id": OAUTH_CLIENT_ID,
    });
    let body_str = body.to_string();

    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "-m", "15",
            "-X", "POST",
            "-H", "Content-Type: application/json",
            "-w", "\n%{http_code}",
            "-d", &body_str,
            OAUTH_TOKEN_ENDPOINT,
        ])
        .output()
        .context("Failed to call OAuth token endpoint")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("curl failed: {}", stderr));
    }

    let raw_output = String::from_utf8_lossy(&output.stdout).to_string();

    // Split response body from HTTP status code (last line)
    let (response_body, http_status) = match raw_output.rfind('\n') {
        Some(pos) => {
            let body = raw_output[..pos].to_string();
            let code = raw_output[pos + 1..].trim().parse::<u16>().unwrap_or(0);
            (body, code)
        }
        None => (raw_output, 0u16),
    };

    log::debug!("OAuth refresh: HTTP {}", http_status);

    // Rate limited — fail immediately, no retries
    if http_status == 429 {
        log::warn!("Token refresh rate limited (HTTP 429)");
        return Err(anyhow::anyhow!("Rate limited (HTTP 429)"));
    }

    // Server error — fail immediately
    if http_status >= 500 {
        log::warn!("Token refresh server error (HTTP {})", http_status);
        return Err(anyhow::anyhow!("Server error (HTTP {})", http_status));
    }

    // Check for error in response body
    if let Ok(err_json) = serde_json::from_str::<serde_json::Value>(&response_body) {
        if let Some(err) = err_json.get("error") {
            let err_msg = err
                .as_str()
                .or_else(|| err.get("message").and_then(|m| m.as_str()))
                .unwrap_or("Unknown error");

            return Err(anyhow::anyhow!("OAuth refresh failed: {}", err_msg));
        }
    }

    // Success — parse token response
    let token_response: OAuthTokenResponse = serde_json::from_str(&response_body)
        .context("Failed to parse OAuth token response")?;

    // Update the credential JSON with new tokens
    let mut creds_json: serde_json::Value = serde_json::from_str(current_creds)
        .context("Failed to parse current credentials")?;

    if let Some(oauth) = creds_json.get_mut("claudeAiOauth") {
        oauth["accessToken"] = serde_json::Value::String(token_response.access_token);
        oauth["refreshToken"] = serde_json::Value::String(token_response.refresh_token);

        if let Some(expires_in) = token_response.expires_in {
            let expires_at = chrono::Utc::now()
                + chrono::Duration::seconds(expires_in as i64);
            oauth["expiresAt"] = serde_json::Value::String(expires_at.to_rfc3339());
        }
    }

    Ok(serde_json::to_string(&creds_json).context("Failed to serialize updated credentials")?)
}

/// Attempt to refresh credentials for a specific account.
/// Updates both the backup and (if active) the global keychain.
/// Respects cooldown — skips if a recent refresh failed within 5 minutes.
pub fn refresh_account_credentials(account_id: &str, is_active: bool) -> Result<RefreshResult> {
    // Cheap pre-check before we even queue on the lock.
    if is_in_cooldown(account_id) {
        log::info!("Account {} is in refresh cooldown, skipping", account_id);
        return Err(anyhow::anyhow!(
            "Token refresh on cooldown — will retry automatically in a few minutes"
        ));
    }

    // Serialize all refresh attempts for this account. If another thread is
    // already refreshing, we wait. Once we get the lock, the token in storage
    // is the result of whatever the other thread just did — so we re-evaluate
    // and short-circuit when possible.
    let lock = get_refresh_lock(account_id);
    let _guard = lock.lock().expect("per-account refresh lock poisoned");

    // Re-check cooldown — another thread may have just failed permanently.
    if is_in_cooldown(account_id) {
        return Err(anyhow::anyhow!(
            "Token refresh on cooldown after concurrent failure"
        ));
    }

    // Read current credentials. When this is the active account, the Claude
    // CLI may have rotated tokens behind our back — prefer the active keychain
    // (CLI's source of truth) and fall back to our file store.
    let current_creds = if is_active {
        super::keychain::read_active_credentials()
            .or_else(|_| super::credential_store::load(account_id))
            .context("Failed to read credentials (active and file store both failed)")?
    } else {
        super::credential_store::load(account_id)
            .context("Failed to read credentials from file store")?
    };

    // Short-circuit: if the stored token is no longer expiring soon, a
    // concurrent refresh just succeeded and we can reuse its result.
    if let Some(info) = extract_token_info(&current_creds) {
        if !is_token_expired(&info) {
            log::info!(
                "Account {}: token already fresh after concurrent refresh, skipping",
                account_id
            );
            return Ok(RefreshResult {
                success: true,
                message: "Token already refreshed by concurrent caller".to_string(),
            });
        }
    }

    // Attempt refresh
    match refresh_oauth_token(&current_creds) {
        Ok(new_creds) => {
            // Save refreshed credentials to file store
            super::credential_store::store(account_id, &new_creds)
                .context("Failed to save refreshed credentials")?;

            // If this is the active account, also update the global keychain
            if is_active {
                super::keychain::write_active_credentials(&new_creds)
                    .context("Failed to update global keychain with refreshed credentials")?;
            }

            clear_cooldown(account_id);
            log::info!("Successfully refreshed token for account {}", account_id);

            Ok(RefreshResult {
                success: true,
                message: "Token refreshed successfully".to_string(),
            })
        }
        Err(e) => {
            let msg = e.to_string();
            if is_permanent_refresh_error(&msg) {
                set_cooldown(account_id);
                log::warn!(
                    "Token refresh permanently failed for {}, cooldown set: {}",
                    account_id, msg
                );
            } else {
                log::info!(
                    "Token refresh transient error for {} (no cooldown): {}",
                    account_id, msg
                );
            }
            Err(e)
        }
    }
}
