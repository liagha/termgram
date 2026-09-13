mod cli;
mod format;

use anyhow::{bail, Context, Result};
use chrono::{Local, NaiveDateTime, NaiveTime};
use clap::Parser;

use cli::{Cli, Command, What};
use format::Out;
use termgram::{Client, Mirror, Update};

async fn watch(client: &mut Client, target: Option<&str>) -> Result<()> {
    let want = match target {
        Some(t) => Some(client.resolve(t).await?.id.bot_api_dialog_id().unwrap_or(0)),
        None => None,
    };
    let mirror = Mirror::open(&client.mirror()).await?;
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
                    let chat = msg.peer_id().bot_api_dialog_id().unwrap_or(0);
                    if let Some(want) = want {
                        if chat != want {
                            continue;
                        }
                    }
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
                            chat,
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
                Update::Raw(raw) => match &*raw {
                    grammers_client::tl::enums::Update::MessageReactions(ur) => {
                        let grammers_client::tl::enums::MessageReactions::Reactions(page) =
                            &ur.reactions;
                        for raw in &page.results {
                            let grammers_client::tl::enums::ReactionCount::Count(count) = raw;
                            let emoji = match &count.reaction {
                                grammers_client::tl::enums::Reaction::Emoji(e) => {
                                    e.emoticon.clone()
                                }
                                grammers_client::tl::enums::Reaction::CustomEmoji(_) => {
                                    "custom".to_string()
                                }
                                grammers_client::tl::enums::Reaction::Paid => "paid".to_string(),
                                grammers_client::tl::enums::Reaction::Empty => "?".to_string(),
                            };
                            println!("[react {}] {} x{}", ur.msg_id, emoji, count.count);
                        }
                    }
                    grammers_client::tl::enums::Update::UserTyping(u) => {
                        println!("[typing] user {}: {u:?}", u.user_id);
                    }
                    grammers_client::tl::enums::Update::ChatUserTyping(u) => {
                        println!("[typing] chat {}: {u:?}", u.chat_id);
                    }
                    grammers_client::tl::enums::Update::ChannelUserTyping(u) => {
                        println!("[typing] channel {}: {u:?}", u.channel_id);
                    }
                    _ => {}
                },
                _ => {}
            },
            Err(err) => eprintln!("update error: {err}"),
        }
    }
}

fn dump<T: serde::Serialize>(json: bool, value: &T) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    }
    Ok(())
}

fn parse_at(s: &str) -> Result<u64> {
    if s == "now" {
        return Ok(Local::now().timestamp() as u64);
    }
    if let Ok(t) = NaiveTime::parse_from_str(s, "%H:%M") {
        let when = Local::now()
            .date_naive()
            .and_time(t)
            .and_local_timezone(Local)
            .single()
            .with_context(|| format!("can't parse time from {s:?}"))?;
        return Ok(when.timestamp() as u64);
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M") {
        let when = dt
            .and_local_timezone(Local)
            .single()
            .with_context(|| format!("can't parse datetime from {s:?}"))?;
        return Ok(when.timestamp() as u64);
    }
    bail!("at must be now, HH:MM, or YYYY-MM-DD HH:MM")
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = termgram::Config::load()?;
    let api_hash = cfg.api_hash.clone();
    let mut client = Client::new(&cfg, &cli.account).await?;
    match cli.op {
        Command::Login => {
            let me = client.login(&api_hash).await?;
            println!("logged in as {}", me.name);
        }
        Command::Logout => println!("{}", client.logout().await?.text),
        Command::Me => {
            let me = client.me().await?;
            dump(cli.json, &me)?;
            if !cli.json {
                print!("{}", me.name);
                if let Some(handle) = me.handle {
                    print!(" / @{handle}");
                }
                println!(" (id {})", me.id);
            }
        }
        Command::Dialogs => {
            let rows = client.dialogs().await?;
            dump(cli.json, &rows)?;
            if cli.json {
                return Ok(());
            }
            for dialog in rows {
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
            let rows = client.messages(&target, limit).await?;
            dump(cli.json, &rows)?;
            if cli.json {
                return Ok(());
            }
            for line in rows {
                let at = if cli.full { Out::date(line.at) } else { Out::time(line.at) };
                println!("[{}] {at} {}: {}", line.id, Out::who(&line), line.text);
            }
        }
        Command::Send { target, text, reply } => {
            let sent = client.send(&target, &text, reply).await?;
            println!("sent {}", sent.id);
        }
        Command::Read { target } => {
            println!("{}", client.mark_as_read(&target).await?.text);
        }
        Command::Watch { target } => watch(&mut client, target.as_deref()).await?,
        Command::Sync { limit } => {
            let s = client.sync(limit).await?;
            dump(cli.json, &s)?;
            if !cli.json {
                println!("synced {} chats, {} lines", s.chats, s.lines);
            }
        }
        Command::Search { query, chat } => {
            let rows = client.search(&query, chat.as_deref()).await?;
            dump(cli.json, &rows)?;
            if cli.json {
                return Ok(());
            }
            for hit in rows {
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
            let rows = client.contacts().await?;
            dump(cli.json, &rows)?;
            if cli.json {
                return Ok(());
            }
            for contact in rows {
                match contact.handle {
                    Some(handle) => println!("{} / @{handle}", contact.name),
                    None => println!("{}", contact.name),
                }
            }
        }
        Command::Folders => {
            let rows = client.folders().await?;
            dump(cli.json, &rows)?;
            if cli.json {
                return Ok(());
            }
            for folder in rows {
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
            let rows = client.pinned(&target, limit).await?;
            dump(cli.json, &rows)?;
            if cli.json {
                return Ok(());
            }
            for pinned in rows {
                println!("[{}] {}", pinned.id, pinned.text);
            }
        }
        Command::Members { target } => {
            let rows = client.members(&target).await?;
            dump(cli.json, &rows)?;
            if cli.json {
                return Ok(());
            }
            for member in rows {
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
        Command::Typing { target } => {
            println!("{}", client.typing(&target).await?.text);
        }
        Command::Status { target } => {
            let status = client.status(&target).await?;
            dump(cli.json, &status)?;
            if !cli.json {
                println!("{}: {}", status.who, status.state);
            }
        }
        Command::Schedule { target, text, at } => {
            let sent = client.schedule(&target, &text, parse_at(&at)?).await?;
            println!("scheduled {}", sent.id);
        }
        Command::Scheduled { target } => {
            let rows = client.scheduled(&target).await?;
            dump(cli.json, &rows)?;
            if cli.json {
                return Ok(());
            }
            for planned in rows {
                println!("[{} {}] {}", planned.id, Out::date(planned.at), planned.text);
            }
        }
        Command::Cancel { target, id } => {
            println!("{}", client.cancel(&target, id).await?.text);
        }
        Command::Draft { target, text } => {
            println!(
                "{}",
                client.draft(&target, text.as_deref()).await?.text
            );
        }
        Command::Drafts => {
            let rows = client.drafts().await?;
            dump(cli.json, &rows)?;
            if cli.json {
                return Ok(());
            }
            for draft in rows {
                let id = draft.chat.to_string();
                let chat = draft.name.as_deref().unwrap_or(&id);
                println!("{chat}: {}", draft.text);
            }
        }
        Command::Profile { target } => {
            let profile = client.profile(&target).await?;
            dump(cli.json, &profile)?;
            if !cli.json {
                println!("{} / @{} (id {})", profile.name, profile.handle.unwrap_or_default(), profile.id);
                println!("state: {}", profile.state);
                if let Some(about) = profile.about {
                    println!("about: {about}");
                }
                println!("blocked: {}", profile.blocked);
            }
        }
        Command::Set { what } => match what {
            What::Name { first, last } => {
                println!("{}", client.setname(&first, &last).await?.text);
            }
            What::Bio { about } => {
                println!("{}", client.setbio(&about).await?.text);
            }
            What::Photo { path } => {
                println!("{}", client.setphoto(&path).await?.text);
            }
        },
        Command::Block { target } => {
            println!("{}", client.block(&target).await?.text);
        }
        Command::Unblock { target } => {
            println!("{}", client.unblock(&target).await?.text);
        }
        Command::Searchall { query, limit } => {
            let rows = client.searchall(&query, limit).await?;
            dump(cli.json, &rows)?;
            if cli.json {
                return Ok(());
            }
            for hit in rows {
                println!(
                    "[{}] {} {}: {}",
                    hit.chat,
                    Out::date(hit.line.at),
                    Out::who(&hit.line),
                    hit.line.text
                );
            }
        }
        Command::Searchin {
            target,
            query,
            limit,
        } => {
            let rows = client.searchin(&target, &query, limit).await?;
            dump(cli.json, &rows)?;
            if cli.json {
                return Ok(());
            }
            for line in rows {
                println!(
                    "[{}] {} {}: {}",
                    line.id,
                    Out::date(line.at),
                    Out::who(&line),
                    line.text
                );
            }
        }
        Command::Photo {
            target,
            path,
            caption,
        } => {
            let sent = client.photo(&target, &path, caption.as_deref()).await?;
            println!("sent {}", sent.id);
        }
        Command::Album {
            target,
            paths,
            caption,
        } => {
            let done = client.album(&target, &paths, caption.as_deref()).await?;
            println!("sent album with {} items", done.n);
        }
        Command::Voice { target, path } => {
            let sent = client.voice(&target, &path).await?;
            println!("sent {}", sent.id);
        }
        Command::Cached { target, limit } => {
            let mirror = Mirror::open(&client.mirror()).await?;
            let rows = mirror.lines(&target, limit).await?;
            dump(cli.json, &rows)?;
            if cli.json {
                return Ok(());
            }
            for row in rows {
                let who = match row.who {
                    Some(name) => name,
                    None => "?".to_string(),
                };
                let at = if cli.full { Out::date(row.at) } else { Out::time(row.at) };
                println!("[{}] {at} {who}: {}", row.id, row.text);
            }
        }
        Command::Grab { target } => {
            let done = client.grab(&target).await?;
            println!("grabbed {} media files", done.n);
        }
        Command::Export { path } => {
            println!("{}", client.export(path.as_deref().unwrap_or("termgram-export")).await?.text);
        }
        Command::Import { path } => {
            println!("{}", client.import(&path).await?.text);
        }
        Command::Wipe => {
            println!("{}", client.wipe().await?.text);
        }
    }
    Ok(())
}