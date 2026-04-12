use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::models::Account;

// ── Data structures ──────────────────────────────────────────────

/// A single cost usage record (per assistant message)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostUsageRecord {
    pub timestamp: String,
    pub account_email: String,
    pub account_label: Option<String>,
    pub account_id: Option<String>,
    pub model: String,
    pub model_display: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub total_tokens: u64,
    pub estimated_cost_usd: f64,
    pub session_id: String,
}

/// Persistent cache stored on disk
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CostUsageCache {
    /// file_path → last modified time (unix secs)
    file_mtimes: HashMap<String, i64>,
    /// All cached records
    records: Vec<CostUsageRecord>,
}

// ── Pricing ──────────────────────────────────────────────────────

struct ModelPricing {
    input_per_mtok: f64,
    output_per_mtok: f64,
    cache_read_per_mtok: f64,
    cache_creation_per_mtok: f64,
}

fn get_model_pricing(model: &str) -> ModelPricing {
    let m = model.to_lowercase();
    if m.contains("opus") {
        ModelPricing {
            input_per_mtok: 15.0,
            output_per_mtok: 75.0,
            cache_read_per_mtok: 1.5,
            cache_creation_per_mtok: 18.75,
        }
    } else if m.contains("haiku") {
        ModelPricing {
            input_per_mtok: 0.8,
            output_per_mtok: 4.0,
            cache_read_per_mtok: 0.08,
            cache_creation_per_mtok: 1.0,
        }
    } else {
        ModelPricing {
            input_per_mtok: 3.0,
            output_per_mtok: 15.0,
            cache_read_per_mtok: 0.3,
            cache_creation_per_mtok: 3.75,
        }
    }
}

fn calculate_cost(
    pricing: &ModelPricing,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_creation_tokens: u64,
) -> f64 {
    (input_tokens as f64 / 1_000_000.0) * pricing.input_per_mtok
        + (output_tokens as f64 / 1_000_000.0) * pricing.output_per_mtok
        + (cache_read_tokens as f64 / 1_000_000.0) * pricing.cache_read_per_mtok
        + (cache_creation_tokens as f64 / 1_000_000.0) * pricing.cache_creation_per_mtok
}

fn model_display_name(model: &str) -> String {
    let m = model.to_lowercase();
    if m.contains("opus") && m.contains("4-6") {
        "Opus 4.6".to_string()
    } else if m.contains("opus") {
        "Opus 4".to_string()
    } else if m.contains("sonnet") && m.contains("4-6") {
        "Sonnet 4.6".to_string()
    } else if m.contains("sonnet") {
        "Sonnet 4".to_string()
    } else if m.contains("haiku") {
        "Haiku".to_string()
    } else if model.is_empty() {
        "Unknown".to_string()
    } else {
        model.to_string()
    }
}

// ── Paths ────────────────────────────────────────────────────────

fn claude_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".claude")
}

fn cache_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".claude-switcher")
        .join("cost-usage-cache.json")
}

// ── Cache I/O ────────────────────────────────────────────────────

fn load_cache() -> CostUsageCache {
    let path = cache_path();
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => CostUsageCache::default(),
    }
}

fn save_cache(cache: &CostUsageCache) {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(cache) {
        let _ = fs::write(&path, json);
    }
}

// ── Account mapping ──────────────────────────────────────────────

fn build_project_account_map(
    accounts: &[Account],
) -> HashMap<String, (String, Option<String>, String)> {
    let mut map = HashMap::new();
    for account in accounts {
        for project in &account.projects {
            let dir_name = project.path.replace('/', "-");
            map.insert(
                dir_name,
                (account.email.clone(), account.label.clone(), account.id.clone()),
            );
        }
    }
    map
}

fn file_mtime_secs(path: &PathBuf) -> Option<i64> {
    fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| {
            let dt: chrono::DateTime<Utc> = t.into();
            dt.timestamp()
        })
}

// ── Parse a single JSONL file ────────────────────────────────────

fn parse_jsonl_file(
    path: &PathBuf,
    dir_name: &str,
    project_map: &HashMap<String, (String, Option<String>, String)>,
) -> Vec<CostUsageRecord> {
    let (account_email, account_label, account_id) = project_map
        .get(dir_name)
        .cloned()
        .unwrap_or_else(|| ("Unknown".to_string(), None, String::new()));

    let session_id = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return vec![],
    };

    let mut records = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for line in BufReader::new(file).lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        if !line.contains("\"usage\"") || !line.contains("\"assistant\"") {
            continue;
        }

        let entry: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if entry.get("type").and_then(|v| v.as_str()) != Some("assistant") {
            continue;
        }
        let message = match entry.get("message") {
            Some(m) => m,
            None => continue,
        };
        if message.get("role").and_then(|r| r.as_str()) != Some("assistant") {
            continue;
        }

        // Dedup within this file
        let msg_id = message.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let req_id = entry.get("requestId").and_then(|v| v.as_str()).unwrap_or("");
        let dedup_key = format!("{}:{}", msg_id, req_id);
        if dedup_key != ":" && !seen.insert(dedup_key) {
            continue;
        }

        let timestamp_str = match entry.get("timestamp").and_then(|v| v.as_str()) {
            Some(ts) => ts.to_string(),
            None => continue,
        };
        if chrono::DateTime::parse_from_rfc3339(&timestamp_str).is_err() {
            continue;
        }

        let usage = match message.get("usage") {
            Some(u) => u,
            None => continue,
        };

        let input_tokens = usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let output_tokens = usage.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let cache_read_tokens = usage.get("cache_read_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let cache_creation_tokens = usage.get("cache_creation_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);

        if input_tokens == 0 && output_tokens == 0 {
            continue;
        }

        let model = message.get("model").and_then(|m| m.as_str()).unwrap_or("unknown").to_string();
        let total_tokens = input_tokens + output_tokens + cache_read_tokens + cache_creation_tokens;
        let pricing = get_model_pricing(&model);
        let cost = (calculate_cost(&pricing, input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens) * 10000.0).round() / 10000.0;

        records.push(CostUsageRecord {
            timestamp: timestamp_str,
            account_email: account_email.clone(),
            account_label: account_label.clone(),
            account_id: if account_id.is_empty() { None } else { Some(account_id.clone()) },
            model: model.clone(),
            model_display: model_display_name(&model),
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_creation_tokens,
            total_tokens,
            estimated_cost_usd: cost,
            session_id: session_id.clone(),
        });
    }

    records
}

// ── Core: incremental scan with cache ────────────────────────────

pub fn scan_cost_usage_cached(accounts: &[Account]) -> Vec<CostUsageRecord> {
    let projects_dir = claude_dir().join("projects");
    if !projects_dir.exists() {
        return vec![];
    }

    let mut cache = load_cache();
    let project_map = build_project_account_map(accounts);

    // Prune records older than 90 days
    let prune_cutoff = (Utc::now() - Duration::days(90)).to_rfc3339();
    let before_prune = cache.records.len();
    cache.records.retain(|r| r.timestamp > prune_cutoff);

    // Discover all JSONL files
    let mut current_files: Vec<(String, PathBuf, String, i64)> = Vec::new(); // (path_str, path, dir_name, mtime)

    if let Ok(project_entries) = fs::read_dir(&projects_dir) {
        for entry in project_entries.flatten() {
            let project_path = entry.path();
            if !project_path.is_dir() {
                continue;
            }
            let dir_name = project_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            if let Ok(files) = fs::read_dir(&project_path) {
                for file in files.flatten() {
                    let path = file.path();
                    if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                        continue;
                    }
                    if let Some(mtime) = file_mtime_secs(&path) {
                        let path_str = path.to_string_lossy().to_string();
                        current_files.push((path_str, path, dir_name.clone(), mtime));
                    }
                }
            }
        }
    }

    // Find files that need (re-)scanning
    let mut dirty = false;
    let mut scanned_count = 0usize;
    let current_path_set: HashSet<String> = current_files.iter().map(|(ps, _, _, _)| ps.clone()).collect();

    for (path_str, path, dir_name, mtime) in &current_files {
        let cached_mtime = cache.file_mtimes.get(path_str).copied().unwrap_or(0);
        if *mtime <= cached_mtime {
            continue; // Already up to date
        }

        // Remove stale records for this session
        let session_id = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        cache.records.retain(|r| r.session_id != session_id);

        // Parse fresh
        let new_records = parse_jsonl_file(path, dir_name, &project_map);
        cache.records.extend(new_records);
        cache.file_mtimes.insert(path_str.clone(), *mtime);
        scanned_count += 1;
        dirty = true;
    }

    // Remove mtime entries for deleted files
    let before_clean = cache.file_mtimes.len();
    cache.file_mtimes.retain(|k, _| current_path_set.contains(k));
    if cache.file_mtimes.len() != before_clean {
        dirty = true;
    }

    if dirty || cache.records.len() != before_prune {
        cache.records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        save_cache(&cache);
        log::info!(
            "cost-usage cache: scanned {} files, total {} records",
            scanned_count,
            cache.records.len()
        );
    }

    cache.records
}

// ── Tauri command ────────────────────────────────────────────────

/// Get cost usage history with incremental cache.
/// `days` filters the returned records (cache keeps up to 90 days).
#[tauri::command]
pub fn get_cost_usage_history(days: Option<u32>) -> Result<Vec<CostUsageRecord>, String> {
    let days = days.unwrap_or(30);
    let accounts = crate::commands::account::load_accounts();
    let all_records = scan_cost_usage_cached(&accounts);

    let cutoff = (Utc::now() - Duration::days(days as i64)).to_rfc3339();
    let filtered: Vec<CostUsageRecord> = all_records
        .into_iter()
        .filter(|r| r.timestamp > cutoff)
        .collect();

    Ok(filtered)
}
