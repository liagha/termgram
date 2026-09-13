use grammers_client::tl;
use anyhow::Result;

use crate::{Ack, Draft, Text};

impl crate::Client {
    pub async fn draft(&self, target: &str, body: Option<&Text>) -> Result<Ack> {
        let peer = self.resolve(target).await?;
        let (message, entities) = match body {
            Some(text) => {
                let (raw, ents) = self.rich(text).await?;
                (raw, Some(ents))
            }
            None => (String::new(), None),
        };
        self.raw
            .invoke(&tl::functions::messages::SaveDraft {
                no_webpage: false,
                invert_media: false,
                reply_to: None,
                peer: peer.into(),
                message,
                entities,
                media: None,
                effect: None,
                suggested_post: None,
                rich_message: None,
            })
            .await?;
        Ok(Ack {
            text: if body.is_some() {
                "draft saved"
            } else {
                "draft cleared"
            }
            .into(),
        })
    }

    pub async fn drafts(&self) -> Result<Vec<Draft>> {
        let res = self
            .raw
            .invoke(&tl::functions::messages::GetAllDrafts {})
            .await?;
        let tl::enums::Updates::Updates(page) = res else {
            return Ok(vec![]);
        };
        let mut seen = vec![];
        let mut rows = vec![];
        for raw in page.updates {
            if let tl::enums::Update::DraftMessage(dm) = raw {
                let id = match dm.peer {
                    tl::enums::Peer::User(user) => user.user_id,
                    tl::enums::Peer::Chat(chat) => chat.chat_id,
                    tl::enums::Peer::Channel(channel) => channel.channel_id,
                };
                let tl::enums::DraftMessage::Message(draft) = dm.draft else {
                    continue;
                };
                if draft.message.is_empty() || seen.contains(&id) {
                    continue;
                }
                seen.push(id);
                let name = self.chat_name(id).await;
                rows.push(Draft {
                    chat: id,
                    name,
                    text: draft.message,
                });
            }
        }
        Ok(rows)
    }
}