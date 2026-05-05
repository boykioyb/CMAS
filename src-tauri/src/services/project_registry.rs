use crate::models::{Project, ProjectRegistry, ProjectSource, ScanRoot};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref REGISTRY_LOCK: Mutex<()> = Mutex::new(());
}

fn registry_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_default();
    home.join(".claude-switcher").join("projects.json")
}

fn ensure_dir() {
    let home = dirs::home_dir().unwrap_or_default();
    let dir = home.join(".claude-switcher");
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }
}

pub fn load() -> ProjectRegistry {
    let _guard = REGISTRY_LOCK.lock().ok();
    let path = registry_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}

pub fn save(registry: &ProjectRegistry) -> Result<(), String> {
    let _guard = REGISTRY_LOCK.lock().ok();
    ensure_dir();
    let json = serde_json::to_string_pretty(registry).map_err(|e| e.to_string())?;
    fs::write(registry_path(), json).map_err(|e| e.to_string())
}

/// Canonicalize path; falls back to as-is if path doesn't exist.
pub fn canonicalize(path: &str) -> String {
    Path::new(path)
        .canonicalize()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string())
}

/// Insert (or update) a manual project; returns the project id.
pub fn upsert_manual(registry: &mut ProjectRegistry, name: &str, path: &str) -> String {
    let canonical = canonicalize(path);
    if let Some(existing) = registry.projects.iter_mut().find(|p| p.path == canonical) {
        existing.last_seen_at = chrono::Utc::now().to_rfc3339();
        existing.missing = !Path::new(&canonical).exists();
        return existing.id.clone();
    }
    let id = uuid::Uuid::new_v4().to_string();
    let project = Project {
        id: id.clone(),
        name: name.to_string(),
        path: canonical.clone(),
        source: ProjectSource::Manual,
        last_seen_at: chrono::Utc::now().to_rfc3339(),
        missing: !Path::new(&canonical).exists(),
        favorite: false,
    };
    registry.projects.push(project);
    id
}

pub fn add_root(registry: &mut ProjectRegistry, path: &str) -> ScanRoot {
    let canonical = canonicalize(path);
    if let Some(existing) = registry.roots.iter().find(|r| r.path == canonical) {
        return existing.clone();
    }
    let root = ScanRoot {
        id: uuid::Uuid::new_v4().to_string(),
        path: canonical,
        max_depth: 2,
        enabled: true,
        last_scanned_at: None,
    };
    registry.roots.push(root.clone());
    root
}

pub fn remove_root(registry: &mut ProjectRegistry, root_id: &str) {
    registry.roots.retain(|r| r.id != root_id);
    // Demote scanned projects under this root to manual so links survive,
    // user can prune them manually if desired.
    for proj in registry.projects.iter_mut() {
        if let ProjectSource::Scanned { root_id: rid } = &proj.source {
            if rid == root_id {
                proj.source = ProjectSource::Manual;
            }
        }
    }
}
