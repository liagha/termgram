use grammers_client::tl;
use anyhow::Result;
use grammers_session::types::PeerId;

use crate::{Hit, Line};

impl crate::Client {
    pub async fn searchall(&self, q: &str, limit: usize) -> Result<Vec<Hit>> {
        let res = self
            .raw
            .invoke(&tl::functions::messages::SearchGlobal {
                broadcasts_only: false,
                groups_only: false,
                users_only: false,
                folder_id: None,
                q: q.to_string(),
                filter: tl::enums::MessagesFilter::InputMessagesFilterEmpty,
                min_date: 0,
                max_date: 0,
                offset_rate: 0,
                offset_peer: PeerId::self_user().to_ambient_ref().into(),
                offset_id: 0,
                limit: limit as i32,
            })
            .await?;
        let messages = match res {
            tl::enums::messages::Messages::Messages(page) => page.messages,
            tl::enums::messages::Messages::ChannelMessages(page) => page.messages,
            _ => vec![],
        };
        let mut rows = vec![];
        for raw in messages {
            if let tl::enums::Message::Message(msg) = raw {
                let id = peer_id(&msg.peer_id);
                let name = self.chat_name(id).await;
                rows.push(Hit {
                    chat: name.unwrap_or_else(|| format!("id {id}")),
                    line: Line {
                        id: msg.id,
                        at: msg.date as i64,
                        out: msg.out,
                        who: None,
                        text: msg.message,
                        media: None,
                    },
                });
            }
        }
        Ok(rows)
    }

    pub async fn searchin(&self, target: &str, q: &str, limit: usize) -> Result<Vec<Line>> {
        let peer = self.resolve(target).await?;
        let res = self
            .raw
            .invoke(&tl::functions::messages::Search {
                peer: peer.into(),
                q: q.to_string(),
                from_id: None,
                saved_peer_id: None,
                saved_reaction: None,
                top_msg_id: None,
                filter: tl::enums::MessagesFilter::InputMessagesFilterEmpty,
                min_date: 0,
                max_date: 0,
                offset_id: 0,
                add_offset: 0,
                limit: limit as i32,
                max_id: 0,
                min_id: 0,
                hash: 0,
            })
            .await?;
        let messages = match res {
            tl::enums::messages::Messages::Messages(page) => page.messages,
            tl::enums::messages::Messages::ChannelMessages(page) => page.messages,
            _ => vec![],
        };
        let mut rows = vec![];
        for raw in messages {
            if let tl::enums::Message::Message(msg) = raw {
                rows.push(Line {
                    id: msg.id,
                    at: msg.date as i64,
                    out: msg.out,
                    who: None,
                    text: msg.message,
                    media: None,
                });
            }
        }
        Ok(rows)
    }
}

fn peer_id(peer: &tl::enums::Peer) -> i64 {
    match peer {
        tl::enums::Peer::User(user) => user.user_id,
        tl::enums::Peer::Chat(chat) => chat.chat_id,
        tl::enums::Peer::Channel(channel) => channel.channel_id,
    }
}