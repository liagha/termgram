use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use grammers_client::tl;

use crate::Sent;

impl crate::Client {
    pub async fn poll(
        &self,
        target: &str,
        question: &str,
        options: &[String],
        anon: bool,
        multi: bool,
    ) -> Result<Sent> {
        if !(2..=10).contains(&options.len()) {
            bail!("a poll needs 2 to 10 options, got {}", options.len());
        }
        let peer = self.resolve(target).await?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as i64)
            .unwrap_or(0);
        let answers = options
            .iter()
            .enumerate()
            .map(|(i, option)| {
                tl::enums::PollAnswer::Answer(tl::types::PollAnswer {
                    text: tl::enums::TextWithEntities::Entities(tl::types::TextWithEntities {
                        text: option.clone(),
                        entities: vec![],
                    }),
                    option: i.to_le_bytes().to_vec(),
                    media: None,
                    added_by: None,
                    date: None,
                })
            })
            .collect();
        let media = tl::types::InputMediaPoll {
            poll: tl::enums::Poll::Poll(tl::types::Poll {
                id: now,
                closed: false,
                public_voters: !anon,
                multiple_choice: multi,
                quiz: false,
                open_answers: false,
                revoting_disabled: false,
                shuffle_answers: false,
                hide_results_until_close: false,
                creator: false,
                subscribers_only: false,
                question: tl::enums::TextWithEntities::Entities(tl::types::TextWithEntities {
                    text: question.to_string(),
                    entities: vec![],
                }),
                answers,
                close_period: None,
                close_date: None,
                countries_iso2: None,
                hash: 0,
            }),
            correct_answers: None,
            attached_media: None,
            solution: None,
            solution_entities: None,
            solution_media: None,
        };
        let res = self
            .raw
            .invoke(&tl::functions::messages::SendMedia {
                silent: false,
                background: false,
                clear_draft: false,
                noforwards: false,
                update_stickersets_order: false,
                invert_media: false,
                allow_paid_floodskip: false,
                peer: peer.into(),
                reply_to: None,
                media: tl::enums::InputMedia::Poll(Box::new(media)),
                message: String::new(),
                random_id: now,
                reply_markup: None,
                entities: None,
                schedule_date: None,
                schedule_repeat_period: None,
                send_as: None,
                quick_reply_shortcut: None,
                effect: None,
                allow_paid_stars: None,
                suggested_post: None,
            })
            .await
            .context("couldn't send the poll")?;
        let id = sent_id(&res).context("couldn't read the sent message id")?;
        Ok(Sent { ids: vec![id] })
    }
}

pub(crate) fn sent_id(res: &tl::enums::Updates) -> Option<i32> {
    match res {
        tl::enums::Updates::UpdateShortSentMessage(update) => Some(update.id),
        tl::enums::Updates::Updates(page) => new_id(&page.updates),
        tl::enums::Updates::Combined(page) => new_id(&page.updates),
        _ => None,
    }
}

pub(crate) fn new_id(updates: &[tl::enums::Update]) -> Option<i32> {
    for update in updates {
        if let tl::enums::Update::NewMessage(update) = update
            && let tl::enums::Message::Message(msg) = &update.message
        {
            return Some(msg.id);
        }
    }
    None
}