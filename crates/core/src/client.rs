use std::io::Write;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use grammers_client::client::UpdateStream;
use grammers_client::media::Media;
use grammers_client::message::{InputMessage, InputReactions};
use grammers_client::peer::User;
use grammers_client::tl;
use grammers_client::{Client as Raw, SignInError};
use grammers_mtsender::SenderPool;
use grammers_session::storages::SqliteSession;
use grammers_session::types::{PeerId, PeerRef};
use grammers_session::updates::UpdatesLike;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::mirror::Mirror;
use crate::{
    Ack, Contact, Dialog, Done, Folder, Hit, Identity, Label, Line, Member, Path, Pinned, Sent,
    Summary,
};

pub struct Client {
    pub raw: Raw,
    updates: Option<mpsc::UnboundedReceiver<UpdatesLike>>,
}

impl Client {
    pub async fn new(cfg: &Config) -> Result<Self> {
        let session = Arc::new(SqliteSession::open(&Config::session()).await?);
        let pool = SenderPool::new(session, cfg.api_id);
        let updates = pool.updates;
        tokio::spawn(pool.runner.run());
        let raw = Raw::new(pool.handle);
        Ok(Self {
            raw,
            updates: Some(updates),
        })
    }

    pub async fn messages_stream(&mut self) -> Result<UpdateStream> {
        let updates = self.updates.take().context("already streaming")?;
        self.raw
            .stream_updates(updates, Default::default())
            .await
            .map_err(|err| anyhow::anyhow!("{err}"))
    }

    pub async fn resolve(&self, target: &str) -> Result<PeerRef> {
        if target == "saved" || target == "me" {
            return Ok(PeerId::self_user().to_ambient_ref());
        }
        if let Ok(id) = target.parse::<i64>() {
            let wanted = PeerId::from_bot_api_dialog_id(id)
                .context("target id out of range")?;
            let mut iter = self.raw.iter_dialogs();
            while let Some(dialog) = iter.next().await? {
                if dialog.peer_id() == wanted {
                    return Ok(dialog.peer_ref());
                }
            }
            bail!(
                "no dialog with id {id} - run `termgram dialogs` or use @username"
            );
        }
        let peer = self
            .raw
            .resolve_username(target)
            .await
            .map_err(|err| anyhow::anyhow!("{err}"))?
            .context("peer not found")?;
        peer.to_ref()
            .await
            .map_err(|err| anyhow::anyhow!("{err}"))?
            .context("peer \"{target}\" not usable")
    }

    fn identity(user: &User) -> Identity {
        Identity {
            name: user.full_name(),
            handle: user.username().map(str::to_string),
            id: user.id().to_ambient_ref().id.bot_api_dialog_id().unwrap_or(0),
        }
    }

    pub async fn login(&self, api_hash: &str) -> Result<Identity> {
        if self.raw.is_authorized().await? {
            bail!("already logged in");
        }
        print!("phone: ");
        std::io::stdout().flush()?;
        let mut phone = String::new();
        std::io::stdin().read_line(&mut phone)?;
        let token = self
            .raw
            .request_login_code(phone.trim(), api_hash)
            .await?;
        print!("code: ");
        std::io::stdout().flush()?;
        let mut code = String::new();
        std::io::stdin().read_line(&mut code)?;
        let user = match self.raw.sign_in(&token, code.trim()).await {
            Ok(user) => user,
            Err(SignInError::PasswordRequired(mut token)) => loop {
                if let Some(hint) = token.hint() {
                    println!("2FA hint: {hint}");
                }
                let password = rpassword::prompt_password("password: ")?;
                match self.raw.check_password(token, &password).await {
                    Ok(user) => break user,
                    Err(SignInError::InvalidPassword(next)) => token = next,
                    Err(err) => bail!("{err}"),
                }
            },
            Err(SignInError::InvalidCode) => bail!("wrong code"),
            Err(SignInError::SignUpRequired) => bail!("account not registered"),
            Err(SignInError::Other(err)) => bail!("{err}"),
            Err(SignInError::InvalidPassword(_)) => bail!("invalid password"),
        };
        Ok(Self::identity(&user))
    }

    pub async fn logout(&self) -> Result<Ack> {
        match self.raw.sign_out().await {
            Ok(_) => Ok(Ack {
                text: "logged out".into(),
            }),
            Err(_) => bail!("not logged in"),
        }
    }

    pub async fn me(&self) -> Result<Identity> {
        if !self.raw.is_authorized().await? {
            bail!("not logged in - run `termgram login`");
        }
        let me = self.raw.get_me().await?;
        Ok(Self::identity(&me))
    }

    pub async fn dialogs(&self) -> Result<Vec<Dialog>> {
        let mut rows = vec![];
        let mut iter = self.raw.iter_dialogs();
        while let Some(dialog) = iter.next().await? {
            let (out, at, last) = match &dialog.last_message {
                Some(m) => (
                    m.outgoing(),
                    Some(m.date().timestamp()),
                    Some(m.text().to_string()),
                ),
                None => (false, None, None),
            };
            rows.push(Dialog {
                id: dialog.peer_id().bot_api_dialog_id().unwrap_or(0),
                name: dialog.peer.name().unwrap_or("").to_string(),
                unread: match &dialog.raw {
                    tl::enums::Dialog::Dialog(d) => d.unread_count,
                    _ => 0,
                },
                out,
                at,
                last,
            });
        }
        Ok(rows)
    }

    pub async fn messages(&self, target: &str, limit: usize) -> Result<Vec<Line>> {
        let peer = self.resolve(target).await?;
        let mut rows = vec![];
        let mut iter = self.raw.iter_messages(peer).limit(limit);
        while let Some(m) = iter.next().await? {
            rows.push(Line {
                id: m.id(),
                at: m.date().timestamp(),
                out: m.outgoing(),
                who: m.sender().and_then(|s| s.name()).map(str::to_string),
                text: m.text().to_string(),
                media: m.media().as_ref().map(Label::kind).flatten().map(str::to_string),
            });
        }
        rows.reverse();
        Ok(rows)
    }

    pub async fn send(&self, target: &str, text: &str, reply: Option<i32>) -> Result<Sent> {
        let peer = self.resolve(target).await?;
        let msg = InputMessage::new().text(text).reply_to(reply);
        let sent = self.raw.send_message(peer, msg).await?;
        Ok(Sent { id: sent.id() })
    }

    pub async fn mark_as_read(&self, target: &str) -> Result<Ack> {
        let peer = self.resolve(target).await?;
        self.raw.mark_as_read(peer).await?;
        Ok(Ack {
            text: "marked as read".into(),
        })
    }

    pub async fn upload(
        &self,
        target: &str,
        path: &str,
        caption: Option<&str>,
    ) -> Result<Sent> {
        let peer = self.resolve(target).await?;
        let file = self.raw.upload_file(path).await?;
        let mut msg = InputMessage::new().document(file);
        if let Some(caption) = caption {
            msg = msg.text(caption);
        }
        let sent = self.raw.send_message(peer, msg).await?;
        Ok(Sent { id: sent.id() })
    }

    pub async fn download(&self, target: &str, wanted: Option<i32>) -> Result<Path> {
        let peer = self.resolve(target).await?;
        let mut iter = self.raw.iter_messages(peer);
        while let Some(m) = iter.next().await? {
            if let Some(id) = wanted {
                if m.id() != id {
                    continue;
                }
                return match m.media() {
                    Some(media) => self.save_media(target, id, &media).await,
                    None => bail!("message {id} has no media"),
                };
            }
            if let Some(media) = m.media() {
                return self.save_media(target, m.id(), &media).await;
            }
        }
        bail!("no media in {target}")
    }

    async fn save_media(&self, target: &str, id: i32, media: &Media) -> Result<Path> {
        std::fs::create_dir_all(Config::media())?;
        let ext = match media {
            Media::Document(doc) => doc
                .name()
                .and_then(|n| n.rsplit('.').next().map(str::to_string))
                .filter(|ext| ext.len() <= 5)
                .unwrap_or_else(|| Label::ext(media).to_string()),
            _ => Label::ext(media).to_string(),
        };
        let path = Config::media().join(format!("{target}_{id}.{ext}"));
        self.raw.download_media(media, &path).await?;
        Ok(Path {
            path: path.display().to_string(),
        })
    }

    pub async fn edit(&self, target: &str, id: i32, text: &str) -> Result<Ack> {
        let peer = self.resolve(target).await?;
        self.raw
            .edit_message(peer, id, InputMessage::new().text(text))
            .await?;
        Ok(Ack {
            text: format!("edited {id}"),
        })
    }

    pub async fn delete(&self, target: &str, ids: &[i32]) -> Result<Done> {
        let peer = self.resolve(target).await?;
        let n = self.raw.delete_messages(peer, ids).await?;
        Ok(Done { n })
    }

    pub async fn forward(&self, from: &str, to: &str, ids: &[i32]) -> Result<Done> {
        let source = self.resolve(from).await?;
        let dest = self.resolve(to).await?;
        let n = self.raw.forward_messages(dest, ids, source).await?.len();
        Ok(Done { n })
    }

    pub async fn pin(&self, target: &str, id: i32, remove: bool) -> Result<Ack> {
        let peer = self.resolve(target).await?;
        if remove {
            self.raw.unpin_message(peer, id).await?;
        } else {
            self.raw.pin_message(peer, id).await?;
        }
        Ok(Ack {
            text: format!("{} {id}", if remove { "unpinned" } else { "pinned" }),
        })
    }

    pub async fn react(
        &self,
        target: &str,
        id: i32,
        emoji: Option<&str>,
        remove: bool,
    ) -> Result<Ack> {
        let peer = self.resolve(target).await?;
        let reactions = if remove {
            InputReactions::remove()
        } else {
            InputReactions::emoticon(emoji.unwrap_or_default())
        };
        self.raw.send_reactions(peer, id, reactions).await?;
        Ok(Ack {
            text: "reacted".into(),
        })
    }

    pub async fn sync(&self, limit: usize) -> Result<Summary> {
        let mirror = Mirror::open().await?;
        let mut chats = 0;
        let mut lines = 0;
        let mut iter = self.raw.iter_dialogs();
        while let Some(dialog) = iter.next().await? {
            let id = dialog.peer_id().bot_api_dialog_id().unwrap_or(0);
            let name = dialog.peer.name().unwrap_or("").to_string();
            let username = dialog.peer.username().map(str::to_string);
            let unread = match &dialog.raw {
                tl::enums::Dialog::Dialog(d) => d.unread_count,
                _ => 0,
            };
            let (at, last) = match &dialog.last_message {
                Some(m) => (
                    Some(m.date().timestamp()),
                    Some(m.text().to_string()),
                ),
                None => (None, None),
            };
            mirror
                .upsert_chat(id, &name, username.as_deref(), unread, at, last.as_deref())
                .await;
            let mut inner = self.raw.iter_messages(dialog.peer_ref()).limit(limit);
            while let Some(m) = inner.next().await? {
                mirror
                    .insert_message(
                        id,
                        m.id(),
                        m.date().timestamp(),
                        m.outgoing() as i32,
m.sender().and_then(|s| s.name()).map(str::to_string).as_deref(),
                        m.text(),
                        m.media()
                            .as_ref()
                            .map(Label::kind)
                            .unwrap_or(None)
                            .as_deref(),
                    )
                    .await;
                lines += 1;
            }
            chats += 1;
        }
        Ok(Summary { chats, lines })
    }

    pub async fn search(&self, q: &str, chat: Option<&str>) -> Result<Vec<Hit>> {
        let mirror = Mirror::open().await?;
        let mut rows = vec![];
        for row in mirror.search(q, chat).await? {
            rows.push(Hit {
                chat: row.chat,
                line: Line {
                    id: row.id,
                    at: row.at,
                    out: row.out == 1,
                    who: row.who,
                    text: row.text,
                    media: None,
                },
            });
        }
        Ok(rows)
    }

    pub async fn contacts(&self) -> Result<Vec<Contact>> {
        let res = self
            .raw
            .invoke(&tl::functions::contacts::GetContacts { hash: 0 })
            .await?;
        let mut rows = vec![];
        if let tl::enums::contacts::Contacts::Contacts(page) = res {
            for raw in page.contacts {
                let tl::enums::Contact::Contact(meta) = raw;
                let who = page
                    .users
                    .iter()
                    .find(|user| match user {
                        tl::enums::User::User(u) => u.id == meta.user_id,
                        _ => false,
                    })
                    .map(|user| match user {
                        tl::enums::User::User(u) => u,
                        _ => unreachable!(),
                    });
                match who {
                    Some(user) => {
                        let name = format!(
                            "{} {}",
                            user.first_name.as_deref().unwrap_or_default(),
                            user.last_name.as_deref().unwrap_or_default()
                        )
                        .trim()
                        .to_string();
                        rows.push(Contact {
                            name,
                            handle: user.username.clone(),
                            id: meta.user_id,
                        });
                    }
                    None => rows.push(Contact {
                        name: format!("id {}", meta.user_id),
                        handle: None,
                        id: meta.user_id,
                    }),
                }
            }
        }
        Ok(rows)
    }

    pub async fn folders(&self) -> Result<Vec<Folder>> {
        let res = self
            .raw
            .invoke(&tl::functions::messages::GetDialogFilters {})
            .await?;
        let tl::enums::messages::DialogFilters::Filters(filters) = res;
        let mut rows = vec![];
        for raw in filters.filters {
            if let tl::enums::DialogFilter::Filter(folder) = raw {
                let tl::enums::TextWithEntities::Entities(text) = folder.title;
                rows.push(Folder {
                    id: folder.id,
                    title: text.text,
                });
            }
        }
        Ok(rows)
    }

    pub async fn folder_new(&self, title: &str, include: &[String]) -> Result<Folder> {
        if include.is_empty() {
            bail!("folder needs chats - pass --include <target> (repeatable)");
        }
        let mut include_peers = vec![];
        for slot in include {
            let peer = self.resolve(slot).await?;
            include_peers.push(peer.into());
        }
        let res = self
            .raw
            .invoke(&tl::functions::messages::GetDialogFilters {})
            .await?;
        let tl::enums::messages::DialogFilters::Filters(filters) = res;
        let mut next = 1;
        for raw in filters.filters {
            if let tl::enums::DialogFilter::Filter(folder) = raw {
                next = next.max(folder.id + 1);
            }
        }
        let folder = tl::enums::DialogFilter::Filter(tl::types::DialogFilter {
            contacts: false,
            non_contacts: false,
            groups: false,
            broadcasts: false,
            bots: false,
            exclude_muted: false,
            exclude_read: false,
            exclude_archived: false,
            title_noanimate: false,
            id: next,
            title: tl::enums::TextWithEntities::Entities(tl::types::TextWithEntities {
                text: title.chars().take(12).collect(),
                entities: vec![],
            }),
            emoticon: None,
            color: None,
            pinned_peers: vec![],
            include_peers,
            exclude_peers: vec![],
        });
        self.raw
            .invoke(&tl::functions::messages::UpdateDialogFilter {
                id: next,
                filter: Some(folder),
            })
            .await?;
        Ok(Folder {
            id: next,
            title: title.to_string(),
        })
    }

    pub async fn folder_rm(&self, id: i32) -> Result<Ack> {
        self.raw
            .invoke(&tl::functions::messages::UpdateDialogFilter {
                id,
                filter: None,
            })
            .await?;
        Ok(Ack {
            text: format!("folder {id} removed"),
        })
    }

    pub async fn pinned(&self, target: &str, limit: i32) -> Result<Vec<Pinned>> {
        let peer = self.resolve(target).await?;
        let res = self
            .raw
            .invoke(&tl::functions::messages::Search {
                peer: peer.into(),
                q: String::new(),
                from_id: None,
                saved_peer_id: None,
                saved_reaction: None,
                top_msg_id: None,
                filter: tl::enums::MessagesFilter::InputMessagesFilterPinned,
                min_date: 0,
                max_date: 0,
                offset_id: 0,
                add_offset: 0,
                limit,
                max_id: 0,
                min_id: 0,
                hash: 0,
            })
            .await?;
        let messages = match res {
            tl::enums::messages::Messages::Messages(page) => page.messages,
            tl::enums::messages::Messages::ChannelMessages(page) => page.messages,
            _ => vec![],
        };
        let mut rows = vec![];
        for raw in messages {
            if let tl::enums::Message::Message(msg) = raw {
                rows.push(Pinned {
                    id: msg.id,
                    text: msg.message,
                });
            }
        }
        Ok(rows)
    }

    pub async fn members(&self, target: &str) -> Result<Vec<Member>> {
        let peer = self.resolve(target).await?;
        let mut rows = vec![];
        let mut iter = self.raw.iter_participants(peer);
        while let Some(member) = iter.next().await? {
            rows.push(Member {
                name: member.user.full_name(),
            });
        }
        Ok(rows)
    }

    pub async fn kick(&self, target: &str, who: &str) -> Result<Ack> {
        let chat = self.resolve(target).await?;
        let user = self.resolve(who).await?;
        self.raw.kick_participant(chat, user).await?;
        Ok(Ack {
            text: format!("kicked {who}"),
        })
    }

    pub async fn ban(&self, target: &str, who: &str) -> Result<Ack> {
        let chat = self.resolve(target).await?;
        let user = self.resolve(who).await?;
        self.raw
            .set_banned_rights(chat, user)
            .view_messages(false)
            .send_messages(false)
            .await?;
        Ok(Ack {
            text: format!("banned {who}"),
        })
    }

    pub async fn unban(&self, target: &str, who: &str) -> Result<Ack> {
        let chat = self.resolve(target).await?;
        let user = self.resolve(who).await?;
        self.raw.set_banned_rights(chat, user).await?;
        Ok(Ack {
            text: format!("unbanned {who}"),
        })
    }

    pub async fn promote(
        &self,
        target: &str,
        who: &str,
        rank: Option<&str>,
    ) -> Result<Ack> {
        let chat = self.resolve(target).await?;
        let user = self.resolve(who).await?;
        let mut rights = self
            .raw
            .set_admin_rights(chat, user)
            .change_info(true)
            .edit_messages(true)
            .delete_messages(true)
            .ban_users(true)
            .invite_users(true)
            .pin_messages(true)
            .manage_call(true)
            .post_messages(true)
            .anonymous(false)
            .add_admins(false);
        if let Some(rank) = rank {
            rights = rights.rank(rank);
        }
        rights.await?;
        Ok(Ack {
            text: format!("promoted {who}"),
        })
    }
}