use anyhow::{Context, Result};
use chrono::{Datelike, Duration, NaiveDate, Utc};
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use crate::models::{ModelUsage, UsageInfo};

fn claude_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".claude")
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DailyActivity {
    pub date: String,
    pub message_count: u64,
    pub session_count: u64,
    pub tool_call_count: u64,
}

/// Read stats-cache.json for daily activity (legacy fallback)
pub fn read_stats_cache() -> Result<Vec<DailyActivity>> {
    let path = claude_dir().join("stats-cache.json");
    let content = fs::read_to_string(&path).context("Cannot read stats-cache.json")?;
    let data: serde_json::Value = serde_json::from_str(&content)?;

    let activities = data
        .get("dailyActivity")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    Some(DailyActivity {
                        date: v.get("date")?.as_str()?.to_string(),
                        message_count: v.get("messageCount")?.as_u64().unwrap_or(0),
                        session_count: v.get("sessionCount")?.as_u64().unwrap_or(0),
                        tool_call_count: v.get("toolCallCount")?.as_u64().unwrap_or(0),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(activities)
}

/// Aggregated usage data from session JSONL files
#[derive(Debug, Default)]
struct SessionUsageData {
    messages_5h: u64,
    messages_today: u64,
    messages_week: u64,
    sessions_5h: u64,
    sessions_today: u64,
    tool_calls_today: u64,
    output_tokens_5h: u64,
    output_tokens_today: u64,
    output_tokens_week: u64,
    // Per-model breakdown
    opus: ModelUsage,
    sonnet: ModelUsage,
    // Earliest message timestamp within the 5h window (for calculating reset time)
    earliest_message_5h: Option<chrono::DateTime<Utc>>,
}

/// Classify model name into category
fn classify_model(model: &str) -> Option<&'static str> {
    let m = model.to_lowercase();
    if m.contains("opus") {
        Some("opus")
    } else if m.contains("sonnet") {
        Some("sonnet")
    } else {
        None
    }
}

/// Convert a project filesystem path to the directory name used in ~/.claude/projects/
/// e.g., "/Users/john/Projects/foo" → "-Users-john-Projects-foo"
fn path_to_project_dir_name(path: &str) -> String {
    path.replace('/', "-").replace('\\', "-")
}

/// Scan session JSONL files under ~/.claude/projects/ for usage data.
/// If `project_paths` is provided, only scan directories matching those project paths.
/// If empty, scan all projects.
fn scan_session_usage(project_paths: &[String]) -> SessionUsageData {
    let projects_dir = claude_dir().join("projects");
    if !projects_dir.exists() {
        return SessionUsageData::default();
    }

    let now = Utc::now();
    let five_hours_ago = now - Duration::hours(5);
    let today: NaiveDate = now.date_naive();
    let week_ago = now - Duration::days(7);

    // Build set of allowed directory names (if filtering)
    let filter_dirs: HashSet<String> = project_paths
        .iter()
        .map(|p| path_to_project_dir_name(p))
        .collect();
    let has_filter = !filter_dirs.is_empty();

    // Collect all .jsonl files modified in last 7 days
    let mut jsonl_files: Vec<PathBuf> = Vec::new();
    if let Ok(project_entries) = fs::read_dir(&projects_dir) {
        for project_entry in project_entries.flatten() {
            let project_path = project_entry.path();
            if !project_path.is_dir() {
                continue;
            }
            // Filter by allowed project directories
            if has_filter {
                let dir_name = project_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if !filter_dirs.contains(&dir_name) {
                    continue;
                }
            }
            if let Ok(files) = fs::read_dir(&project_path) {
                for file in files.flatten() {
                    let path = file.path();
                    if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                        continue;
                    }
                    // Pre-filter by file modification time
                    if let Ok(meta) = file.metadata() {
                        if let Ok(modified) = meta.modified() {
                            let mod_time: chrono::DateTime<Utc> = modified.into();
                            if mod_time > week_ago {
                                jsonl_files.push(path);
                            }
                        }
                    }
                }
            }
        }
    }

    let mut data = SessionUsageData::default();
    let mut session_ids_5h: HashSet<String> = HashSet::new();
    let mut session_ids_today: HashSet<String> = HashSet::new();

    // Track last model used per session for attributing user messages
    let mut last_model_in_session: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    for path in &jsonl_files {
        let session_id = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let file = match fs::File::open(path) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };

            // Quick pre-filter: skip lines without "timestamp"
            if !line.contains("\"timestamp\"") {
                continue;
            }

            let entry: serde_json::Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue,
            };

            // Extract and parse timestamp
            let timestamp = match entry.get("timestamp").and_then(|v| v.as_str()) {
                Some(ts) => match chrono::DateTime::parse_from_rfc3339(ts) {
                    Ok(dt) => dt.with_timezone(&Utc),
                    Err(_) => continue,
                },
                None => continue,
            };

            // Determine time windows
            let in_week = timestamp > week_ago;
            if !in_week {
                continue;
            }
            let in_today = timestamp.date_naive() == today;
            let in_5h = timestamp > five_hours_ago;

            // Get message role and model
            let role = entry
                .get("message")
                .and_then(|m| m.get("role"))
                .and_then(|r| r.as_str());

            let model = entry
                .get("message")
                .and_then(|m| m.get("model"))
                .and_then(|m| m.as_str())
                .unwrap_or("");

            // Track model for this session
            if !model.is_empty() {
                last_model_in_session.insert(session_id.clone(), model.to_string());
            }

            // Count user messages (each user turn = 1 message)
            if role == Some("user") {
                if in_5h {
                    data.messages_5h += 1;
                    session_ids_5h.insert(session_id.clone());
                    // Track the earliest message in the 5h window
                    if data.earliest_message_5h.is_none() || Some(timestamp) < data.earliest_message_5h {
                        data.earliest_message_5h = Some(timestamp);
                    }
                }
                if in_today {
                    data.messages_today += 1;
                    session_ids_today.insert(session_id.clone());
                }
                data.messages_week += 1;

                // Attribute user message to the model used in this session
                let session_model = last_model_in_session.get(&session_id).map(|s| s.as_str()).unwrap_or("");
                match classify_model(session_model) {
                    Some("opus") => {
                        if in_5h { data.opus.messages_5h += 1; }
                        if in_today { data.opus.messages_today += 1; }
                        data.opus.messages_week += 1;
                    }
                    Some("sonnet") => {
                        if in_5h { data.sonnet.messages_5h += 1; }
                        if in_today { data.sonnet.messages_today += 1; }
                        data.sonnet.messages_week += 1;
                    }
                    _ => {}
                }
            }

            // Extract token usage and tool calls from assistant messages
            if role == Some("assistant") {
                if let Some(msg) = entry.get("message") {
                    // Token usage
                    if let Some(usage) = msg.get("usage") {
                        let output =
                            usage.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);

                        if in_5h {
                            data.output_tokens_5h += output;
                        }
                        if in_today {
                            data.output_tokens_today += output;
                        }
                        data.output_tokens_week += output;

                        // Per-model token tracking
                        match classify_model(model) {
                            Some("opus") => {
                                if in_5h { data.opus.output_tokens_5h += output; }
                                if in_today { data.opus.output_tokens_today += output; }
                                data.opus.output_tokens_week += output;
                            }
                            Some("sonnet") => {
                                if in_5h { data.sonnet.output_tokens_5h += output; }
                                if in_today { data.sonnet.output_tokens_today += output; }
                                data.sonnet.output_tokens_week += output;
                            }
                            _ => {}
                        }
                    }

                    // Tool call count
                    if in_today {
                        if let Some(content) = msg.get("content").and_then(|c| c.as_array()) {
                            let tool_count = content
                                .iter()
                                .filter(|c| {
                                    c.get("type").and_then(|t| t.as_str()) == Some("tool_use")
                                })
                                .count() as u64;
                            data.tool_calls_today += tool_count;
                        }
                    }
                }
            }
        }
    }

    data.sessions_5h = session_ids_5h.len() as u64;
    data.sessions_today = session_ids_today.len() as u64;
    data
}

/// Scan debug logs for recent rate limit errors
pub fn find_last_rate_limit() -> Option<String> {
    let debug_dir = claude_dir().join("debug");
    if !debug_dir.exists() {
        return None;
    }

    let mut entries: Vec<_> = fs::read_dir(&debug_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "txt")
                .unwrap_or(false)
        })
        .collect();

    entries.sort_by(|a, b| {
        b.metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            .cmp(
                &a.metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
            )
    });

    for entry in entries.iter().take(5) {
        if let Ok(content) = fs::read_to_string(entry.path()) {
            for line in content.lines().rev() {
                if line.contains("rate_limit_error") || line.contains("429") {
                    if let Some(ts) = line.split_whitespace().next() {
                        if ts.len() > 20 && ts.contains('T') {
                            return Some(ts.to_string());
                        }
                    }
                }
            }
        }
    }

    None
}

/// Public wrapper for read_credential_metadata
pub fn read_credential_metadata_pub() -> (Option<String>, Option<String>) {
    read_credential_metadata()
}

/// Read subscription type and rate limit tier from the active account's backup.
/// Uses CMAS backup (not global keychain) to avoid macOS keychain prompts.
fn read_credential_metadata() -> (Option<String>, Option<String>) {
    // Find active account and read from its backup
    let accounts = crate::commands::account::load_accounts();
    let active = match accounts.iter().find(|a| a.is_active) {
        Some(a) => a,
        None => return (None, None),
    };
    if let Ok(creds_str) = crate::services::keychain::restore_credentials(&active.id) {
        if let Ok(creds) = serde_json::from_str::<serde_json::Value>(&creds_str) {
            let oauth = creds.get("claudeAiOauth").unwrap_or(&creds);
            let sub_type = oauth
                .get("subscriptionType")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let tier = oauth
                .get("rateLimitTier")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            return (sub_type, tier);
        }
    }
    (None, None)
}

/// Build usage info filtered by specific project paths (for per-account usage).
/// If project_paths is empty, scans all projects (machine-wide).
pub fn get_usage_info_for_projects(project_paths: &[String]) -> UsageInfo {
    let now = Utc::now();

    // Scan session JSONL files for real usage data
    let session_data = scan_session_usage(project_paths);

    build_usage_info(now, session_data)
}

/// Build UsageInfo from scanned session data
fn build_usage_info(now: chrono::DateTime<Utc>, session_data: SessionUsageData) -> UsageInfo {
    // Check for rate limits
    let last_rate_limit_at = find_last_rate_limit();
    let is_rate_limited = last_rate_limit_at
        .as_ref()
        .map(|ts| {
            if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(ts) {
                let limit_time = parsed.with_timezone(&Utc);
                now - limit_time < Duration::minutes(30)
            } else {
                false
            }
        })
        .unwrap_or(false);

    let estimated_reset_at = if is_rate_limited {
        last_rate_limit_at.as_ref().and_then(|ts| {
            chrono::DateTime::parse_from_rfc3339(ts)
                .ok()
                .map(|parsed| (parsed.with_timezone(&Utc) + Duration::hours(5)).to_rfc3339())
        })
    } else {
        None
    };

    // Session reset: when the oldest message in the 5h window drops out
    let session_reset_at = session_data.earliest_message_5h.map(|earliest| {
        (earliest + Duration::hours(5)).to_rfc3339()
    });

    // Weekly reset: next Monday 00:00 UTC
    let weekly_reset_at = {
        let today = now.date_naive();
        let weekday = today.weekday();
        let days_until_monday = match weekday {
            chrono::Weekday::Mon => 7,
            chrono::Weekday::Tue => 6,
            chrono::Weekday::Wed => 5,
            chrono::Weekday::Thu => 4,
            chrono::Weekday::Fri => 3,
            chrono::Weekday::Sat => 2,
            chrono::Weekday::Sun => 1,
        };
        let next_monday = today + Duration::days(days_until_monday);
        Some(format!("{}T00:00:00+00:00", next_monday))
    };

    let (subscription_type, rate_limit_tier) = read_credential_metadata();

    UsageInfo {
        messages_today: session_data.messages_today,
        messages_5h_window: session_data.messages_5h,
        sessions_today: session_data.sessions_today,
        tool_calls_today: session_data.tool_calls_today,
        messages_week: session_data.messages_week,
        output_tokens_5h: session_data.output_tokens_5h,
        output_tokens_today: session_data.output_tokens_today,
        output_tokens_week: session_data.output_tokens_week,
        last_rate_limit_at,
        is_rate_limited,
        estimated_reset_at,
        session_reset_at,
        weekly_reset_at,
        subscription_type,
        rate_limit_tier,
        last_checked_at: now.to_rfc3339(),
        opus_usage: session_data.opus,
        sonnet_usage: session_data.sonnet,
    }
}

/// Build complete usage info by scanning all session data (machine-wide)
pub fn get_usage_info() -> UsageInfo {
    let now = Utc::now();
    let session_data = scan_session_usage(&[]);
    build_usage_info(now, session_data)
}
