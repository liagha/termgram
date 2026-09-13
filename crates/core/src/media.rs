use anyhow::Result;

use crate::Done;

impl crate::Client {
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