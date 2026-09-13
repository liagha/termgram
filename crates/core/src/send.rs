use std::path::Path;
use std::time::{Duration, SystemTime};

use anyhow::{bail, Context, Result};
use chrono::{Datelike, TimeZone};
use grammers_client::media::{Attribute, InputMedia};
use grammers_client::message::InputMessage;
use grammers_client::parsers;
use grammers_client::tl;
use grammers_session::types::PeerRef;

use crate::{Ack, Send, Format, Sent, Text};

enum Kind {
    Photo,
    Document,
    Voice,
}

fn kind(path: &str) -> Kind {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "ogg" | "opus" | "oga" => Kind::Voice,
        "jpg" | "jpeg" | "png" | "webp" | "gif" => Kind::Photo,
        _ => Kind::Document,
    }
}

fn date_entity(text: &str, format_date: &str) -> Result<tl::types::MessageEntityFormattedDate> {
    let idx = text.find(format_date).with_context(|| {
        format!("format_date '{format_date}' was not found in the message text.")
    })?;
    let tokens: Vec<&str> = format_date.split_whitespace().collect();
    let parts: Vec<&str> = match tokens.first() {
        Some(t) => t.split('/').collect(),
        None => bail!("format_date must look like 13/09, 13/09/2026 or 13/09 17:00."),
    };
    if !(2..=3).contains(&parts.len())
        || parts.iter().any(|p| !p.chars().all(|c| c.is_ascii_digit()))
    {
        bail!("format_date '{format_date}' must look like 13/09, 13/09/2026 or 13/09 17:00.");
    }
    let clock: Option<Vec<&str>> = match tokens.len() {
        2 => {
            let c: Vec<&str> = tokens[1].split(':').collect();
            if c.len() != 2 || c.iter().any(|p| !p.chars().all(|c| c.is_ascii_digit())) {
                bail!("format_date '{format_date}' must look like 13/09, 13/09/2026 or 13/09 17:00.");
            }
            Some(c)
        }
        1 => None,
        _ => bail!("format_date '{format_date}' must look like 13/09, 13/09/2026 or 13/09 17:00."),
    };
    let day: u32 = parts[0]
        .parse()
        .with_context(|| format!("format_date '{format_date}' is not a valid date."))?;
    let month: u32 = parts[1]
        .parse()
        .with_context(|| format!("format_date '{format_date}' is not a valid date."))?;
    let year: i32 = if parts.len() == 3 {
        parts[2].parse().with_context(|| {
            format!("format_date '{format_date}' is not a valid date.")
        })?
    } else {
        chrono::Local::now().year()
    };
    let (hour, minute): (u32, u32) = match &clock {
        Some(c) => (c[0].parse()?, c[1].parse()?),
        None => (0, 0),
    };
    let naive = chrono::NaiveDate::from_ymd_opt(year, month, day)
        .and_then(|d| d.and_hms_opt(hour, minute, 0))
        .context(format!("format_date '{format_date}' is not a valid date."))?;
    let date = chrono::Local
        .from_local_datetime(&naive)
        .earliest()
        .context(format!("format_date '{format_date}' is not a valid date."))?
        .timestamp() as i32;
    Ok(tl::types::MessageEntityFormattedDate {
        relative: false,
        short_time: clock.is_some(),
        long_time: false,
        short_date: parts.len() == 2,
        long_date: parts.len() == 3,
        day_of_week: false,
        offset: text[..idx].encode_utf16().count() as i32,
        length: format_date.encode_utf16().count() as i32,
        date,
    })
}

impl crate::Client {
    pub async fn send(&self, args: &Send) -> Result<Sent> {
        let peer = self.resolve(&args.target).await?;
        let rich = match &args.text {
            Some(text) => Some(self.rich(text).await?),
            None => None,
        };
        let ids = if args.files.is_empty() {
            let Some((raw, entities)) = rich else {
                bail!("nothing to send: give a text or a file")
            };
            let msg = InputMessage::new().text(raw).fmt_entities(entities).reply_to(args.reply);
            vec![self.send_one(peer, msg, args.at).await?]
        } else if args.files.len() == 1 {
            let mut msg = self.single(&args.files[0]).await?;
            if let Some((caption, entities)) = rich {
                msg = msg.text(caption).fmt_entities(entities);
            }
            vec![self.send_one(peer, msg, args.at).await?]
        } else {
            if args.at.is_some() {
                bail!("can't schedule an album");
            }
            if let Some(text) = &args.text {
                if text.format != Format::Plain || !text.dates.is_empty() {
                    bail!("album captions can only be plain text");
                }
            }
            let mut media = vec![];
            for path in &args.files {
                let file = self.raw.upload_file(path).await?;
                media.push(match kind(path) {
                    Kind::Photo => InputMedia::new().photo(file),
                    Kind::Document | Kind::Voice => InputMedia::new().document(file),
                });
            }
            if let Some((caption, _)) = rich {
                if let Some(last) = media.last_mut() {
                    *last = std::mem::take(last).caption(caption);
                }
            }
            self.raw
                .send_album(peer, media)
                .await?
                .into_iter()
                .flatten()
                .map(|m| m.id())
                .collect()
        };
        Ok(Sent { ids })
    }

    pub(crate) async fn rich(
        &self,
        text: &Text,
    ) -> Result<(String, Vec<tl::enums::MessageEntity>)> {
        let (raw, mut entities) = match text.format {
            Format::Markdown => parsers::parse_markdown_message(&text.text),
            Format::Html => parsers::parse_html_message(&text.text),
            Format::Plain => (text.text.clone(), vec![]),
        };
        for date in &text.dates {
            entities.push(tl::enums::MessageEntity::FormattedDate(date_entity(
                &raw,
                date,
            )?));
        }
        Ok((raw, entities))
    }

    pub async fn edit(&self, target: &str, id: i32, text: &Text) -> Result<Ack> {
        let peer = self.resolve(target).await?;
        let (raw, entities) = self.rich(text).await?;
        self.raw
            .edit_message(peer, id, InputMessage::new().text(raw).fmt_entities(entities))
            .await?;
        Ok(Ack {
            text: format!("edited {id}"),
        })
    }

    async fn single(&self, path: &str) -> Result<InputMessage> {
        let file = self.raw.upload_file(path).await?;
        Ok(match kind(path) {
            Kind::Photo => InputMessage::new().photo(file),
            Kind::Document => InputMessage::new().document(file),
            Kind::Voice => InputMessage::new().document(file).attribute(Attribute::Voice {
                duration: Duration::ZERO,
                waveform: None,
            }),
        })
    }

    pub(crate) async fn dialog_key(&self, peer: &PeerRef) -> Option<i64> {
        if let Some(id) = peer.id.bot_api_dialog_id() {
            return Some(id);
        }
        self.raw
            .get_me()
            .await
            .ok()
            .and_then(|me| me.id().bot_api_dialog_id())
    }

    async fn send_one(&self, peer: PeerRef, msg: InputMessage, at: Option<u64>) -> Result<i32> {
        let msg = match at {
            Some(at) => msg.schedule_date(Some(SystemTime::UNIX_EPOCH + Duration::from_secs(at))),
            None => msg,
        };
        let sent = self.raw.send_message(peer.clone(), msg).await?;
        let id = sent.id();
        if at.is_some() {
            if let Some(key) = self.dialog_key(&peer).await {
                let mut queue = self.read_queue();
                queue.0.entry(key).or_default().push(id);
                let _ = self.write_queue(&queue);
            }
        }
        Ok(id)
    }
}