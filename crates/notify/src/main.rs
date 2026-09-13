use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use termgram::{Client, Config, Label, Update};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;

#[derive(Default)]
pub struct Watch {
    pub on: bool,
    pub chats: HashSet<i64>,
}

const HEAD_CAP: usize = 64 * 1024;

fn parse_request(head: &[u8]) -> (String, HashMap<String, String>) {
    let text = String::from_utf8_lossy(head);
    let line = text.lines().next().unwrap_or("");
    let mut parts = line.split_whitespace();
    let path = parts.nth(1).unwrap_or("/").to_string();
    let (path, query) = path
        .split_once('?')
        .map_or((path.as_str(), ""), |(p, q)| (p, q));
    let mut params = HashMap::new();
    for pair in query.split('&').filter(|s| !s.is_empty()) {
        if let Some((k, v)) = pair.split_once('=') {
            params.insert(k.to_string(), v.to_string());
        }
    }
    (path.to_string(), params)
}

async fn read_head(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let mut buffered = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    loop {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            return Ok(buffered);
        }
        buffered.extend_from_slice(&chunk[..n]);
        if buffered.windows(4).any(|w| w == b"\r\n\r\n") {
            return Ok(buffered);
        }
        if buffered.len() > HEAD_CAP {
            anyhow::bail!("request head too large");
        }
    }
}

async fn serve_stream(
    mut stream: TcpStream,
    mut events: broadcast::Receiver<Update>,
    watch: Arc<Mutex<Watch>>,
) -> Result<()> {
    let head = read_head(&mut stream).await?;
    let (path, params) = parse_request(&head);
    if path != "/events" {
        stream
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n")
            .await?;
        return Ok(());
    }
    let chat_filter = params
        .get("chat")
        .and_then(|v| v.parse::<i64>().ok());
    stream
        .write_all(
            b"HTTP/1.1 200 OK\r\n\
              Content-Type: text/event-stream\r\n\
              Cache-Control: no-cache\r\n\
              Connection: keep-alive\r\n\
              Access-Control-Allow-Origin: *\r\n\r\n",
        )
        .await?;
    stream.flush().await?;
    let mut heartbeat = tokio::time::interval(Duration::from_secs(15));
    heartbeat.tick().await;
    loop {
        tokio::select! {
            update = events.recv() => {
                let Ok(Update::NewMessage(msg)) = update else {
                    continue;
                };
                let chat = msg.peer_id().bot_api_dialog_id().unwrap_or(0);
                {
                    let w = watch.lock().unwrap();
                    if !w.on || (!w.chats.is_empty() && !w.chats.contains(&chat)) {
                        continue;
                    }
                }
                if let Some(filter) = chat_filter {
                    if chat != filter {
                        continue;
                    }
                }
                let line = serde_json::json!({
                    "chat": chat,
                    "id": msg.id(),
                    "at": msg.date().timestamp(),
                    "out": msg.outgoing(),
                    "from": msg.sender().and_then(|s| s.name()),
                    "text": msg.text().to_string(),
                    "media": msg.media().as_ref().and_then(Label::kind),
                });
                if stream.write_all(format!("data: {line}\n\n").as_bytes()).await.is_err() {
                    return Ok(());
                }
                if stream.flush().await.is_err() {
                    return Ok(());
                }
            }
            _ = heartbeat.tick() => {
                if stream.write_all(b": ping\n\n").await.is_err() {
                    return Ok(());
                }
                if stream.flush().await.is_err() {
                    return Ok(());
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = Config::load()?;
    let client = Client::new(&cfg, "default").await?;
    if !client.raw.is_authorized().await.unwrap_or(false) {
        anyhow::bail!("not logged in - run `termgram login` in a terminal first");
    }
    eprintln!("termgram-notify serving");
    let port = std::env::var("TERMGRAM_NOTIFY_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(6837);
    let watch = Arc::new(Mutex::new(Watch {
        on: true,
        chats: HashSet::new(),
    }));
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    eprintln!("SSE stream on http://127.0.0.1:{port}/events");
    loop {
        let (stream, _) = listener.accept().await?;
        let events = client.signals();
        let watch = watch.clone();
        tokio::spawn(async move {
            let _ = serve_stream(stream, events, watch).await;
        });
    }
}