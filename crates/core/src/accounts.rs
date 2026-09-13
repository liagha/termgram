use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::Ack;
use anyhow::Result;

impl crate::Client {
    pub fn session(&self) -> PathBuf {
        if self.account == "default" {
            Config::session()
        } else {
            Config::under(&self.account).join("session.db")
        }
    }

    pub fn mirror(&self) -> PathBuf {
        if self.account == "default" {
            Config::mirror()
        } else {
            Config::under(&self.account).join("mirror.db")
        }
    }

    pub fn media(&self) -> PathBuf {
        if self.account == "default" {
            Config::media()
        } else {
            Config::under(&self.account).join("media")
        }
    }

    pub async fn chat_name(&self, id: i64) -> Option<String> {
        crate::Mirror::open(&self.mirror())
            .await
            .ok()?
            .chat_name(id)
            .await
    }

    pub async fn export(&self, dest: &str) -> Result<Ack> {
        let dest = Path::new(dest);
        std::fs::create_dir_all(dest)?;
        copy(&Config::file(), &dest.join("config.toml"))?;
        copy(&self.session(), &dest.join("session.db"))?;
        copy(&self.mirror(), &dest.join("mirror.db"))?;
        copy_dir(&self.media(), &dest.join("media"))?;
        Ok(Ack {
            text: format!("exported to {}", dest.display()),
        })
    }

    pub async fn import(&self, src: &str) -> Result<Ack> {
        let src = Path::new(src);
        for (name, dest) in [
            ("config.toml", Config::file()),
            ("session.db", self.session()),
            ("mirror.db", self.mirror()),
        ] {
            let file = src.join(name);
            if file.exists() {
                copy(&file, &dest)?;
            }
        }
        copy_dir(&src.join("media"), &self.media())?;
        Ok(Ack {
            text: format!("imported from {}", src.display()),
        })
    }

    pub async fn wipe(&self) -> Result<Ack> {
        let mut n = 0;
        for file in [self.session(), self.mirror()] {
            if file.exists() {
                std::fs::remove_file(file)?;
                n += 1;
            }
        }
        if self.media().exists() {
            std::fs::remove_dir_all(self.media())?;
        }
        Ok(Ack {
            text: format!("wiped {n} files"),
        })
    }
}

fn copy(from: &Path, to: &Path) -> Result<()> {
    if from.exists() {
        std::fs::copy(from, to)?;
    }
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    if from.exists() {
        std::fs::create_dir_all(to)?;
        for entry in std::fs::read_dir(from)? {
            let entry = entry?;
            let dest = to.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                copy_dir(&entry.path(), &dest)?;
            } else {
                copy(&entry.path(), &dest)?;
            }
        }
    }
    Ok(())
}