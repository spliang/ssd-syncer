use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub machine: MachineConfig,
    #[serde(default)]
    pub sync: Vec<SyncMapping>,
    #[serde(default)]
    pub ignore: IgnoreConfig,
    #[serde(default)]
    pub conflict: ConflictConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineConfig {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMapping {
    pub local: String,
    pub ssd: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoreConfig {
    #[serde(default = "default_ignore_patterns")]
    pub patterns: Vec<String>,
}

impl Default for IgnoreConfig {
    fn default() -> Self {
        Self {
            patterns: default_ignore_patterns(),
        }
    }
}

fn default_ignore_patterns() -> Vec<String> {
    vec![
        ".DS_Store".to_string(),
        "Thumbs.db".to_string(),
        "desktop.ini".to_string(),
        ".ssd-syncer".to_string(),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictConfig {
    #[serde(default = "default_conflict_strategy")]
    pub strategy: ConflictStrategy,
}

impl Default for ConflictConfig {
    fn default() -> Self {
        Self {
            strategy: default_conflict_strategy(),
        }
    }
}

fn default_conflict_strategy() -> ConflictStrategy {
    ConflictStrategy::Both
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ConflictStrategy {
    Both,
    LocalWins,
    SsdWins,
    NewerWins,
    Ask,
}

impl AppConfig {
    pub fn config_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Cannot determine home directory")?;
        Ok(home.join(".ssd-syncer"))
    }

    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            anyhow::bail!(
                "Config not found at {}. Run `ssd-syncer init` first.",
                path.display()
            );
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config: {}", path.display()))?;
        let config: AppConfig =
            toml::from_str(&content).with_context(|| "Failed to parse config")?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn create_new(machine_name: &str) -> Result<Self> {
        let config = AppConfig {
            machine: MachineConfig {
                name: machine_name.to_string(),
            },
            sync: vec![],
            ignore: IgnoreConfig::default(),
            conflict: ConflictConfig::default(),
        };
        config.save()?;
        Ok(config)
    }

    pub fn find_mapping_by_ssd(&self, ssd_rel: &str) -> Option<&SyncMapping> {
        self.sync.iter().find(|m| m.ssd == ssd_rel)
    }

    pub fn ssd_syncer_dir(ssd_mount: &Path) -> PathBuf {
        ssd_mount.join(".ssd-syncer")
    }

    pub fn ssd_snapshots_dir(ssd_mount: &Path, machine_name: &str) -> PathBuf {
        Self::ssd_syncer_dir(ssd_mount)
            .join("snapshots")
            .join(machine_name)
    }
}
