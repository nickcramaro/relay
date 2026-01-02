use super::Config;
use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::path::PathBuf;

pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn new() -> Result<Self> {
        let dirs = ProjectDirs::from("", "", "relay")
            .context("Could not determine config directory")?;
        let config_dir = dirs.config_dir();
        std::fs::create_dir_all(config_dir)
            .with_context(|| format!("Failed to create config directory: {:?}", config_dir))?;
        Ok(Self {
            path: config_dir.join("config.yaml"),
        })
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Result<Config> {
        if !self.path.exists() {
            return Ok(Config::default());
        }
        let contents = std::fs::read_to_string(&self.path)
            .with_context(|| format!("Failed to read config from {:?}", self.path))?;
        let config: Config = serde_yaml::from_str(&contents)
            .with_context(|| "Failed to parse config YAML")?;
        Ok(config)
    }

    pub fn save(&self, config: &Config) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent directory: {:?}", parent))?;
        }
        let contents = serde_yaml::to_string(config)
            .context("Failed to serialize config to YAML")?;
        std::fs::write(&self.path, contents)
            .with_context(|| format!("Failed to write config to {:?}", self.path))?;
        Ok(())
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ServerConfig, TransportConfig};
    use tempfile::tempdir;

    #[test]
    fn test_config_persistence() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        let store = ConfigStore::with_path(path);

        // Load from non-existent file returns default
        let config = store.load().unwrap();
        assert!(config.servers.is_empty());

        // Save and reload
        let mut config = Config::default();
        config.servers.insert(
            "test".to_string(),
            ServerConfig {
                transport: TransportConfig::Http {
                    url: "http://localhost:3000".to_string(),
                },
                env: Default::default(),
            },
        );
        store.save(&config).unwrap();

        let loaded = store.load().unwrap();
        assert!(loaded.servers.contains_key("test"));
    }
}
