use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    #[serde(skip)]
    pub name: String,
    pub remote_host: String,
    pub remote_pool: String,
    pub local_pool: String,
    #[serde(default)]
    pub keep_snapshots: Vec<KeepPolicy>,
    #[serde(default)]
    pub skip_filesystems: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeepPolicy {
    pub count: u32,
    pub label: String,
}

pub fn load_pool_configs(config_dir: &Path) -> Result<Vec<PoolConfig>> {
    let mut configs = Vec::new();
    if !config_dir.exists() {
        return Ok(configs);
    }

    for entry in std::fs::read_dir(config_dir).context("Failed to read config directory")? {
        let path = entry.context("Failed to read directory entry")?.path();
        if !path.is_file() {
            continue;
        }
        match path.extension() {
            Some(ext) if ext == "pool" => {}
            _ => continue,
        }

        debug!("Loading config from: {:?}", path);
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;
        let mut config: PoolConfig = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {:?}", path))?;

        if let Some(filename) = path.file_stem() {
            config.name = filename.to_string_lossy().to_string();
        }

        configs.push(config);
    }

    Ok(configs)
}
