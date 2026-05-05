use crate::models::{Project, ProjectRegistry, ProjectSource, ScanRoot, ScanSummary};
use crate::services::project_registry;
use std::path::Path;

const MARKER_FILES: &[&str] = &[
    ".git",
    "package.json",
    "Cargo.toml",
    "pyproject.toml",
    "go.mod",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "composer.json",
    "Gemfile",
    "pubspec.yaml",
];

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "dist",
    "build",
    ".venv",
    "venv",
    "__pycache__",
    ".next",
    ".nuxt",
    ".cache",
    ".idea",
    ".vscode",
    ".gradle",
    ".tox",
    "vendor",
];

fn is_project_dir(dir: &Path) -> bool {
    MARKER_FILES.iter().any(|m| dir.join(m).exists())
}

fn folder_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

fn walk(dir: &Path, depth: u8, max_depth: u8, out: &mut Vec<String>) {
    if depth > max_depth {
        return;
    }
    if is_project_dir(dir) {
        out.push(dir.to_string_lossy().to_string());
        // Don't descend into a project (avoid finding sub-packages of monorepos
        // unless user explicitly wants it via deeper max_depth on a different root)
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') && name != ".git" {
            // Skip hidden dirs except .git which we already handle
            continue;
        }
        if SKIP_DIRS.iter().any(|s| s == &name) {
            continue;
        }
        walk(&path, depth + 1, max_depth, out);
    }
}

/// Scan a single root, upserting found projects into the registry.
pub fn scan_root(registry: &mut ProjectRegistry, root: &ScanRoot) -> ScanSummary {
    let mut found = Vec::new();
    walk(Path::new(&root.path), 0, root.max_depth, &mut found);

    let mut added = 0u32;
    let mut updated = 0u32;
    let now = chrono::Utc::now().to_rfc3339();

    for path in &found {
        let canonical = project_registry::canonicalize(path);
        if let Some(existing) = registry.projects.iter_mut().find(|p| p.path == canonical) {
            existing.last_seen_at = now.clone();
            existing.missing = false;
            // If user added it manually first then it shows up in a scan,
            // adopt the scan source so user can see it's now part of a root.
            if matches!(existing.source, ProjectSource::Manual) {
                existing.source = ProjectSource::Scanned {
                    root_id: root.id.clone(),
                };
            }
            updated += 1;
        } else {
            registry.projects.push(Project {
                id: uuid::Uuid::new_v4().to_string(),
                name: folder_name(Path::new(&canonical)),
                path: canonical,
                source: ProjectSource::Scanned {
                    root_id: root.id.clone(),
                },
                last_seen_at: now.clone(),
                missing: false,
                favorite: false,
            });
            added += 1;
        }
    }

    // Mark previously-scanned projects under this root that weren't found this pass as missing
    let found_set: std::collections::HashSet<String> = found
        .iter()
        .map(|p| project_registry::canonicalize(p))
        .collect();
    for proj in registry.projects.iter_mut() {
        if let ProjectSource::Scanned { root_id } = &proj.source {
            if root_id == &root.id && !found_set.contains(&proj.path) {
                proj.missing = true;
            }
        }
    }

    if let Some(r) = registry.roots.iter_mut().find(|r| r.id == root.id) {
        r.last_scanned_at = Some(now);
    }

    ScanSummary {
        root_id: root.id.clone(),
        scanned: found.len() as u32,
        added,
        updated,
    }
}

/// Scan all enabled roots.
pub fn scan_all(registry: &mut ProjectRegistry) -> Vec<ScanSummary> {
    let roots: Vec<ScanRoot> = registry
        .roots
        .iter()
        .filter(|r| r.enabled)
        .cloned()
        .collect();
    roots.iter().map(|r| scan_root(registry, r)).collect()
}
