use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub api_id: i32,
    pub api_hash: String,
}

impl Config {
    pub fn dir() -> PathBuf {
        let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("termgram")
    }

    pub fn file() -> PathBuf {
        Self::dir().join("config.toml")
    }

    pub fn session() -> PathBuf {
        Self::dir().join("session.db")
    }

    pub fn mirror() -> PathBuf {
        Self::dir().join("mirror.db")
    }

    pub fn media() -> PathBuf {
        Self::dir().join("media")
    }

    pub fn ensure() -> Result<()> {
        let dir = Self::dir();
        fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let file = Self::file();
        if !file.exists() {
            fs::write(&file, "api_id = 0\napi_hash = \"\"\n")
                .with_context(|| format!("writing {}", file.display()))?;
        }
        Ok(())
    }

    pub fn load() -> Result<Self> {
        Self::ensure()?;
        let file = Self::file();
        let raw =
            fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
        let cfg: Config = toml::from_str(&raw)?;
        if cfg.api_id == 0 || cfg.api_hash.is_empty() {
            bail!("set api_id and api_hash in {}", file.display());
        }
        Ok(cfg)
    }
}