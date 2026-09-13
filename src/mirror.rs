use anyhow::Result;
use libsql::{Connection, Row, Value};

use crate::config::Config;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS chats (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  username TEXT,
  unread INTEGER NOT NULL DEFAULT 0,
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
CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(text, content='messages', content_rowid='rowid');
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

pub struct SearchResult {
    pub date: i64,
    pub out: i32,
    pub sender: String,
    pub text: String,
    pub chat_name: String,
}

fn text(value: Option<&str>) -> Value {
    match value {
        Some(v) => Value::Text(v.to_string()),
        None => Value::Null,
    }
}

fn int(value: Option<i64>) -> Value {
    match value {
        Some(v) => Value::Integer(v),
        None => Value::Null,
    }
}

pub struct Mirror {
    conn: Connection,
}

impl Mirror {
    pub async fn open() -> Result<Self> {
        let db = libsql::Builder::new_local(Config::mirror())
            .build()
            .await?;
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
    ) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO chats (id, name, username, unread, last_date, last)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET
                   name=excluded.name,
                   username=excluded.username,
                   unread=excluded.unread,
                   last_date=excluded.last_date,
                   last=excluded.last",
                libsql::params![id, name, text(username), unread, int(last_date), text(last)],
            )
            .await?;
        Ok(())
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
    ) -> Result<()> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO messages (chat, id, date, out, sender, text, media)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                libsql::params![chat, id, date, out, text(sender), body, text(media)],
            )
            .await?;
        Ok(())
    }

    pub async fn search(&self, q: &str, chat: Option<&str>) -> Result<Vec<SearchResult>> {
        let sql = match chat {
            Some(_) => {
                "SELECT m.date, m.out, m.sender, m.text, c.name
                 FROM messages_fts f
                 JOIN messages m ON f.rowid = m.rowid
                 JOIN chats c ON c.id = m.chat
                 WHERE messages_fts MATCH ?1 AND (c.id = ?2 OR c.name = ?2 OR c.username = ?2)
                 ORDER BY m.date DESC LIMIT 20"
            }
            None => {
                "SELECT m.date, m.out, m.sender, m.text, c.name
                 FROM messages_fts f
                 JOIN messages m ON f.rowid = m.rowid
                 JOIN chats c ON c.id = m.chat
                 WHERE messages_fts MATCH ?1
                 ORDER BY m.date DESC LIMIT 20"
            }
        };
        let mut values = vec![Value::Text(q.to_string())];
        if let Some(c) = chat {
            values.push(Value::Text(c.to_string()));
        }
        let mut rows = self
            .conn
            .query(sql, libsql::params_from_iter(values))
            .await?;
        let mut res = Vec::new();
        while let Some(row) = rows.next().await? {
            res.push(read(row));
        }
        Ok(res)
    }
}

fn read(row: Row) -> SearchResult {
    SearchResult {
        date: row.get(0).unwrap_or_default(),
        out: row.get(1).unwrap_or_default(),
        sender: row.get::<Option<String>>(2).unwrap_or_default().unwrap_or_default(),
        text: row.get::<String>(3).unwrap_or_default(),
        chat_name: row.get::<String>(4).unwrap_or_default(),
    }
}