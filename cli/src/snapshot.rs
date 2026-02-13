use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub sync_folder: String,
    pub machine: String,
    pub synced_at: chrono::DateTime<chrono::Utc>,
    pub files: BTreeMap<String, FileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileEntry {
    pub size: u64,
    pub mtime_secs: i64,
    pub hash: String,
    #[serde(default)]
    pub is_dir: bool,
}

impl Snapshot {
    pub fn new(sync_folder: &str, machine: &str) -> Self {
        Self {
            sync_folder: sync_folder.to_string(),
            machine: machine.to_string(),
            synced_at: chrono::Utc::now(),
            files: BTreeMap::new(),
        }
    }

    pub fn snapshot_filename(ssd_rel: &str) -> String {
        let safe_name = ssd_rel.replace('/', "_").replace('\\', "_").replace(':', "_");
        format!("{}.json", safe_name)
    }

    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read snapshot: {}", path.display()))?;
        let snap: Snapshot =
            serde_json::from_str(&content).with_context(|| "Failed to parse snapshot")?;
        Ok(snap)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn load_or_empty(path: &Path, sync_folder: &str, machine: &str) -> Result<Self> {
        if path.exists() {
            Self::load(path)
        } else {
            Ok(Self::new(sync_folder, machine))
        }
    }
}
