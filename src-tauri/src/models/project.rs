use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ProjectSource {
    Scanned { root_id: String },
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub source: ProjectSource,
    pub last_seen_at: String,
    #[serde(default)]
    pub missing: bool,
    #[serde(default)]
    pub favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanRoot {
    pub id: String,
    pub path: String,
    #[serde(default = "default_max_depth")]
    pub max_depth: u8,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub last_scanned_at: Option<String>,
}

fn default_max_depth() -> u8 {
    2
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectRegistry {
    #[serde(default)]
    pub roots: Vec<ScanRoot>,
    #[serde(default)]
    pub projects: Vec<Project>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    pub root_id: String,
    pub scanned: u32,
    pub added: u32,
    pub updated: u32,
}
