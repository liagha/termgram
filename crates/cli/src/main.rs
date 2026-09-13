mod cli;
mod format;

use anyhow::{bail, Context, Result};
use chrono::{Local, NaiveDateTime, NaiveTime};
use clap::Parser;
use std::time::Instant;

use cli::{Cli, Command, What};
use format::Out;
use std::time::Duration;
use termgram::{Client, Mirror, Update};

async fn with_feedback<T, F>(label: &str, json: bool, fut: F) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    if json {
        return fut.await;
    }
    let start = Instant::now();
    let mut fut = Box::pin(fut);
    let mut timer = Box::pin(tokio::time::sleep(Duration::from_millis(300)));
    let mut printed = false;
    loop {
        tokio::select! {
            res = &mut fut => {
                if printed {
                    match &res {
                        Ok(_) => eprintln!(" ok {:.1}s", start.elapsed().as_secs_f32()),
                        Err(_) => eprintln!(" failed {:.1}s", start.elapsed().as_secs_f32()),
                    }
                }
                return res;
            }
            _ = &mut timer, if !printed => {
                eprint!("{label}...");
                let _ = std::io::Write::flush(&mut std::io::stderr());
                printed = true;
            }
        }
    }
}

async fn watch(
    client: &mut Client,
    target: Option<&str>,
    once: bool,
    unread: bool,
    every: u64,
) -> Result<()> {
    if unread {
        let mut seen = std::collections::HashMap::new();
        loop {
            for dialog in client.unread().await? {
                let prev = seen.get(&dialog.id).copied().unwrap_or(0);
                if dialog.unread > prev {
                    seen.insert(dialog.id, dialog.unread);
                    let last = dialog.last.unwrap_or_default();
                    let arrow = if dialog.out { "-> " } else { "" };
                    println!(
                        "[{}] {} ({}): {arrow}{}",
                        Out::time(dialog.at.unwrap_or(0)),
                        dialog.name,
                        dialog.unread,
                        last
                    );
                }
            }
            if once {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_secs(every)).await;
        }
    }
    let want = match target {
        Some(t) => Some(client.resolve(t).await?.id.bot_api_dialog_id().unwrap_or(0)),
        None => None,
    };
    if once {
        let t = target.context("watch --once needs a target")?;
        for line in client.messages(t, 20).await? {
            let who = if line.out {
                "you".to_string()
            } else {
                line.who.unwrap_or_else(|| "?".to_string())
            };
            let mut text = format!("[{}] {}: {}", Out::time(line.at), who, line.text);
            if let Some(kind) = line.media {
                text.push_str(&format!(" [{kind}]"));
            }
            println!("{text}");
        }
        return Ok(());
    }
    let mirror = Mirror::open(&client.mirror()).await?;
    let mut dirs = client.raw.iter_dialogs();
    let mut names = std::collections::HashMap::new();
    while let Some(dialog) = dirs.next().await? {
        let id = dialog.peer_id().bot_api_dialog_id().unwrap_or(0);
        let name = dialog
            .peer
            .name()
            .map(str::to_string)
            .or_else(|| dialog.peer.username().map(str::to_string))
            .unwrap_or_default();
        names.insert(id, name.clone());
        mirror
            .upsert_chat(
                id,
                &name,
                dialog.peer.username().map(str::to_string).as_deref(),
                0,
                None,
                None,
            )
            .await;
    }
    let mut events = client.signals();
    println!("watching... (Ctrl-C to stop)");
    loop {
        let update = match events.recv().await {
            Ok(update) => update,
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            Err(_) => break,
        };
        match update {
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
                    let from = if target.is_none() {
                        format!("{}: ", names.get(&chat).map(String::as_str).unwrap_or("?"))
                    } else {
                        String::new()
                    };
                    let mut text =
                        format!("[{}] {}{}: {}", Out::time(msg.date().timestamp()), from, who, msg.text());
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
            }
    }
    Ok(())
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
    let mut client = with_feedback(
        "connecting",
        cli.json,
        async {
            if matches!(cli.op, Command::Login) {
                Client::new_boot(&cfg, &cli.account).await
            } else {
                Client::new(&cfg, &cli.account).await
            }
        },
    )
    .await?;
    match cli.op {
        Command::Login => {
            let me = client.login(&api_hash).await?;
            println!("logged in as {}", me.name);
        }
        Command::Logout => println!("{}", client.logout().await?.text),
        Command::Me => {
            let me = with_feedback("fetching identity", cli.json, client.me()).await?;
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
            let rows = with_feedback("fetching dialogs", cli.json, client.dialogs()).await?;
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
            let rows = with_feedback(
                "fetching messages",
                cli.json,
                client.messages(&target, limit),
            )
            .await?;
            dump(cli.json, &rows)?;
            if cli.json {
                return Ok(());
            }
            for line in rows {
                let at = if cli.full { Out::date(line.at) } else { Out::time(line.at) };
                println!("[{}] {at} {}: {}", line.id, Out::who(&line), line.text);
            }
        }
        Command::Send {
            target,
            text,
            files,
            reply,
            topic,
            format,
            dates,
            at,
        } => {
            let format = termgram::Format::parse(format.as_deref())?;
            let args = termgram::Send {
                target,
                text: text.map(|t| termgram::Text {
                    text: t,
                    format,
                    dates,
                }),
                files,
                reply,
                topic,
                at: at.as_deref().map(parse_at).transpose()?,
            };
            let show = !cli.json;
            if show {
                eprint!("sending to {}...", args.target);
                let _ = std::io::Write::flush(&mut std::io::stderr());
            }
            let tick = Instant::now();
            let sent = match client.send(&args).await {
                Ok(v) => {
                    if show {
                        eprintln!(" ok {:.1}s", tick.elapsed().as_secs_f32());
                    }
                    v
                }
                Err(err) => {
                    if show {
                        eprintln!(" failed {:.1}s", tick.elapsed().as_secs_f32());
                    }
                    return Err(err);
                }
            };
            if sent.ids.len() == 1 {
                println!("sent {}", sent.ids[0]);
            } else {
                println!("sent {} messages", sent.ids.len());
            }
        }
        Command::Read { target } => {
            println!("{}", client.mark_as_read(&target).await?.text);
        }
        Command::Watch {
            target,
            once,
            unread,
            every,
        } => watch(&mut client, target.as_deref(), once, unread, every).await?,
        Command::Poll {
            target,
            question,
            options,
            anon,
            multi,
        } => {
            for id in client
                .poll(&target, &question, &options, anon, multi)
                .await?
                .ids
            {
                println!("sent {id}");
            }
        }
        Command::Topics { target } => {
            let rows = with_feedback("fetching topics", cli.json, client.topics(&target)).await?;
            dump(cli.json, &rows)?;
            if !cli.json {
                for row in rows {
                    println!("[{}] {}", row.id, row.title);
                }
            }
        }
        Command::Sync { limit } => {
            let show = !cli.json;
            if show {
                eprint!("syncing...");
                let _ = std::io::Write::flush(&mut std::io::stderr());
            }
            let tick = Instant::now();
            let s = match client.sync(limit).await {
                Ok(v) => {
                    if show {
                        eprintln!(" ok {:.1}s", tick.elapsed().as_secs_f32());
                    }
                    v
                }
                Err(err) => {
                    if show {
                        eprintln!(" failed {:.1}s", tick.elapsed().as_secs_f32());
                    }
                    return Err(err);
                }
            };
            dump(cli.json, &s)?;
            if !cli.json {
                println!("synced {} chats, {} lines", s.chats, s.lines);
            }
        }
        Command::Search { query, chat } => {
            let rows = with_feedback("searching", cli.json, client.search(&query, chat.as_deref()))
                .await?;
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
        Command::Download { target, id } => {
            let show = !cli.json;
            if show {
                eprint!("downloading...");
                let _ = std::io::Write::flush(&mut std::io::stderr());
            }
            let tick = Instant::now();
            let done = match client.download(&target, id).await {
                Ok(v) => {
                    if show {
                        eprintln!(" ok {:.1}s", tick.elapsed().as_secs_f32());
                    }
                    v
                }
                Err(err) => {
                    if show {
                        eprintln!(" failed {:.1}s", tick.elapsed().as_secs_f32());
                    }
                    return Err(err);
                }
            };
            println!("saved {}", done.path);
        }
        Command::Edit {
            target,
            id,
            text,
            format,
            dates,
        } => {
            let text = termgram::Text {
                text,
                format: termgram::Format::parse(format.as_deref())?,
                dates,
            };
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
            big,
        } => {
            match (emoji, remove) {
                (Some(emoji), false) => {
                    println!(
                        "{}",
                        client.react(&target, id, Some(&emoji), false, big).await?.text
                    );
                }
                (None, true) => {
                    println!("{}", client.react(&target, id, None, true, false).await?.text);
                }
                (Some(_), true) => bail!("pick an emoji or --remove, not both"),
                (None, false) => bail!("need an emoji or --remove"),
            }
        }
        Command::Reactions { target, id } => {
            let rows = client.reactions(&target, id).await?;
            dump(cli.json, &rows)?;
            if cli.json {
                return Ok(());
            }
            for reaction in rows {
                println!("{} x{}", reaction.emoji, reaction.count);
            }
        }
        Command::Contacts => {
            let rows = with_feedback("fetching contacts", cli.json, client.contacts()).await?;
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
        Command::AddContact { phone, first, last } => {
            println!("{}", client.add_contact(&phone, &first, &last).await?.text);
        }
        Command::DeleteContact { target } => {
            println!("{}", client.delete_contact(&target).await?.text);
        }
        Command::ExportContacts => {
            let rows = client.export_contacts().await?;
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
        Command::ImportContacts { contact } => {
            let mut contacts = Vec::new();
            for pair in &contact {
                let parts: Vec<&str> = pair.split(',').collect();
                if parts.len() != 3 {
                    bail!("contact {pair:?} must be \"phone,first,last\"");
                }
                contacts.push(termgram::PhoneContact {
                    phone: parts[0].to_string(),
                    first: parts[1].to_string(),
                    last: parts[2].to_string(),
                });
            }
            println!("imported {} contact(s)", client.import_contacts(&contacts).await?.n);
        }
        Command::BlockList => {
            let rows = client.block_list().await?;
            dump(cli.json, &rows)?;
            if cli.json {
                return Ok(());
            }
            for member in rows {
                println!("{}", member.name);
            }
        }
        Command::DelPhoto { target } => {
            println!("{}", client.del_photo(&target).await?.text);
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
        Command::Draft {
            target,
            text,
            format,
            dates,
        } => {
            let format = termgram::Format::parse(format.as_deref())?;
            let body = text.map(|t| termgram::Text {
                text: t,
                format,
                dates,
            });
            println!("{}", client.draft(&target, body.as_ref()).await?.text);
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
            let show = !cli.json;
            if show {
                eprint!("grabbing media from {}...", target);
                let _ = std::io::Write::flush(&mut std::io::stderr());
            }
            let tick = Instant::now();
            let done = match client.grab(&target).await {
                Ok(v) => {
                    if show {
                        eprintln!(" ok {:.1}s", tick.elapsed().as_secs_f32());
                    }
                    v
                }
                Err(err) => {
                    if show {
                        eprintln!(" failed {:.1}s", tick.elapsed().as_secs_f32());
                    }
                    return Err(err);
                }
            };
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