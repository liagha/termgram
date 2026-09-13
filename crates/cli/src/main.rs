mod cli;
mod format;

use anyhow::{bail, Result};
use clap::Parser;

use cli::{Cli, Command};
use format::Out;
use termgram::{Client, Mirror, Update};

async fn watch(client: &mut Client) -> Result<()> {
    let mirror = Mirror::open().await?;
    let mut iter = client.raw.iter_dialogs();
    while let Some(dialog) = iter.next().await? {
        mirror
            .upsert_chat(
                dialog.peer_id().bot_api_dialog_id().unwrap_or(0),
                dialog.peer.name().unwrap_or(""),
                dialog.peer.username().map(str::to_string).as_deref(),
                0,
                None,
                None,
            )
            .await;
    }
    let mut stream = client.messages_stream().await?;
    println!("watching... (Ctrl-C to stop)");
    loop {
        match stream.next().await {
            Ok(update) => match update {
                Update::NewMessage(msg) => {
                    let who = if msg.outgoing() {
                        "you"
                    } else {
                        msg.sender().and_then(|s| s.name()).unwrap_or("?")
                    };
                    let mut text =
                        format!("[{}] {}: {}", Out::time(msg.date().timestamp()), who, msg.text());
                    if let Some(media) = msg.media() {
                        if let Some(kind) = termgram::Label::kind(&media) {
                            text.push_str(&format!(" [{kind}]"));
                        }
                    }
                    println!("{text}");
                    mirror
                        .insert_message(
                            msg.peer_id().bot_api_dialog_id().unwrap_or(0),
                            msg.id(),
                            msg.date().timestamp(),
                            msg.outgoing() as i32,
                            msg.sender().and_then(|s| s.name()).map(str::to_string).as_deref(),
                            msg.text(),
                            msg.media()
                                .as_ref()
                                .map(termgram::Label::kind)
                                .unwrap_or(None)
                                .as_deref(),
                        )
                        .await;
                }
                Update::MessageEdited(msg) => {
                    println!(
                        "[edited {}] {}",
                        Out::time(msg.date().timestamp()),
                        msg.text()
                    );
                }
                _ => {}
            },
            Err(err) => eprintln!("update error: {err}"),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = termgram::Config::load()?;
    let api_hash = cfg.api_hash.clone();
    let mut client = Client::new(&cfg).await?;
    match cli.op {
        Command::Login => {
            let me = client.login(&api_hash).await?;
            println!("logged in as {}", me.name);
        }
        Command::Logout => println!("{}", client.logout().await?.text),
        Command::Me => {
            let me = client.me().await?;
            print!("{}", me.name);
            if let Some(handle) = me.handle {
                print!(" / @{handle}");
            }
            println!(" (id {})", me.id);
        }
        Command::Dialogs => {
            for dialog in client.dialogs().await? {
                println!("{}", dialog.name);
                let marker = if dialog.unread > 0 {
                    format!(" ({})", dialog.unread)
                } else {
                    String::new()
                };
                match dialog.last {
                    Some(text) => {
                        let line = Out::preview(&text);
                        if dialog.out {
                            println!("  {} {marker}-> {line}", dialog.id);
                        } else {
                            println!(
                                "  {}{} {} {line}",
                                dialog.id,
                                marker,
                                Out::time(dialog.at.unwrap_or(0))
                            );
                        }
                    }
                    None => println!("  {}{} no messages", dialog.id, marker),
                }
            }
        }
        Command::Messages { target, limit } => {
            for line in client.messages(&target, limit).await? {
                println!(
                    "[{}] {} {}: {}",
                    line.id,
                    Out::date(line.at),
                    Out::who(&line),
                    line.text
                );
            }
        }
        Command::Send { target, text, reply } => {
            let sent = client.send(&target, &text, reply).await?;
            println!("sent {}", sent.id);
        }
        Command::Read { target } => {
            println!("{}", client.mark_as_read(&target).await?.text);
        }
        Command::Watch => watch(&mut client).await?,
        Command::Sync { limit } => {
            let s = client.sync(limit).await?;
            println!("synced {} chats, {} lines", s.chats, s.lines);
        }
        Command::Search { query, chat } => {
            for hit in client.search(&query, chat.as_deref()).await? {
                println!(
                    "[{}] {} {}: {}",
                    hit.chat,
                    Out::date(hit.line.at),
                    Out::who(&hit.line),
                    hit.line.text
                );
            }
        }
        Command::Upload {
            target,
            path,
            caption,
        } => {
            let sent = client.upload(&target, &path, caption.as_deref()).await?;
            println!("sent {}", sent.id);
        }
        Command::Download { target, id } => {
            println!("saved {}", client.download(&target, id).await?.path);
        }
        Command::Edit { target, id, text } => {
            println!("{}", client.edit(&target, id, &text).await?.text);
        }
        Command::Delete { target, ids } => {
            println!("deleted {}", client.delete(&target, &ids).await?.n);
        }
        Command::Forward { from, to, ids } => {
            println!("forwarded {}", client.forward(&from, &to, &ids).await?.n);
        }
        Command::Pin { target, id, remove } => {
            println!("{}", client.pin(&target, id, remove).await?.text);
        }
        Command::React {
            target,
            id,
            emoji,
            remove,
        } => {
            match (emoji, remove) {
                (Some(emoji), false) => {
                    println!("{}", client.react(&target, id, Some(&emoji), false).await?.text);
                }
                (None, true) => {
                    println!("{}", client.react(&target, id, None, true).await?.text);
                }
                (Some(_), true) => bail!("pick an emoji or --remove, not both"),
                (None, false) => bail!("need an emoji or --remove"),
            }
        }
        Command::Contacts => {
            for contact in client.contacts().await? {
                match contact.handle {
                    Some(handle) => println!("{} / @{handle}", contact.name),
                    None => println!("{}", contact.name),
                }
            }
        }
        Command::Folders => {
            for folder in client.folders().await? {
                println!("{} {}", folder.id, folder.title);
            }
        }
        Command::FolderNew { title, include } => {
            let folder = client.folder_new(&title, &include).await?;
            println!("folder {} created", folder.id);
        }
        Command::FolderRm { id } => {
            println!("{}", client.folder_rm(id).await?.text);
        }
        Command::Pinned { target, limit } => {
            for pinned in client.pinned(&target, limit).await? {
                println!("[{}] {}", pinned.id, pinned.text);
            }
        }
        Command::Members { target } => {
            for member in client.members(&target).await? {
                println!("{}", member.name);
            }
        }
        Command::Kick { target, user } => {
            println!("{}", client.kick(&target, &user).await?.text);
        }
        Command::Ban { target, user } => {
            println!("{}", client.ban(&target, &user).await?.text);
        }
        Command::Unban { target, user } => {
            println!("{}", client.unban(&target, &user).await?.text);
        }
        Command::Promote { target, user, rank } => {
            println!(
                "{}",
                client.promote(&target, &user, rank.as_deref()).await?.text
            );
        }
    }
    Ok(())
}