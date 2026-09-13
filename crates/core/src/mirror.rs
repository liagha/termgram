use std::path::Path;

use anyhow::Result;
use libsql::Value;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS chats (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    username TEXT,
    unread INTEGER DEFAULT 0,
    last_date INTEGER,
    last TEXT
);
CREATE TABLE IF NOT EXISTS messages (
    rowid INTEGER PRIMARY KEY AUTOINCREMENT,
    chat INTEGER NOT NULL,
    id INTEGER NOT NULL,
    date INTEGER NOT NULL,
    out INTEGER NOT NULL,
    sender TEXT,
    text TEXT,
    media TEXT,
    UNIQUE(chat, id)
);
CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    text,
    content='messages',
    content_rowid='rowid'
);
CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, text) VALUES (new.rowid, new.text);
END;
CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, text) VALUES ('delete', old.rowid, old.text);
END;
CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, text) VALUES ('delete', old.rowid, old.text);
    INSERT INTO messages_fts(rowid, text) VALUES (new.rowid, new.text);
END;
";

#[derive(Debug, serde::Serialize)]
pub struct Row {
    pub id: i32,
    pub at: i64,
    pub out: i32,
    pub who: Option<String>,
    pub text: String,
    pub chat: String,
}

fn text(value: Option<&str>) -> Value {
    match value {
        Some(text) => Value::Text(text.to_string()),
        None => Value::Null,
    }
}

fn int(value: Option<i64>) -> Value {
    match value {
        Some(i) => Value::Integer(i),
        None => Value::Null,
    }
}

pub struct Mirror {
    pub conn: libsql::Connection,
}

impl Mirror {
    pub async fn open(path: &Path) -> Result<Self> {
        let db = libsql::Builder::new_local(path).build().await?;
        let conn = db.connect()?;
        conn.execute_batch("PRAGMA journal_mode=WAL;").await?;
        conn.execute_batch(SCHEMA).await?;
        Ok(Self { conn })
    }

    pub async fn upsert_chat(
        &self,
        id: i64,
        name: &str,
        username: Option<&str>,
        unread: i32,
        last_date: Option<i64>,
        last: Option<&str>,
    ) {
        let _ = self
            .conn
            .execute(
                "INSERT INTO chats(id, name, username, unread, last_date, last)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET
                     name = excluded.name,
                     username = excluded.username,
                     unread = excluded.unread,
                     last_date = excluded.last_date,
                     last = excluded.last",
                libsql::params![id, name, username, unread, int(last_date), text(last)],
            )
            .await;
    }

    pub async fn insert_message(
        &self,
        chat: i64,
        id: i32,
        date: i64,
        out: i32,
        sender: Option<&str>,
        body: &str,
        media: Option<&str>,
    ) {
        let _ = self
            .conn
            .execute(
                "INSERT OR IGNORE INTO messages(chat, id, date, out, sender, text, media)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                libsql::params![chat, id, date, out, text(sender), body, text(media)],
            )
            .await;
    }

    pub async fn chat_name(&self, id: i64) -> Option<String> {
        let mut rows = self
            .conn
            .query("SELECT name FROM chats WHERE id = ?1", libsql::params![id])
            .await
            .ok()?;
        rows.next().await.ok().flatten()?.get::<String>(0).ok()
    }

    pub async fn count(&self, chat: i64) -> Result<i64> {
        let mut rows = self
            .conn
            .query(
                "SELECT COUNT(*) FROM messages WHERE chat = ?1",
                libsql::params![chat],
            )
            .await?;
        Ok(rows
            .next()
            .await?
            .map(|row| row.get::<i64>(0).unwrap_or_default())
            .unwrap_or_default())
    }

    pub async fn lines(&self, chat: &str, limit: usize) -> Result<Vec<Row>> {
        let mut rows = self
            .conn
            .query(
                "SELECT m.date, m.out, m.sender, m.text, c.name, m.id
                 FROM messages m
                 JOIN chats c ON c.id = m.chat
                 WHERE c.id = ?1 OR LOWER(c.name) = LOWER(?1) OR LOWER(c.username) = LOWER(?1)
                 ORDER BY m.date DESC
                 LIMIT ?2",
                libsql::params![chat, limit as i64],
            )
            .await?;
        let mut out = vec![];
        while let Some(row) = rows.next().await? {
            out.push(Row {
                id: row.get(5).unwrap_or_default(),
                at: row.get(0).unwrap_or_default(),
                out: row.get(1).unwrap_or_default(),
                who: row.get(2).unwrap_or_default(),
                text: row.get(3).unwrap_or_default(),
                chat: row.get(4).unwrap_or_default(),
            });
        }
        Ok(out)
    }

    pub async fn search(&self, q: &str, chat: Option<&str>) -> Result<Vec<Row>> {
        let (sql, values) = match chat {
            Some(slot) => (
                "SELECT m.date, m.out, m.sender, m.text, c.name, m.id
                 FROM messages_fts f
                 JOIN messages m ON f.rowid = m.rowid
                 JOIN chats c ON c.id = m.chat
                 WHERE messages_fts MATCH ?1 AND (c.id = ?2 OR LOWER(c.name) = LOWER(?2) OR LOWER(c.username) = LOWER(?2))
                 ORDER BY m.date DESC
                 LIMIT 20",
                vec![Value::Text(q.to_string()), Value::Text(slot.to_string())],
            ),
            None => (
                "SELECT m.date, m.out, m.sender, m.text, c.name, m.id
                 FROM messages_fts f
                 JOIN messages m ON f.rowid = m.rowid
                 JOIN chats c ON c.id = m.chat
                 WHERE messages_fts MATCH ?1
                 ORDER BY m.date DESC
                 LIMIT 20",
                vec![Value::Text(q.to_string())],
            ),
        };
        let mut rows = self.conn.query(sql, libsql::params_from_iter(values)).await?;
        let mut out = vec![];
        while let Some(row) = rows.next().await? {
            out.push(Row {
                id: row.get(5).unwrap_or_default(),
                at: row.get(0).unwrap_or_default(),
                out: row.get(1).unwrap_or_default(),
                who: row.get(2).unwrap_or_default(),
                text: row.get(3).unwrap_or_default(),
                chat: row.get(4).unwrap_or_default(),
            });
        }
        Ok(out)
    }
}