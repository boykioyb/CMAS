use crate::services::keychain;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RealUsageData {
    pub success: bool,
    pub authenticated: bool,
    pub session_percent: Option<u32>,
    pub weekly_all_percent: Option<u32>,
    pub weekly_sonnet_percent: Option<u32>,
    pub session_reset: Option<String>,
    pub weekly_reset: Option<String>,
    /// Seconds until rate limit resets (from retry-after header)
    pub retry_after: Option<u32>,
    /// Error message if request failed
    pub error_message: Option<String>,
}

impl Default for RealUsageData {
    fn default() -> Self {
        Self {
            success: false,
            authenticated: true,
            session_percent: None,
            weekly_all_percent: None,
            weekly_sonnet_percent: None,
            session_reset: None,
            weekly_reset: None,
            retry_after: None,
            error_message: None,
        }
    }
}

/// Fetch usage data via OAuth API.
/// Uses curl with -i to capture response headers (for retry-after).
fn fetch_usage_via_api(access_token: &str) -> Result<RealUsageData, String> {
    let output = std::process::Command::new("curl")
        .args([
            "-si",          // include headers
            "-m", "15",
            "-H", &format!("Authorization: Bearer {}", access_token),
            "-H", "anthropic-beta: oauth-2025-04-20",
            "-H", "Content-Type: application/json",
            "https://api.anthropic.com/api/oauth/usage",
        ])
        .output()
        .map_err(|e| format!("Failed to call usage API: {}", e))?;

    let raw = String::from_utf8_lossy(&output.stdout).to_string();

    // Split headers and body
    let (headers, body) = match raw.find("\r\n\r\n") {
        Some(idx) => (&raw[..idx], &raw[idx + 4..]),
        None => ("", raw.as_str()),
    };

    // Extract HTTP status
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);

    // Extract retry-after header
    let retry_after: Option<u32> = headers
        .lines()
        .find(|l| l.to_lowercase().starts_with("retry-after:"))
        .and_then(|l| l.split(':').nth(1))
        .and_then(|v| v.trim().parse().ok());

    log::info!("usage API: status={}, retry_after={:?}, body_len={}", status, retry_after, body.len());

    // Handle rate limit (429)
    if status == 429 {
        return Ok(RealUsageData {
            success: false,
            authenticated: true,
            retry_after,
            error_message: Some(format!("Rate limited")),
            ..Default::default()
        });
    }

    // Handle auth errors
    if status == 401 {
        return Ok(RealUsageData {
            success: false,
            authenticated: false,
            error_message: Some("Token expired or invalid".to_string()),
            ..Default::default()
        });
    }

    // Parse JSON body
    let json: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("Invalid JSON: {}", e))?;

    log::info!("usage API response: {}", &body[..body.len().min(500)]);

    // Check for error in body
    if let Some(err) = json.get("error") {
        let err_type = err.get("type").and_then(|t| t.as_str()).unwrap_or("unknown");
        let err_msg = err.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error");

        return Ok(RealUsageData {
            success: false,
            authenticated: err_type != "authentication_error",
            error_message: Some(err_msg.to_string()),
            retry_after,
            ..Default::default()
        });
    }

    // Parse usage windows
    let session_percent = parse_window_percent(&json, "five_hour");
    let weekly_all_percent = parse_window_percent(&json, "seven_day");
    let weekly_sonnet_percent = parse_window_percent(&json, "seven_day_sonnet");
    let session_reset = parse_window_reset(&json, "five_hour");
    let weekly_reset = parse_window_reset(&json, "seven_day");

    Ok(RealUsageData {
        success: session_percent.is_some() || weekly_all_percent.is_some(),
        authenticated: true,
        session_percent,
        weekly_all_percent,
        weekly_sonnet_percent,
        session_reset,
        weekly_reset,
        retry_after: None,
        error_message: None,
    })
}

fn parse_window_percent(json: &serde_json::Value, window_key: &str) -> Option<u32> {
    let window = json.get(window_key)?;

    // Null window = model not tracked
    if window.is_null() {
        return None;
    }

    // API returns "utilization" as percentage (0.0 - 100.0)
    if let Some(pct) = window.get("utilization").and_then(|v| v.as_f64()) {
        return Some(pct.round() as u32);
    }

    // Fallback patterns
    if let Some(pct) = window.get("percent_used").and_then(|v| v.as_f64()) {
        return Some(pct.round() as u32);
    }

    None
}

fn parse_window_reset(json: &serde_json::Value, window_key: &str) -> Option<String> {
    let window = json.get(window_key)?;

    if window.is_null() {
        return None;
    }

    // API returns "resets_at" as ISO 8601 timestamp
    let reset_str = window
        .get("resets_at")
        .or_else(|| window.get("reset_at"))
        .and_then(|v| v.as_str())?;

    // Convert to human-readable relative time
    if let Ok(reset_time) = chrono::DateTime::parse_from_rfc3339(reset_str) {
        let now = chrono::Utc::now();
        let duration = reset_time.signed_duration_since(now);
        let total_mins = duration.num_minutes();

        if total_mins <= 0 {
            return Some("now".to_string());
        } else if total_mins < 60 {
            return Some(format!("{} min", total_mins));
        } else if total_mins < 1440 {
            let hours = total_mins / 60;
            let mins = total_mins % 60;
            if mins > 0 {
                return Some(format!("{}h {}m", hours, mins));
            }
            return Some(format!("{}h", hours));
        } else {
            let days = total_mins / 1440;
            // Format as day name
            let weekday = reset_time.format("%a").to_string();
            let time = reset_time.format("%H:%M").to_string();
            if days <= 7 {
                return Some(format!("{} {}", weekday, time));
            }
            return Some(reset_time.format("%d/%m %H:%M").to_string());
        }
    }

    Some(reset_str.to_string())
}

fn extract_access_token(creds: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(creds).ok()?;
    v.get("claudeAiOauth")
        .and_then(|o| o.get("accessToken"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
}

/// Fetch real usage data via Claude OAuth API (active account).
#[tauri::command]
pub async fn scrape_claude_usage() -> Result<RealUsageData, String> {
    let creds = keychain::read_active_credentials().map_err(|e| format!("{}", e))?;
    let token = extract_access_token(&creds).ok_or("No access token in credentials")?;
    fetch_usage_via_api(&token)
}

/// Fetch usage for a specific account by ID.
#[tauri::command]
pub fn fetch_account_usage(account_id: String) -> Result<RealUsageData, String> {
    let creds = keychain::restore_credentials(&account_id).map_err(|e| format!("{}", e))?;
    let token = extract_access_token(&creds).ok_or("No access token in backup credentials")?;
    fetch_usage_via_api(&token)
}

/// Open claude.ai usage page in default browser.
#[tauri::command]
pub async fn open_claude_login() -> Result<(), String> {
    std::process::Command::new("open")
        .arg("https://claude.ai/settings/usage")
        .spawn()
        .map_err(|e| format!("Cannot open browser: {}", e))?;
    Ok(())
}
