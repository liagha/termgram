use std::collections::HashSet;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use grammers_client::media::Media;
use grammers_client::message::InputMessage;
use grammers_client::tl;
use grammers_session::types::PeerKind;

use crate::presence;
use crate::{
    Ack, Chat, Contact, Done, Identity, Line, Member, PhoneContact, Sent,
};

fn nid() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

impl crate::Client {
    pub async fn wait(
        &self,
        target: &str,
        after_id: Option<i32>,
        timeout_secs: u64,
    ) -> Result<Vec<Line>> {
        let deadline =
            SystemTime::now() + Duration::from_secs(timeout_secs);
        loop {
            let lines = self.messages(target, 10).await?;
            let fresh: Vec<Line> = lines
                .into_iter()
                .filter(|l| {
                    after_id.map_or(true, |a| l.id > a)
                })
                .collect();
            if !fresh.is_empty() || SystemTime::now() >= deadline {
                return Ok(fresh);
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }

    pub async fn chat(&self, target: &str) -> Result<Chat> {
        let peer = self.resolve(target).await?;
        let res = self
            .raw
            .invoke(
                &tl::functions::messages::GetPeerDialogs {
                    peers: vec![tl::enums::InputDialogPeer::Peer(
                        tl::types::InputDialogPeer {
                            peer: peer.into(),
                        },
                    )],
                },
            )
            .await
            .context("couldn't fetch the dialog")?;
        let tl::enums::messages::PeerDialogs::Dialogs(page) = res;
        let d = match page
            .dialogs
            .into_iter()
            .next()
            .context("no dialog")?
        {
            tl::enums::Dialog::Dialog(d) => d,
            _ => bail!("unexpected dialog variant"),
        };
        let bare = peer.id.bare_id().unwrap_or(0);
        let mut name = String::new();
        let mut title = None;
        let mut about = String::new();
        for u in &page.users {
            if let tl::enums::User::User(u) = u {
                if u.id == bare {
                    name = presence::who(&u);
                    break;
                }
            }
        }
        for c in &page.chats {
            let cid = c.id();
            let (t, a) = match c {
                tl::enums::Chat::Chat(c) => (
                    c.title.clone(),
                    String::new(),
                ),
                tl::enums::Chat::Channel(c) => {
                    (c.title.clone(), String::new())
                }
                _ => continue,
            };
            if cid == bare || name.is_empty() {
                title = Some(t.clone());
                about = a;
                name = t;
                break;
            }
        }
        Ok(Chat {
            id: peer.id.bot_api_dialog_id().unwrap_or(0),
            name,
            title,
            about,
            unread: d.unread_count,
        })
    }

    pub async fn full(
        &self,
        target: &str,
    ) -> Result<Chat> {
        let peer = self.resolve(target).await?;
        let bare = peer.id.bare_id().unwrap_or(0);
        let mut name = String::new();
        let mut about = String::new();
        let mut unread = 0;
        let res = self
            .raw
            .invoke(
                &tl::functions::messages::GetPeerDialogs {
                    peers: vec![tl::enums::InputDialogPeer::Peer(
                        tl::types::InputDialogPeer {
                            peer: peer.into(),
                        },
                    )],
                },
            )
            .await
            .ok();
        if let Some(
            tl::enums::messages::PeerDialogs::Dialogs(page),
        ) = res
        {
            if let Some(tl::enums::Dialog::Dialog(d)) =
                page.dialogs.into_iter().next()
            {
                unread = d.unread_count;
            }
            for u in &page.users {
                if let tl::enums::User::User(u) = u {
                    if u.id == bare {
                        name = presence::who(&u);
                        break;
                    }
                }
            }
            for c in &page.chats {
                let (t, a) = match c {
                    tl::enums::Chat::Chat(c) => (
                        c.title.clone(),
                        String::new(),
                    ),
                    tl::enums::Chat::Channel(c) => (
                        c.title.clone(),
                        String::new(),
                    ),
                    _ => continue,
                };
                if c.id() == bare || name.is_empty() {
                    name = t;
                    about = a;
                    break;
                }
            }
        }
        if matches!(
            peer.id.kind(),
            PeerKind::Channel | PeerKind::Chat
        ) {
            if let Ok(
                tl::enums::messages::ChatFull::Full(page),
            ) = self
                .raw
                .invoke(
                    &tl::functions::messages::GetFullChat {
                        chat_id: bare,
                    },
                )
                .await
            {
                about = page.full_chat.about();
                for c in &page.chats {
                    if c.id() == bare {
                        match c {
                            tl::enums::Chat::Chat(v) => {
                                name = v.title.clone()
                            }
                            tl::enums::Chat::Channel(v) => {
                                name = v.title.clone()
                            }
                            _ => {}
                        }
                        break;
                    }
                }
            }
        }
        Ok(Chat {
            id: peer.id
                .bot_api_dialog_id()
                .unwrap_or(0),
            title: Some(name.clone()),
            name,
            about,
            unread,
        })
    }

    pub async fn resolve_alias(
        &self,
        target: &str,
    ) -> Result<Identity> {
        let c = self.chat(target).await?;
        Ok(Identity {
            name: c.name,
            handle: None,
            id: c.id,
        })
    }

    pub async fn mute(
        &self,
        target: &str,
        forever: bool,
    ) -> Result<Ack> {
        let peer = self.resolve(target).await?;
        let until = if forever {
            Some(i32::MAX)
        } else {
            Some(0)
        };
        self.raw
            .invoke(
                &tl::functions::account::UpdateNotifySettings {
                    peer: tl::enums::InputNotifyPeer::Peer(
                        tl::types::InputNotifyPeer {
                            peer: peer.into(),
                        },
                    ),
                    settings:
                        tl::enums::InputPeerNotifySettings::Settings(
                            tl::types::InputPeerNotifySettings {
                                show_previews: None,
                                silent: None,
                                mute_until: until,
                                sound: None,
                                stories_muted: None,
                                stories_hide_sender: None,
                                stories_sound: None,
                            },
                        ),
                },
            )
            .await?;
        Ok(Ack {
            text: if forever {
                "muted"
            } else {
                "unmuted"
            }
            .into(),
        })
    }

    pub async fn admins(
        &self,
        target: &str,
    ) -> Result<Vec<Member>> {
        let peer = self.resolve(target).await?;
        let res = self
            .raw
            .invoke(
                &tl::functions::channels::GetParticipants {
                    channel: (&peer).into(),
                    filter:
                        tl::enums::ChannelParticipantsFilter
                            ::ChannelParticipantsAdmins,
                    offset: 0,
                    limit: 200,
                    hash: 0,
                },
            )
            .await?;
        let tl::enums::channels::ChannelParticipants::Participants(
            page,
        ) = res
        else {
            return Ok(vec![]);
        };
        Ok(page
            .users
            .iter()
            .filter_map(|u| match u {
                tl::enums::User::User(u) => {
                    Some(Member { name: presence::who(&u) })
                }
                _ => None,
            })
            .collect())
    }

    pub async fn banned(
        &self,
        target: &str,
    ) -> Result<Vec<Member>> {
        let peer = self.resolve(target).await?;
        let res = self
            .raw
            .invoke(
                &tl::functions::channels::GetParticipants {
                    channel: (&peer).into(),
                    filter:
                        tl::enums::ChannelParticipantsFilter
                            ::ChannelParticipantsBanned(
                            tl::types::ChannelParticipantsBanned {
                                q: String::new(),
                            },
                        ),
                    offset: 0,
                    limit: 200,
                    hash: 0,
                },
            )
            .await?;
        let tl::enums::channels::ChannelParticipants::Participants(
            page,
        ) = res
        else {
            return Ok(vec![]);
        };
        Ok(page
            .users
            .iter()
            .filter_map(|u| match u {
                tl::enums::User::User(u) => {
                    Some(Member { name: presence::who(&u) })
                }
                _ => None,
            })
            .collect())
    }

    pub async fn demote(
        &self,
        target: &str,
        who: &str,
    ) -> Result<Ack> {
        let peer = self.resolve(target).await?;
        let user = self.resolve(who).await?;
        let channel: tl::enums::InputChannel =
            (&peer).into();
        let input_user: tl::enums::InputUser =
            (&user).into();
        self.raw
            .invoke(
                &tl::functions::channels::EditAdmin {
                    channel,
                    user_id: input_user,
                    admin_rights:
                        tl::enums::ChatAdminRights::Rights(
                            tl::types::ChatAdminRights {
                                change_info: false,
                                post_messages: false,
                                edit_messages: false,
                                delete_messages: false,
                                ban_users: false,
                                invite_users: false,
                                pin_messages: false,
                                add_admins: false,
                                anonymous: false,
                                manage_call: false,
                                other: false,
                                manage_topics: false,
                                post_stories: false,
                                edit_stories: false,
                                delete_stories: false,
                                manage_direct_messages: false,
                                manage_ranks: false,
                            },
                        ),
                    rank: None,
                },
            )
            .await?;
        Ok(Ack {
            text: format!("demoted {who}"),
        })
    }

    pub async fn invite(
        &self,
        target: &str,
        users: &[String],
    ) -> Result<Ack> {
        let peer = self.resolve(target).await?;
        let input_users = self.resolve_many(users).await?;
        if users.is_empty() {
            bail!("no users to invite");
        }
        match peer.id.kind() {
            PeerKind::Chat => {
                let chat_id = peer.id.bare_id().unwrap_or(0);
                for user in &input_users {
                    self.raw
                        .invoke(&tl::functions::messages::AddChatUser {
                            chat_id,
                            user_id: user.clone(),
                            fwd_limit: 50,
                        })
                        .await?;
                }
            }
            _ => {
                self.raw
                    .invoke(
                        &tl::functions::channels::InviteToChannel {
                            channel: (&peer).into(),
                            users: input_users,
                        },
                    )
                    .await?;
            }
        }
        Ok(Ack {
            text: "invited".into(),
        })
    }

    pub async fn create_group(
        &self,
        title: &str,
        users: &[String],
    ) -> Result<Sent> {
        let input_users = self.resolve_many(users).await?;
        self.raw
            .invoke(
                &tl::functions::messages::CreateChat {
                    users: input_users,
                    title: title.to_string(),
                    ttl_period: None,
                },
            )
            .await?;
        Ok(Sent { ids: vec![] })
    }

    pub async fn create_channel(
        &self,
        title: &str,
        about: &str,
        users: &[String],
    ) -> Result<Sent> {
        let res = self
            .raw
            .invoke(
                &tl::functions::channels::CreateChannel {
                    broadcast: false,
                    megagroup: true,
                    for_import: false,
                    forum: false,
                    title: title.to_string(),
                    about: about.to_string(),
                    geo_point: None,
                    address: None,
                    ttl_period: None,
                },
            )
            .await?;
        let id = crate::poll::sent_id(&res)
            .unwrap_or(0);
        if !users.is_empty() {
            let input = self.resolve_many(users).await?;
            let peer =
                tl::enums::InputPeer::Channel(
                    tl::types::InputPeerChannel {
                        channel_id: id as i64,
                        access_hash: 0,
                    },
                );
            if let tl::enums::InputPeer::Channel(ch) =
                &peer
            {
                self.raw
                    .invoke(
                        &tl::functions::channels::InviteToChannel {
                            channel: tl::enums::InputChannel::Channel(
                                tl::types::InputChannel {
                                    channel_id: ch.channel_id,
                                    access_hash: ch.access_hash,
                                },
                            ),
                            users: input,
                        },
                    )
                    .await?;
            }
        }
        Ok(Sent {
            ids: vec![id],
        })
    }

    pub async fn leave(
        &self,
        target: &str,
    ) -> Result<Ack> {
        let peer = self.resolve(target).await?;
        match peer.id.kind() {
            PeerKind::Chat => {
                self.raw
                    .invoke(
                        &tl::functions::messages::DeleteChatUser {
                            revoke_history: false,
                            chat_id: peer.id
                                .bare_id()
                                .unwrap_or(0),
                            user_id:
                                tl::enums::InputUser::UserSelf,
                        },
                    )
                    .await?;
            }
            _ => {
                self.raw
                    .invoke(
                        &tl::functions::channels::LeaveChannel {
                            channel: (&peer).into(),
                        },
                    )
                    .await?;
            }
        }
        Ok(Ack {
            text: "left".into(),
        })
    }

    pub async fn join(
        &self,
        link: &str,
    ) -> Result<Ack> {
        let hash = link
            .split('/')
            .last()
            .unwrap_or(link);
        self.raw
            .invoke(
                &tl::functions::messages::ImportChatInvite {
                    hash: hash.to_string(),
                },
            )
            .await
            .context("couldn't join via link")?;
        Ok(Ack {
            text: "joined".into(),
        })
    }

    pub async fn edit_title(
        &self,
        target: &str,
        title: &str,
    ) -> Result<Ack> {
        let peer = self.resolve(target).await?;
        match peer.id.kind() {
            PeerKind::Chat => {
                self.raw
                    .invoke(
                        &tl::functions::messages::EditChatTitle {
                            chat_id: peer.id
                                .bare_id()
                                .unwrap_or(0),
                            title: title.to_string(),
                        },
                    )
                    .await?;
            }
            _ => {
                self.raw
                    .invoke(
                        &tl::functions::channels::EditTitle {
                            channel: (&peer).into(),
                            title: title.to_string(),
                        },
                    )
                    .await?;
            }
        }
        Ok(Ack {
            text: format!("title set to {title}"),
        })
    }

    pub async fn edit_about(
        &self,
        target: &str,
        about: &str,
    ) -> Result<Ack> {
        let peer = self.resolve(target).await?;
        self.raw
            .invoke(
                &tl::functions::messages::EditChatAbout {
                    peer: peer.into(),
                    about: about.to_string(),
                },
            )
            .await?;
        Ok(Ack {
            text: "about updated".into(),
        })
    }

    pub async fn edit_photo(
        &self,
        target: &str,
        path: &str,
    ) -> Result<Ack> {
        let peer = self.resolve(target).await?;
        let uploaded =
            self.raw.upload_file(path).await?;
        let photo =
            tl::enums::InputChatPhoto::InputChatUploadedPhoto(
                tl::types::InputChatUploadedPhoto {
                    file: Some(uploaded.raw),
                    video: None,
                    video_start_ts: None,
                    video_emoji_markup: None,
                },
            );
        match peer.id.kind() {
            PeerKind::Chat => {
                self.raw
                    .invoke(
                        &tl::functions::messages::EditChatPhoto {
                            chat_id: peer.id
                                .bare_id()
                                .unwrap_or(0),
                            photo,
                        },
                    )
                    .await?;
            }
            _ => {
                self.raw
                    .invoke(
                        &tl::functions::channels::EditPhoto {
                            channel: (&peer).into(),
                            photo,
                        },
                    )
                    .await?;
            }
        }
        Ok(Ack {
            text: "photo updated".into(),
        })
    }

    pub async fn invite_link(
        &self,
        target: &str,
    ) -> Result<Ack> {
        let peer = self.resolve(target).await?;
        let res = self
            .raw
            .invoke(
                &tl::functions::messages::ExportChatInvite {
                    peer: peer.into(),
                    legacy_revoke_permanent: false,
                    request_needed: false,
                    expire_date: None,
                    usage_limit: None,
                    title: None,
                    subscription_pricing: None,
                },
            )
            .await?;
        match res {
            tl::enums::ExportedChatInvite::ChatInviteExported(
                i,
            ) => Ok(Ack { text: i.link }),
            _ => bail!("unexpected response"),
        }
    }

    pub async fn flush(
        &self,
        target: &str,
        revoke: bool,
    ) -> Result<Ack> {
        let peer = self.resolve(target).await?;
        self.raw
            .invoke(
                &tl::functions::messages::DeleteHistory {
                    just_clear: false,
                    revoke,
                    peer: peer.into(),
                    max_id: 0,
                    min_date: None,
                    max_date: None,
                },
            )
            .await?;
        Ok(Ack {
            text: "history cleared".into(),
        })
    }

    pub async fn send_sticker(
        &self,
        target: &str,
        path: &str,
    ) -> Result<Sent> {
        let peer = self.resolve(target).await?;
        let uploaded =
            self.raw.upload_file(path).await?;
        let msg = InputMessage::new().media(
            tl::enums::InputMedia::UploadedDocument(
                tl::types::InputMediaUploadedDocument {
                    nosound_video: false,
                    force_file: false,
                    spoiler: false,
                    file: uploaded.raw,
                    thumb: None,
                    mime_type: "image/webp".into(),
                    attributes: vec![
                        tl::enums::DocumentAttribute::Sticker(
                            tl::types::DocumentAttributeSticker {
                                mask: false,
                                alt: String::new(),
                                stickerset:
                                    tl::enums::InputStickerSet::Empty,
                                mask_coords: None,
                            },
                        ),
                    ],
                    stickers: None,
                    video_cover: None,
                    video_timestamp: None,
                    ttl_seconds: None,
                },
            ),
        );
        let res = self
            .raw
            .send_message(peer, msg)
            .await?;
        Ok(Sent {
            ids: vec![res.id()],
        })
    }

    pub async fn sticker_sets(
        &self,
    ) -> Result<Vec<Identity>> {
        let res = self
            .raw
            .invoke(
                &tl::functions::messages::GetAllStickers {
                    hash: 0,
                },
            )
            .await?;
        let tl::enums::messages::AllStickers::Stickers(
            page,
        ) = res
        else {
            return Ok(vec![]);
        };
        Ok(page
            .sets
            .into_iter()
            .filter_map(|s| match s {
                tl::enums::StickerSet::Set(s) => {
                    Some(Identity {
                        name: s.title,
                        handle: Some(s.short_name),
                        id: s.id,
                    })
                }
            })
            .collect())
    }

    pub async fn media_info(
        &self,
        target: &str,
        id: i32,
    ) -> Result<Ack> {
        let lines = self.messages(target, 100).await?;
        let m = lines
            .iter()
            .find(|l| l.id == id)
            .context("message not found")?;
        let kind = m
            .media
            .as_deref()
            .unwrap_or("none");
        Ok(Ack {
            text: format!("media: {kind}"),
        })
    }

    pub async fn unpin_all(
        &self,
        target: &str,
    ) -> Result<Ack> {
        let peer = self.resolve(target).await?;
        self.raw
            .invoke(
&tl::functions::messages::UnpinAllMessages {
                peer: peer.into(),
                top_msg_id: None,
                saved_peer_id: None,
            },
            )
            .await?;
        Ok(Ack {
            text: "all unpinned".into(),
        })
    }

    pub async fn context(
        &self,
        target: &str,
        id: i32,
        size: i32,
    ) -> Result<Vec<Line>> {
        let peer = self.resolve(target).await?;
        let raw = self
            .raw
            .invoke(
                &tl::functions::messages::GetHistory {
                    peer: peer.into(),
                    offset_id: id + 1,
                    offset_date: 0,
                    add_offset: -size,
                    limit: size * 2 + 1,
                    max_id: 0,
                    min_id: 0,
                    hash: 0,
                },
            )
            .await?;
        let tl::enums::messages::Messages::Messages(
            page,
        ) = raw
        else {
            return Ok(vec![]);
        };
        let mut rows: Vec<Line> = page
            .messages
            .into_iter()
            .filter_map(|m| {
                if let tl::enums::Message::Message(m) = m {
                    Some(Line {
                        id: m.id,
                        at: m.date as i64,
                        out: m.out,
                        who: None,
                        text: m.message.clone(),
                        media: m
                            .media
                            .clone()
                            .and_then(Media::from_raw)
                            .and_then(|mm| {
                                crate::Label::kind(&mm)
                            })
                            .map(str::to_string),
                    })
                } else {
                    None
                }
            })
            .collect();
        rows.reverse();
        Ok(rows)
    }

    pub async fn user(
        &self,
        target: &str,
    ) -> Result<crate::Profile> {
        self.profile(target).await
    }

    pub async fn del_photo(
        &self,
        target: &str,
    ) -> Result<Ack> {
        let peer = self.resolve(target).await?;
        match peer.id.kind() {
            PeerKind::Chat => {
                self.raw
                    .invoke(
                        &tl::functions::messages::EditChatPhoto {
                            chat_id: peer.id
                                .bare_id()
                                .unwrap_or(0),
                            photo:
                                tl::enums::InputChatPhoto::Empty,
                        },
                    )
                    .await?;
            }
            _ => {
                self.raw
                    .invoke(
                        &tl::functions::channels::EditPhoto {
                            channel: (&peer).into(),
                            photo:
                                tl::enums::InputChatPhoto::Empty,
                        },
                    )
                    .await?;
            }
        }
        Ok(Ack {
            text: "photo removed".into(),
        })
    }

    pub async fn add_contact(
        &self,
        phone: &str,
        first_name: &str,
        last_name: &str,
    ) -> Result<Ack> {
        let res = self
            .raw
            .invoke(
                &tl::functions::contacts::ImportContacts {
                    contacts: vec![
                        tl::enums::InputContact::InputPhoneContact(
                            tl::types::InputPhoneContact {
                                client_id: nid(),
                                phone: phone.to_string(),
                                first_name:
                                    first_name.to_string(),
                                last_name:
                                    last_name.to_string(),
                                note: None,
                            },
                        ),
                    ],
                },
            )
            .await?;
        let tl::enums::contacts::ImportedContacts::Contacts(
            page,
        ) = res;
        Ok(Ack {
            text: format!(
                "imported {} contact(s)",
                page.imported.len()
            ),
        })
    }

    pub async fn delete_contact(
        &self,
        target: &str,
    ) -> Result<Ack> {
        let peer = self.resolve(target).await?;
        let user: tl::enums::InputUser = (&peer).into();
        self.raw
            .invoke(&tl::functions::contacts::DeleteContacts {
                id: vec![user],
            })
            .await?;
        Ok(Ack {
            text: "contact deleted".into(),
        })
    }

    pub async fn export_contacts(
        &self,
    ) -> Result<Vec<Contact>> {
        self.contacts().await
    }

    pub async fn import_contacts(
        &self,
        contacts: &[PhoneContact],
    ) -> Result<Done> {
        let res = self
            .raw
            .invoke(
                &tl::functions::contacts::ImportContacts {
                    contacts: contacts
                        .iter()
                        .map(|c| {
                            tl::enums::InputContact::InputPhoneContact(
                                tl::types::InputPhoneContact {
                                    client_id: nid(),
                                    phone: c.phone.clone(),
                                    first_name: c.first.clone(),
                                    last_name: c.last.clone(),
                                    note: None,
                                },
                            )
                        })
                        .collect(),
                },
            )
            .await?;
        let tl::enums::contacts::ImportedContacts::Contacts(
            page,
        ) = res;
        Ok(Done {
            n: page.imported.len(),
        })
    }

    pub async fn block_list(
        &self,
    ) -> Result<Vec<Member>> {
        let res = self
            .raw
            .invoke(
                &tl::functions::contacts::GetBlocked {
                    my_stories_from: false,
                    offset: 0,
                    limit: 200,
                },
            )
            .await?;
        let mut ids = HashSet::new();
        for b in res.blocked() {
            let tl::enums::PeerBlocked::Blocked(b) = b;
            if let tl::enums::Peer::User(u) = b.peer_id {
                ids.insert(u.user_id);
            }
        }
        Ok(res
            .users()
            .into_iter()
            .filter_map(|u| match u {
                tl::enums::User::User(u)
                    if ids.contains(&u.id) =>
                {
                    Some(Member {
                        name: presence::who(&u),
                    })
                }
                _ => None,
            })
            .collect())
    }

    async fn resolve_many(
        &self,
        targets: &[String],
    ) -> Result<Vec<tl::enums::InputUser>> {
        let mut v = Vec::new();
        for t in targets {
            let p = self.resolve(t).await?;
            v.push(p.into());
        }
        Ok(v)
    }
}
