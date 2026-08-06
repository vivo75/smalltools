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

    #[serde(default = "default_compress")]
    pub compress: String,

    #[serde(default = "default_compress_flags")]
    pub compress_flags: String,

    #[serde(default = "default_uncompress_flags")]
    pub uncompress_flags: String,

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

fn default_compress() -> String {
    "lz4".to_string()
}

fn default_compress_flags() -> String {
    String::new()
}

fn default_uncompress_flags() -> String {
    "-dcfm".to_string()
}

pub fn load_pool_configs(config_dir: &Path) -> Result<Vec<PoolConfig>> {
    let mut configs = Vec::new();

    if !config_dir.exists() {
        return Ok(configs);
    }

    for entry in std::fs::read_dir(config_dir).context("Failed to read config directory")? {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        if let Some(extension) = path.extension() {
            if extension != "pool" {
                continue;
            }
        } else {
            continue;
        }

        debug!("Loading config from: {:?}", path);
        let mut config = load_pool_config(&path)?;
        
        // Use filename (without extension) as pool name
        if let Some(filename) = path.file_stem() {
            config.name = filename.to_string_lossy().to_string();
        }

        configs.push(config);
    }

    Ok(configs)
}

fn load_pool_config(path: &Path) -> Result<PoolConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {:?}", path))?;

    let config: PoolConfig = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {:?}", path))?;

    validate_pool_config(&config)?;

    Ok(config)
}

fn validate_pool_config(config: &PoolConfig) -> Result<()> {
    if config.remote_host.is_empty() {
        anyhow::bail!("remote_host is required");
    }
    if config.remote_pool.is_empty() {
        anyhow::bail!("remote_pool is required");
    }
    if config.local_pool.is_empty() {
        anyhow::bail!("local_pool is required");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parsing() {
        let yaml = r#"
remote_host: backup.example.com
remote_pool: tank
local_pool: backup/tank
compress: lz4
keep_snapshots:
  - count: 7
    label: daily
  - count: 4
    label: weekly
        "#;

        let config: PoolConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.remote_host, "backup.example.com");
        assert_eq!(config.remote_pool, "tank");
        assert_eq!(config.local_pool, "backup/tank");
        assert_eq!(config.keep_snapshots.len(), 2);
    }
}
