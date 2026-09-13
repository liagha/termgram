use anyhow::Result;
use grammers_client::tl;

use crate::Topic;

impl crate::Client {
    pub async fn topics(&self, target: &str) -> Result<Vec<Topic>> {
        let peer = self.resolve(target).await?;
        let res = self
            .raw
            .invoke(&tl::functions::messages::GetForumTopics {
                peer: peer.into(),
                q: None,
                offset_date: 0,
                offset_id: 0,
                offset_topic: 0,
                limit: 100,
            })
            .await?;
        let mut rows = vec![];
        match res {
            tl::enums::messages::ForumTopics::Topics(page) => {
                for raw in page.topics {
                    if let tl::enums::ForumTopic::Topic(topic) = raw {
                        rows.push(Topic {
                            id: topic.id,
                            title: topic.title,
                        });
                    }
                }
            }
        }
        Ok(rows)
    }
}