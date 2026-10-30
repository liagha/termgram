use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use rpassword;
use serde::Deserialize;
use webbrowser;

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
        let mut config: Config = toml::from_str(&raw)?;

        if let Ok(id) = env::var("TELEGRAM_API_ID") {
            config.api_id = id.parse().context("TELEGRAM_API_ID must be an integer")?;
        }
        if let Ok(hash) = env::var("TELEGRAM_API_HASH") {
            config.api_hash = hash;
        }

        if config.api_id == 0 || config.api_hash.is_empty() {
            return Self::prompt_setup();
        }

        Ok(config)
    }

    fn prompt_setup() -> Result<Self> {
        let _ = webbrowser::open("https://my.telegram.org/apps");

        println!(
            "api_id and api_hash are not set in {}.\n\
             Open https://my.telegram.org/apps in your browser, log in, and\n\
             create a new application under \"API development tools\".\n\
             Then paste the values below.",
            Self::file().display()
        );

        let api_id = loop {
            print!("api_id: ");
            io::stdout().flush()?;
            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(0) => bail!("no input received - set TELEGRAM_API_ID and TELEGRAM_API_HASH instead"),
                Ok(_) => match input.trim().parse::<i32>() {
                    Ok(id) if id != 0 => break id,
                    _ => println!("please enter a valid numeric api_id"),
                },
                Err(err) => bail!("failed to read api_id: {err}"),
            }
        };

        let api_hash = rpassword::prompt_password("api_hash: ")?;
        if api_hash.is_empty() {
            bail!("api_hash must not be empty");
        }

        let config = Config { api_id, api_hash };

        let toml = format!("api_id = {}\napi_hash = \"{}\"\n", config.api_id, config.api_hash);
        fs::write(Self::file(), &toml)?;
        println!("saved to {}", Self::file().display());

        Ok(config)
    }
}