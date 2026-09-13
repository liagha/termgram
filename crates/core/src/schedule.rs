use grammers_client::tl;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use anyhow::Result;
use grammers_client::message::InputMessage;

use crate::{Ack, Planned, Sent};

#[derive(Serialize, Deserialize, Default)]
struct Queue(HashMap<i64, Vec<i32>>);

impl crate::Client {
    pub async fn schedule(&self, target: &str, text: &str, at: u64) -> Result<Sent> {
        let peer = self.resolve(target).await?;
        let when = SystemTime::UNIX_EPOCH + Duration::from_secs(at);
        let msg = InputMessage::new().text(text).schedule_date(Some(when));
        let sent = self.raw.send_message(peer.clone(), msg).await?;
        let id = sent.id();
        if let Some(peer) = peer.id.bot_api_dialog_id() {
            let mut queue = self.read_queue();
            queue.0.entry(peer).or_default().push(id);
            let _ = self.write_queue(&queue);
        }
        Ok(Sent { id })
    }

    pub async fn scheduled(&self, target: &str) -> Result<Vec<Planned>> {
        let peer = self.resolve(target).await?;
        let input = peer.clone().into();
        let mut rows = vec![];
        let mut keep = vec![];
        if let Some(peer) = peer.id.bot_api_dialog_id() {
            for id in self.read_queue().0.remove(&peer).unwrap_or_default() {
                self.collect(&input, id, &mut rows, &mut keep).await;
            }
            let extra = self.collect_all(&input).await;
            for row in extra {
                if !rows.iter().any(|r| r.id == row.id) {
                    rows.push(row);
                }
            }
            if !keep.is_empty() {
                let mut queue = self.read_queue();
                queue.0.entry(peer).or_default().extend(keep);
                let _ = self.write_queue(&queue);
            }
        }
        rows.sort_by_key(|r| r.at);
        Ok(rows)
    }

    pub async fn cancel(&self, target: &str, id: i32) -> Result<Ack> {
        let peer = self.resolve(target).await?;
        self.raw
            .invoke(&tl::functions::messages::DeleteScheduledMessages {
                peer: peer.into(),
                id: vec![id],
            })
            .await?;
        if let Some(peer) = peer.id.bot_api_dialog_id() {
            let mut queue = self.read_queue();
            if let Some(ids) = queue.0.get_mut(&peer) {
                ids.retain(|x| *x != id);
            }
            let _ = self.write_queue(&queue);
        }
        Ok(Ack {
            text: format!("cancelled {id}"),
        })
    }

    async fn collect(&self, input: &tl::enums::InputPeer, id: i32, rows: &mut Vec<Planned>, keep: &mut Vec<i32>) {
        let Ok(res) = self
            .raw
            .invoke(&tl::functions::messages::GetScheduledMessages {
                peer: input.clone(),
                id: vec![id],
            })
            .await
        else {
            return;
        };
        for raw in matches(res) {
            if let tl::enums::Message::Message(msg) = raw {
                if msg.id == id {
                    keep.push(id);
                    rows.push(Planned {
                        id: msg.id,
                        at: msg.date as i64,
                        text: msg.message,
                    });
                }
            }
        }
    }

    async fn collect_all(&self, input: &tl::enums::InputPeer) -> Vec<Planned> {
        let Ok(res) = self
            .raw
            .invoke(&tl::functions::messages::GetScheduledMessages {
                peer: input.clone(),
                id: vec![],
            })
            .await
        else {
            return vec![];
        };
        let mut rows = vec![];
        for raw in matches(res) {
            if let tl::enums::Message::Message(msg) = raw {
                rows.push(Planned {
                    id: msg.id,
                    at: msg.date as i64,
                    text: msg.message,
                });
            }
        }
        rows
    }

    fn queue(&self) -> PathBuf {
        if self.account == "default" {
            crate::config::Config::dir().join("scheduled.json")
        } else {
            crate::config::Config::under(&self.account).join("scheduled.json")
        }
    }

    fn read_queue(&self) -> Queue {
        std::fs::read_to_string(self.queue())
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    fn write_queue(&self, queue: &Queue) -> Option<()> {
        std::fs::write(self.queue(), serde_json::to_string_pretty(queue).ok()?).ok()
    }
}

fn matches(res: tl::enums::messages::Messages) -> Vec<tl::enums::Message> {
    match res {
        tl::enums::messages::Messages::Messages(page) => page.messages,
        tl::enums::messages::Messages::ChannelMessages(page) => page.messages,
        _ => vec![],
    }
}