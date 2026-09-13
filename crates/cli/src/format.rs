use chrono::DateTime;
use termgram::Line;

pub struct Out;

impl Out {
    pub fn time(at: i64) -> String {
        DateTime::from_timestamp(at, 0)
            .map(|date| date.with_timezone(&chrono::Local).format("%H:%M").to_string())
            .unwrap_or_default()
    }

    pub fn date(at: i64) -> String {
        DateTime::from_timestamp(at, 0)
            .map(|date| {
                date.with_timezone(&chrono::Local)
                    .format("%Y-%m-%d %H:%M")
                    .to_string()
            })
            .unwrap_or_default()
    }

    pub fn preview(text: &str) -> String {
        let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let mut chars = flat.chars();
        if chars.clone().count() > 60 {
            format!("{}...", chars.by_ref().take(60).collect::<String>())
        } else {
            flat
        }
    }

    pub fn who(line: &Line) -> String {
        if line.out {
            "you".to_string()
        } else {
            line.who.as_deref().unwrap_or("?").to_string()
        }
    }
}