use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Result};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub api_id: i32,
    pub api_hash: String,
}

impl Config {
    pub fn dir() -> PathBuf {
        dirs::config_dir().unwrap().join("termgram")
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

    pub fn under(account: &str) -> PathBuf {
        Self::dir().join(account)
    }

    pub fn ensure() -> Result<()> {
        fs::create_dir_all(Self::dir())?;
        let file = Self::file();
        if !file.exists() {
            fs::write(&file, "api_id = 0\napi_hash = \"\"\n")?;
        }
        Ok(())
    }

    pub fn load() -> Result<Self> {
        Self::ensure()?;
        let raw = fs::read_to_string(Self::file())?;
        let config: Config = toml::from_str(&raw)?;
        if config.api_id == 0 || config.api_hash.is_empty() {
            bail!("set api_id and api_hash in {}", Self::file().display());
        }
        Ok(config)
    }
}