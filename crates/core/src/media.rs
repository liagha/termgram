use std::time::Duration;

use anyhow::{bail, Result};
use grammers_client::media::{Attribute, InputMedia};
use grammers_client::message::InputMessage;

use crate::{Done, Sent};

impl crate::Client {
    pub async fn photo(&self, target: &str, path: &str, caption: Option<&str>) -> Result<Sent> {
        let peer = self.resolve(target).await?;
        let file = self.raw.upload_file(path).await?;
        let mut msg = InputMessage::new().photo(file);
        if let Some(caption) = caption {
            msg = msg.text(caption);
        }
        let sent = self.raw.send_message(peer, msg).await?;
        Ok(Sent { id: sent.id() })
    }

    pub async fn album(&self, target: &str, paths: &[String], caption: Option<&str>) -> Result<Done> {
        let peer = self.resolve(target).await?;
        if paths.len() < 2 {
            bail!("album needs at least 2 files");
        }
        let mut media = vec![];
        for path in paths {
            let file = self.raw.upload_file(path).await?;
            let image = std::path::Path::new(path)
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| {
                    matches!(ext, "jpg" | "jpeg" | "png" | "webp" | "gif")
                });
            media.push(if image {
                InputMedia::new().photo(file)
            } else {
                InputMedia::new().document(file)
            });
        }
        if let Some(caption) = caption {
            if let Some(last) = media.last_mut() {
                *last = std::mem::take(last).caption(caption);
            }
        }
        let sent = self.raw.send_album(peer, media).await?;
        Ok(Done { n: sent.len() })
    }

    pub async fn voice(&self, target: &str, path: &str) -> Result<Sent> {
        let peer = self.resolve(target).await?;
        let file = self.raw.upload_file(path).await?;
        let msg = InputMessage::new()
            .document(file)
            .attribute(Attribute::Voice {
                duration: Duration::ZERO,
                waveform: None,
            });
        let sent = self.raw.send_message(peer, msg).await?;
        Ok(Sent { id: sent.id() })
    }

    pub async fn grab(&self, target: &str) -> Result<Done> {
        let peer = self.resolve(target).await?;
        let mut iter = self.raw.iter_messages(peer).limit(5000);
        let mut n = 0;
        while let Some(m) = iter.next().await? {
            if let Some(media) = m.media() {
                self.save_media(target, m.id(), &media).await?;
                n += 1;
            }
        }
        Ok(Done { n })
    }
}