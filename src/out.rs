use chrono::{DateTime, Local, Utc};
use grammers_client::media::Media;
use grammers_session::types::PeerId;

pub struct Out;

impl Out {
    pub fn media_kind(media: &Media) -> &'static str {
        match media {
            Media::Photo(_) => "photo",
            Media::Document(_) => "document",
            Media::Sticker(_) => "sticker",
            Media::Contact(_) => "contact",
            Media::Poll(_) => "poll",
            Media::Geo(_) => "geo",
            Media::Dice(_) => "dice",
            Media::Venue(_) => "venue",
            Media::GeoLive(_) => "geo_live",
            Media::WebPage(_) => "webpage",
            _ => "other",
        }
    }

    pub fn time(dt: &DateTime<Utc>) -> String {
        dt.with_timezone(&Local).format("%H:%M").to_string()
    }

    pub fn date(dt: &DateTime<Utc>) -> String {
        dt.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string()
    }

    pub fn preview(text: &str) -> String {
        let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let size = flat.chars().count();
        if size > 60 {
            let mut out = String::new();
            out.extend(flat.chars().take(57));
            out.push_str("...");
            out
        } else {
            flat
        }
    }

    pub fn dialog_id(id: PeerId) -> i64 {
        id.bot_api_dialog_id().unwrap_or(0)
    }

    pub fn ext(media: &Media) -> &'static str {
        match media {
            Media::Photo(_) => "jpg",
            Media::Sticker(_) => "webp",
            Media::Document(doc) => match doc.mime_type() {
                Some("application/pdf") => "pdf",
                Some("application/zip") => "zip",
                Some("application/x-tar") => "tar",
                Some("application/vnd.android.package-archive") => "apk",
                Some("text/plain") => "txt",
                Some("text/html") => "html",
                Some("image/jpeg") => "jpg",
                Some("image/png") => "png",
                Some("image/webp") => "webp",
                Some("image/gif") => "gif",
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