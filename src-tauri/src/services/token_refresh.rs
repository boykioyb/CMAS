use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const OAUTH_TOKEN_ENDPOINT: &str = "https://console.anthropic.com/v1/oauth/token";
const OAUTH_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";

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
    let expires_at = oauth
        .get("expiresAt")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Some(TokenInfo {
        access_token,
        refresh_token,
        expires_at,
    })
}

/// Check if a token is expired or about to expire (within 5 minutes).
pub fn is_token_expired(token_info: &TokenInfo) -> bool {
    let expires_at = match &token_info.expires_at {
        Some(s) => s,
        None => return false, // No expiry info — assume valid
    };

    let expires = match chrono::DateTime::parse_from_rfc3339(expires_at) {
        Ok(dt) => dt,
        Err(_) => {
            // Try parsing as milliseconds timestamp
            if let Ok(ms) = expires_at.parse::<i64>() {
                match chrono::DateTime::from_timestamp_millis(ms) {
                    Some(dt) => dt.fixed_offset(),
                    None => return false,
                }
            } else {
                return false;
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

    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "-m", "15",
            "-X", "POST",
            "-H", "Content-Type: application/json",
            "-d", &body.to_string(),
            OAUTH_TOKEN_ENDPOINT,
        ])
        .output()
        .context("Failed to call OAuth token endpoint")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("curl failed: {}", stderr));
    }

    let response_body = String::from_utf8_lossy(&output.stdout).to_string();

    // Check for error response
    if let Ok(err_json) = serde_json::from_str::<serde_json::Value>(&response_body) {
        if let Some(err) = err_json.get("error") {
            let err_msg = err
                .as_str()
                .or_else(|| err.get("message").and_then(|m| m.as_str()))
                .unwrap_or("Unknown error");
            return Err(anyhow::anyhow!("OAuth refresh failed: {}", err_msg));
        }
    }

    let token_response: OAuthTokenResponse = serde_json::from_str(&response_body)
        .context("Failed to parse OAuth token response")?;

    // Update the credential JSON with new tokens
    let mut creds_json: serde_json::Value = serde_json::from_str(current_creds)
        .context("Failed to parse current credentials")?;

    if let Some(oauth) = creds_json.get_mut("claudeAiOauth") {
        oauth["accessToken"] = serde_json::Value::String(token_response.access_token);
        oauth["refreshToken"] = serde_json::Value::String(token_response.refresh_token);

        // Update expiresAt if expires_in is provided
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
pub fn refresh_account_credentials(account_id: &str, is_active: bool) -> Result<RefreshResult> {
    // Read current backup credentials
    let current_creds = super::keychain::restore_credentials(account_id)
        .context("Failed to read backup credentials")?;

    // Attempt refresh
    let new_creds = refresh_oauth_token(&current_creds)?;

    // Save refreshed credentials to backup
    super::keychain::backup_credentials(account_id, &new_creds)
        .context("Failed to save refreshed credentials to backup")?;

    // If this is the active account, also update the global keychain
    if is_active {
        super::keychain::write_active_credentials(&new_creds)
            .context("Failed to update global keychain with refreshed credentials")?;
    }

    log::info!("Successfully refreshed token for account {}", account_id);

    Ok(RefreshResult {
        success: true,
        message: "Token refreshed successfully".to_string(),
    })
}
