mod cli;
mod client;
mod config;
mod mirror;
mod out;

use anyhow::{bail, Result};
use chrono::DateTime;
use clap::Parser;
use grammers_client::tl;
use grammers_client::update::Update;

use crate::cli::{Cli, Op};
use crate::out::Out;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = config::Config::load()?;
    let client = client::Client::new(&cfg).await?;
    let mut app = App { client, api_hash: cfg.api_hash };
    app.run(cli.op).await
}

struct App {
    client: client::Client,
    api_hash: String,
}

impl App {
    async fn run(&mut self, op: Op) -> Result<()> {
        match op {
            Op::Login => self.client.login(&self.api_hash).await,
            Op::Logout => self.client.logout().await,
            Op::Me => self.client.me().await,
            Op::Dialogs => self.dialogs().await,
            Op::Messages { target, limit } => self.messages(&target, limit).await,
            Op::Send { target, text, reply } => self.client.send(&target, &text, reply).await,
            Op::Read { target } => self.client.mark_as_read(&target).await,
            Op::Sync { limit } => self.sync(limit).await,
            Op::Search { query, chat } => self.search(&query, chat).await,
            Op::Watch => self.watch().await,
            Op::Upload { target, path, caption } => {
                self.client.upload(&target, &path, caption.as_deref()).await
            }
            Op::Download { target, id } => self.client.download(&target, id).await,
            Op::Edit { target, id, text } => self.client.edit(&target, id, &text).await,
            Op::Delete { target, ids } => self.client.delete(&target, ids).await,
            Op::Forward { from, to, ids } => self.client.forward(&from, &to, ids).await,
            Op::Pin { target, id, remove } => self.client.pin(&target, id, remove).await,
            Op::React { target, id, emoji, remove } => match (emoji, remove) {
                (Some(emoji), false) => self.client.react(&target, id, Some(&emoji), false).await,
                (None, true) => self.client.react(&target, id, None, true).await,
                (Some(_), true) => bail!("pick an emoji or --remove, not both"),
                (None, false) => bail!("need an emoji or --remove"),
            },
            Op::Contacts => self.client.contacts().await,
            Op::Folders => self.client.folders().await,
            Op::FolderNew { title, include } => self.client.folder_new(&title, &include).await,
            Op::FolderRm { id } => self.client.folder_rm(id).await,
            Op::Pinned { target, limit } => self.client.pinned(&target, limit).await,
            Op::Members { target } => self.client.members(&target).await,
            Op::Kick { target, user } => self.client.kick(&target, &user).await,
            Op::Ban { target, user } => self.client.ban(&target, &user).await,
            Op::Unban { target, user } => self.client.unban(&target, &user).await,
            Op::Promote { target, user, rank } => self.client.promote(&target, &user, rank.as_deref()).await,
        }
    }

    async fn watch(&mut self) -> Result<()> {
        let mirror = mirror::Mirror::open().await?;
        let mut dialogs = self.client.raw.iter_dialogs();
        while let Some(dialog) = dialogs.next().await? {
            let id = dialog.peer.id().bot_api_dialog_id().unwrap_or(0);
            let name = dialog.peer.name().unwrap_or("nobody");
            let username = dialog.peer.username().map(str::to_string);
            let unread = match &dialog.raw {
                tl::enums::Dialog::Dialog(d) => d.unread_count,
                _ => 0,
            };
            mirror.upsert_chat(id, name, username.as_deref(), unread, None, None).await?;
        }
        let mut stream = self.client.messages_stream().await?;
        println!("watching... (Ctrl-C to stop)");
        loop {
            match stream.next().await {
                Ok(update) => match update {
                    Update::NewMessage(msg) => {
                        let who = if msg.outgoing() {
                            "you".into()
                        } else {
                            msg.sender()
                                .and_then(|sender| sender.name().map(str::to_string))
                                .unwrap_or_default()
                        };
                        let media = msg.media().as_ref().map(Out::media_kind);
                        if media.is_some() {
                            println!("[{}] {}: {} [{}]", Out::time(&msg.date()), who, msg.text(), media.unwrap());
                        } else {
                            println!("[{}] {}: {}", Out::time(&msg.date()), who, msg.text());
                        }
                        mirror
                            .insert_message(
                                msg.peer_id().bot_api_dialog_id().unwrap_or(0),
                                msg.id(),
                                msg.date().timestamp(),
                                msg.outgoing() as i32,
                                msg.sender()
                                    .and_then(|sender| sender.name())
                                    .map(str::to_string)
                                    .as_deref(),
                                msg.text(),
                                media,
                            )
                            .await?;
                    }
                    Update::MessageEdited(msg) => {
                        println!("[edited {}] {}", Out::time(&msg.date()), msg.text());
                    }
                    _ => {}
                },
                Err(err) => eprintln!("update error: {err}"),
            }
        }
    }

    async fn sync(&self, limit: usize) -> Result<()> {
        let mirror = mirror::Mirror::open().await?;
        let mut iter = self.client.raw.iter_dialogs();
        while let Some(dialog) = iter.next().await? {
            let name = dialog.peer.name().unwrap_or("nobody");
            let id = dialog.peer.id().bot_api_dialog_id().unwrap_or(0);
            let username = dialog.peer.username().map(str::to_string);
            let unread = match &dialog.raw {
                tl::enums::Dialog::Dialog(d) => d.unread_count,
                _ => 0,
            };
            let (last_date, last) = match &dialog.last_message {
                Some(msg) => (Some(msg.date().timestamp()), Some(Out::preview(msg.text()))),
                None => (None, None),
            };
            mirror.upsert_chat(id, &name, username.as_deref(), unread, last_date, last.as_deref()).await?;
            let mut msgs = self.client.raw.iter_messages(dialog.peer_ref()).limit(limit);
            while let Some(msg) = msgs.next().await? {
                let sender = msg.sender().and_then(|sender| sender.name()).map(str::to_string);
                let media = msg.media().as_ref().map(Out::media_kind);
                mirror.insert_message(
                    id,
                    msg.id(),
                    msg.date().timestamp(),
                    msg.outgoing() as i32,
                    sender.as_deref(),
                    msg.text(),
                    media,
                ).await?;
            }
            println!("synced {name}");
        }
        Ok(())
    }

    async fn search(&self, query: &str, chat: Option<String>) -> Result<()> {
        let mirror = mirror::Mirror::open().await?;
        let results = mirror.search(query, chat.as_deref()).await?;
        for hit in results {
            let date = DateTime::from_timestamp(hit.date, 0)
                .map(|when| Out::date(&when))
                .unwrap_or_default();
            let who = if hit.out == 1 { "you" } else { &hit.sender };
            println!("[{}] {} {who}: {}", hit.chat_name, date, hit.text);
        }
        Ok(())
    }

    async fn dialogs(&self) -> Result<()> {
        let mut iter = self.client.raw.iter_dialogs();
        while let Some(dialog) = iter.next().await? {
            let unread = match &dialog.raw {
                tl::enums::Dialog::Dialog(d) => d.unread_count,
                _ => 0,
            };
            let marker = if unread > 0 {
                format!(" ({unread} unread)")
            } else {
                String::new()
            };
            let preview = match &dialog.last_message {
                Some(msg) => {
                    let text = Out::preview(msg.text());
                    if msg.outgoing() {
                        format!("{}{} -> {text}", Out::dialog_id(dialog.peer.id()), marker)
                    } else {
                        format!(
                            "{}{} {} {text}",
                            Out::dialog_id(dialog.peer.id()),
                            marker,
                            Out::time(&msg.date())
                        )
                    }
                }
                None => format!("{} no messages", Out::dialog_id(dialog.peer.id())),
            };
            println!("{}", dialog.peer.name().unwrap_or("?"));
            println!("  {preview}");
        }
        Ok(())
    }

    async fn messages(&self, target: &str, limit: usize) -> Result<()> {
        let peer = self.client.resolve(target).await?;
        let mut iter = self.client.raw.iter_messages(peer).limit(limit);
        let mut out = vec![];
        while let Some(msg) = iter.next().await? {
            out.push(msg);
        }
        for msg in out.iter().rev() {
            let who = msg
                .sender()
                .and_then(|sender| sender.name())
                .unwrap_or("?");
            println!(
                "[{}] {} {}: {}",
                msg.id(),
                Out::date(&msg.date()),
                who,
                msg.text()
            );
        }
        Ok(())
    }
}