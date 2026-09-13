use std::io::Write;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use grammers_client::client::UpdateStream;
use grammers_client::media::Media;
use grammers_client::message::{InputMessage, InputReactions};
use grammers_client::tl;
use grammers_client::{Client as Raw, SignInError};
use grammers_mtsender::SenderPool;
use grammers_session::storages::SqliteSession;
use grammers_session::types::{PeerId, PeerRef};
use grammers_session::updates::UpdatesLike;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::out::Out;

pub struct Client {
    pub raw: Raw,
    updates: Option<mpsc::UnboundedReceiver<UpdatesLike>>,
}

impl Client {
    pub async fn new(cfg: &Config) -> Result<Self> {
        let session = Arc::new(
            SqliteSession::open(&Config::session())
                .await
                .context("opening session")?,
        );
        let pool = SenderPool::new(session, cfg.api_id);
        let updates = pool.updates;
        tokio::spawn(pool.runner.run());
        Ok(Self {
            raw: Raw::new(pool.handle),
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
        let target = target.trim().trim_start_matches('@');
        if target.eq_ignore_ascii_case("saved") || target.eq_ignore_ascii_case("me") {
            return Ok(PeerId::self_user().to_ambient_ref());
        }
        if let Ok(n) = target.parse::<i64>() {
            let id = PeerId::from_bot_api_dialog_id(n).context("invalid peer id")?;
            let mut iter = self.raw.iter_dialogs();
            while let Some(dialog) = iter.next().await.context("listing dialogs")? {
                if dialog.peer_id() == id {
                    return Ok(dialog.peer_ref());
                }
            }
            bail!("no dialog with id {n} - run `termgram dialogs` or use @username");
        }
        let peer = self
            .raw
            .resolve_username(target)
            .await
            .context("resolving")?
            .ok_or_else(|| anyhow::anyhow!("user \"{target}\" not found"))?;
        peer.to_ref()
            .await
            .map_err(|err| anyhow::anyhow!("{err}"))?
            .ok_or_else(|| anyhow::anyhow!("peer \"{target}\" not usable"))
    }

    pub async fn login(&self, api_hash: &str) -> Result<()> {
        if self.raw.is_authorized().await.context("checking session")? {
            println!("already logged in");
            return Ok(());
        }
        let mut phone = String::new();
        print!("phone (with country code): ");
        std::io::stdout().flush()?;
        std::io::stdin().read_line(&mut phone)?;
        let phone = phone.trim().to_string();
        let token = self
            .raw
            .request_login_code(&phone, api_hash)
            .await
            .context("requesting code")?;
        let mut code = String::new();
        print!("code: ");
        std::io::stdout().flush()?;
        std::io::stdin().read_line(&mut code)?;
        let code = code.trim();
        let user = match self.raw.sign_in(&token, code).await {
            Ok(user) => user,
            Err(SignInError::PasswordRequired(mut pass_token)) => loop {
                if let Some(hint) = pass_token.hint() {
                    println!("hint: {hint}");
                }
                let password = rpassword::prompt_password("password: ")?;
                match self.raw.check_password(pass_token, password).await {
                    Ok(user) => break user,
                    Err(SignInError::InvalidPassword(next)) => {
                        println!("wrong password");
                        pass_token = next;
                    }
                    Err(err) => bail!("{err}"),
                }
            },
            Err(SignInError::InvalidCode) => bail!("wrong code"),
            Err(SignInError::SignUpRequired) => bail!("no account for this number"),
            Err(err) => bail!("{err}"),
        };
        println!("logged in as {}", user.full_name());
        Ok(())
    }

    pub async fn logout(&self) -> Result<()> {
        if !self.raw.is_authorized().await.context("checking session")? {
            println!("not logged in");
            return Ok(());
        }
        self.raw.sign_out().await.context("signing out")?;
        println!("logged out");
        Ok(())
    }

    pub async fn me(&self) -> Result<()> {
        if !self.raw.is_authorized().await.context("checking session")? {
            println!("not logged in - run `termgram login`");
            return Ok(());
        }
        let user = self.raw.get_me().await.context("fetching profile")?;
        println!("{}", user.full_name());
        match user.username() {
            Some(name) => println!("@{name} (id {})", user.id()),
            None => println!("id {}", user.id()),
        }
        Ok(())
    }

    pub async fn send(&self, target: &str, text: &str, reply: Option<i32>) -> Result<()> {
        let peer = self.resolve(target).await?;
        let msg = InputMessage::new().text(text).reply_to(reply);
        let sent = self.raw.send_message(peer, msg).await?;
        println!("sent {}", sent.id());
        Ok(())
    }

    pub async fn mark_as_read(&self, target: &str) -> Result<()> {
        let peer = self.resolve(target).await?;
        self.raw.mark_as_read(peer).await.context("marking read")?;
        println!("marked as read");
        Ok(())
    }

    pub async fn upload(&self, target: &str, path: &str, caption: Option<&str>) -> Result<()> {
        let peer = self.resolve(target).await?;
        let file = self.raw.upload_file(path).await?;
        let mut msg = InputMessage::new().document(file);
        if let Some(caption) = caption {
            msg = msg.text(caption);
        }
        let sent = self.raw.send_message(peer, msg).await?;
        println!("sent {}", sent.id());
        Ok(())
    }

    pub async fn download(&self, target: &str, id: Option<i32>) -> Result<()> {
        let peer = self.resolve(target).await?;
        let mut iter = self.raw.iter_messages(peer);
        if let Some(wanted) = id {
            while let Some(msg) = iter.next().await? {
                if msg.id() == wanted {
                    return match msg.media() {
                        Some(media) => self.save_media(target, wanted, media).await,
                        None => bail!("message {wanted} has no media"),
                    };
                }
            }
            bail!("message {wanted} not found");
        }
        while let Some(msg) = iter.next().await? {
            if let Some(media) = msg.media() {
                return self.save_media(target, msg.id(), media).await;
            }
        }
        bail!("no media in {target}");
    }

    async fn save_media(&self, target: &str, id: i32, media: Media) -> Result<()> {
        let dir = Config::media();
        std::fs::create_dir_all(&dir)?;
        let ext = match &media {
            Media::Document(doc) => doc
                .name()
                .and_then(|name| name.rsplit('.').next().map(str::to_string))
                .filter(|name| name.chars().count() <= 5)
                .unwrap_or_else(|| Out::ext(&media).to_string()),
            _ => Out::ext(&media).to_string(),
        };
        let path = dir.join(format!("{target}_{id}.{}", ext));
        self.raw.download_media(&media, &path).await?;
        println!("saved {}", path.display());
        Ok(())
    }

    pub async fn edit(&self, target: &str, id: i32, text: &str) -> Result<()> {
        let peer = self.resolve(target).await?;
        self.raw
            .edit_message(peer, id, InputMessage::new().text(text))
            .await?;
        println!("edited {id}");
        Ok(())
    }

    pub async fn delete(&self, target: &str, ids: Vec<i32>) -> Result<()> {
        let peer = self.resolve(target).await?;
        self.raw.delete_messages(peer, &ids).await?;
        println!("deleted {}", ids.len());
        Ok(())
    }

    pub async fn forward(&self, from: &str, to: &str, ids: Vec<i32>) -> Result<()> {
        let from = self.resolve(from).await?;
        let to = self.resolve(to).await?;
        self.raw.forward_messages(to, &ids, from).await?;
        println!("forwarded {}", ids.len());
        Ok(())
    }

    pub async fn pin(&self, target: &str, id: i32, remove: bool) -> Result<()> {
        let peer = self.resolve(target).await?;
        if remove {
            self.raw.unpin_message(peer, id).await?;
            println!("unpinned {id}");
        } else {
            self.raw.pin_message(peer, id).await?;
            println!("pinned {id}");
        }
        Ok(())
    }

    pub async fn react(&self, target: &str, id: i32, emoji: Option<&str>, remove: bool) -> Result<()> {
        let peer = self.resolve(target).await?;
        let reactions = if remove {
            InputReactions::remove()
        } else {
            emoji.unwrap_or_default().into()
        };
        self.raw.send_reactions(peer, id, reactions).await?;
        println!("reacted");
        Ok(())
    }

    pub async fn contacts(&self) -> Result<()> {
        let res = self
            .raw
            .invoke(&tl::functions::contacts::GetContacts { hash: 0 })
            .await?;
        if let tl::enums::contacts::Contacts::Contacts(page) = res {
            for raw in page.contacts {
                let tl::enums::Contact::Contact(meta) = raw;
                let who = page.users.iter().find(|user| match user {
                    tl::enums::User::User(u) => u.id == meta.user_id,
                    _ => false,
                });
                match who {
                    Some(tl::enums::User::User(u)) => {
                        let first = u.first_name.as_deref().unwrap_or_default();
                        let last = u.last_name.as_deref().unwrap_or_default();
                        let name = format!("{first} {last}").trim().to_string();
                        match &u.username {
                            Some(handle) => println!("{name} / @{handle}"),
                            None => println!("{name} / id {}", meta.user_id),
                        }
                    }
                    _ => println!("id {}", meta.user_id),
                }
            }
        }
        Ok(())
    }

    pub async fn folders(&self) -> Result<()> {
        let res = self
            .raw
            .invoke(&tl::functions::messages::GetDialogFilters {})
            .await?;
        let tl::enums::messages::DialogFilters::Filters(filters) = res;
        for raw in filters.filters {
            if let tl::enums::DialogFilter::Filter(folder) = raw {
                let tl::enums::TextWithEntities::Entities(text) = folder.title;
                println!("{} {}", folder.id, text.text);
            }
        }
        Ok(())
    }

    pub async fn folder_new(&self, title: &str, include: &[String]) -> Result<()> {
        let mut include_peers = Vec::new();
        for target in include {
            let ref_ = self.resolve(target).await?;
            let input: tl::enums::InputPeer = ref_.into();
            include_peers.push(input);
        }
        if include_peers.is_empty() {
            bail!("folder needs chats - pass --include <target> (repeatable)");
        }
        let res = self
            .raw
            .invoke(&tl::functions::messages::GetDialogFilters {})
            .await?;
        let mut next = 1;
        let tl::enums::messages::DialogFilters::Filters(filters) = res;
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
                text: title.chars().take(12).collect::<String>(),
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
        println!("folder {next} created");
        Ok(())
    }

    pub async fn folder_rm(&self, id: i32) -> Result<()> {
        self.raw
            .invoke(&tl::functions::messages::UpdateDialogFilter {
                id,
                filter: None,
            })
            .await?;
        println!("folder {id} removed");
        Ok(())
    }

    pub async fn pinned(&self, target: &str, limit: i32) -> Result<()> {
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
        for raw in messages {
            if let tl::enums::Message::Message(msg) = raw {
                println!("[{}] {}", msg.id, msg.message);
            }
        }
        Ok(())
    }

    pub async fn members(&self, target: &str) -> Result<()> {
        let peer = self.resolve(target).await?;
        let mut iter = self.raw.iter_participants(peer);
        while let Some(member) = iter.next().await? {
            println!("{}", member.user.full_name());
        }
        Ok(())
    }

    pub async fn kick(&self, target: &str, who: &str) -> Result<()> {
        let chat = self.resolve(target).await?;
        let user = self.resolve(who).await?;
        self.raw.kick_participant(chat, user).await?;
        println!("kicked {who}");
        Ok(())
    }

    pub async fn ban(&self, target: &str, who: &str) -> Result<()> {
        let chat = self.resolve(target).await?;
        let user = self.resolve(who).await?;
        self.raw
            .set_banned_rights(chat, user)
            .view_messages(false)
            .send_messages(false)
            .await?;
        println!("banned {who}");
        Ok(())
    }

    pub async fn unban(&self, target: &str, who: &str) -> Result<()> {
        let chat = self.resolve(target).await?;
        let user = self.resolve(who).await?;
        self.raw.set_banned_rights(chat, user).await?;
        println!("unbanned {who}");
        Ok(())
    }

    pub async fn promote(&self, target: &str, who: &str, rank: Option<&str>) -> Result<()> {
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
        println!("promoted {who}");
        Ok(())
    }
}