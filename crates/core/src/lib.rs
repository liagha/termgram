pub mod accounts;
pub mod client;
pub mod config;
pub mod draft;
pub mod media;
pub mod mirror;
pub mod poll;
pub mod presence;
pub mod profile;
pub mod schedule;
pub mod search;
pub mod send;
pub mod topics;

pub use client::Client;
pub use config::Config;
pub use grammers_client::update::Update;
pub use mirror::Mirror;

use grammers_client::media::Media;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct Label;

impl Label {
    pub fn kind(media: &Media) -> Option<&'static str> {
        match media {
            Media::Photo(_) => Some("photo"),
            Media::Document(_) => Some("document"),
            Media::Sticker(_) => Some("sticker"),
            Media::Contact(_) => Some("contact"),
            Media::Poll(_) => Some("poll"),
            Media::Geo(_) => Some("geo"),
            Media::Dice(_) => Some("dice"),
            Media::Venue(_) => Some("venue"),
            Media::GeoLive(_) => Some("geo_live"),
            Media::WebPage(_) => Some("webpage"),
            _ => None,
        }
    }

    pub fn ext(media: &Media) -> &'static str {
        match media {
            Media::Photo(_) => "jpg",
            Media::Sticker(_) => "webp",
            Media::Document(doc) => match doc.mime_type() {
                Some("application/pdf") => "pdf",
                Some("application/gzip") => "tar",
                Some("application/zip") => "zip",
                Some("application/vnd.android.package-archive") => "apk",
                Some("text/plain") => "txt",
                Some("text/html") => "html",
                Some("image/jpeg" | "image/png" | "image/webp" | "image/gif") => "img",
                Some("video/mp4") => "mp4",
                Some("video/x-matroska") => "mkv",
                Some("audio/ogg") => "ogg",
                Some("audio/mpeg") => "mp3",
                Some("audio/mp4") => "m4a",
                _ => "bin",
            },
            _ => "bin",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Identity {
    pub name: String,
    pub handle: Option<String>,
    pub id: i64,
}

#[derive(Debug, Serialize)]
pub struct Dialog {
    pub id: i64,
    pub name: String,
    pub unread: i32,
    pub out: bool,
    pub at: Option<i64>,
    pub last: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Line {
    pub id: i32,
    pub at: i64,
    pub out: bool,
    pub who: Option<String>,
    pub text: String,
    pub media: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Hit {
    pub chat: String,
    pub line: Line,
}

#[derive(Debug, Serialize)]
pub struct Contact {
    pub name: String,
    pub handle: Option<String>,
    pub id: i64,
}

#[derive(Debug, Serialize)]
pub struct Folder {
    pub id: i32,
    pub title: String,
}

#[derive(Debug, Serialize)]
pub struct Pinned {
    pub id: i32,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct Member {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct Topic {
    pub id: i32,
    pub title: String,
}

#[derive(Debug, Serialize)]
pub struct Reaction {
    pub emoji: String,
    pub count: i32,
}

#[derive(Debug, Serialize)]
pub struct Sent {
    pub ids: Vec<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    /// plain text, no formatting
    Plain,
    /// markdown formatting
    Markdown,
    /// html formatting
    Html,
}

impl Format {
    pub fn parse(s: Option<&str>) -> anyhow::Result<Self> {
        match s.unwrap_or("plain").to_ascii_lowercase().as_str() {
            "plain" => Ok(Format::Plain),
            "md" | "markdown" => Ok(Format::Markdown),
            "html" => Ok(Format::Html),
            other => anyhow::bail!("format must be plain, md, or html, got {other}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Text {
    /// message body, or media caption when files are set
    pub text: String,
    /// how to parse the message: plain, markdown, or html
    pub format: Format,
    /// exact date text in the message to render as tappable chips; repeat for more
    #[serde(default)]
    pub dates: Vec<String>,
}

impl Text {
    pub fn plain(text: impl Into<String>) -> Self {
        Text {
            text: text.into(),
            format: Format::Plain,
            dates: vec![],
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct Send {
    /// chat: @username, chat id, or contact name
    pub target: String,
    /// message body, or media caption when files are set
    pub text: Option<Text>,
    /// one file sends a photo, document, or voice note by type; several send an album
    #[serde(default)]
    pub files: Vec<String>,
    /// message id to reply to
    pub reply: Option<i32>,
    /// forum topic id to send into
    pub topic: Option<i32>,
    /// future unix timestamp to schedule the send
    pub at: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct Done {
    pub n: usize,
}

#[derive(Debug, Serialize)]
pub struct Path {
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct Ack {
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct Summary {
    pub chats: usize,
    pub lines: usize,
}

#[derive(Debug, Serialize)]
pub struct Status {
    pub who: String,
    pub state: String,
}

#[derive(Debug, Serialize)]
pub struct Profile {
    pub name: String,
    pub handle: Option<String>,
    pub id: i64,
    pub phone: Option<String>,
    pub about: Option<String>,
    pub state: String,
    pub blocked: bool,
}

#[derive(Debug, Serialize)]
pub struct Planned {
    pub id: i32,
    pub at: i64,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct Draft {
    pub chat: i64,
    pub name: Option<String>,
    pub text: String,
}