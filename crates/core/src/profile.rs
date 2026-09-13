use grammers_client::tl;
use anyhow::{anyhow, bail, Result};
use grammers_session::types::PeerKind;

use crate::{Ack, Profile};

impl crate::Client {
    pub async fn profile(&self, target: &str) -> Result<Profile> {
        let peer = self.resolve(target).await?;
        if peer.id.kind() != PeerKind::User {
            bail!("{target} is not a user");
        }
        let input: tl::enums::InputUser = (&peer).into();
        let raw: Vec<tl::enums::User> = self
            .raw
            .invoke(&tl::functions::users::GetUsers {
                id: vec![input.clone()],
            })
            .await?;
        let tl::enums::User::User(user) = raw
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("{target} not found"))?
        else {
            bail!("{target} not found");
        };
        let full = self
            .raw
            .invoke(&tl::functions::users::GetFullUser { id: input })
            .await?;
        let (about, blocked) = match full {
            tl::enums::users::UserFull::Full(page) => match page.full_user {
                tl::enums::UserFull::Full(info) => (info.about, info.blocked),
            },
        };
        Ok(Profile {
            name: super::presence::who(&user),
            handle: user.username.clone(),
            id: user.id,
            phone: user.phone.clone(),
            about,
            state: Self::state_of(user.status.as_ref()),
            blocked,
        })
    }

    pub async fn setname(&self, first: &str, last: &str) -> Result<Ack> {
        self.raw
            .invoke(&tl::functions::account::UpdateProfile {
                first_name: Some(first.to_string()),
                last_name: Some(last.to_string()),
                about: None,
            })
            .await?;
        Ok(Ack {
            text: "name updated".into(),
        })
    }

    pub async fn setbio(&self, about: &str) -> Result<Ack> {
        self.raw
            .invoke(&tl::functions::account::UpdateProfile {
                first_name: None,
                last_name: None,
                about: Some(about.to_string()),
            })
            .await?;
        Ok(Ack {
            text: "bio updated".into(),
        })
    }

    pub async fn setphoto(&self, path: &str) -> Result<Ack> {
        let file = self.raw.upload_file(path).await?;
        self.raw
            .invoke(&tl::functions::photos::UploadProfilePhoto {
                fallback: false,
                bot: None,
                file: Some(file.raw),
                video: None,
                video_start_ts: None,
                video_emoji_markup: None,
            })
            .await?;
        Ok(Ack {
            text: "photo updated".into(),
        })
    }

    pub async fn block(&self, target: &str) -> Result<Ack> {
        let peer = self.resolve(target).await?;
        self.raw
            .invoke(&tl::functions::contacts::Block {
                my_stories_from: false,
                id: peer.into(),
            })
            .await?;
        Ok(Ack {
            text: format!("blocked {target}"),
        })
    }

    pub async fn unblock(&self, target: &str) -> Result<Ack> {
        let peer = self.resolve(target).await?;
        self.raw
            .invoke(&tl::functions::contacts::Unblock {
                my_stories_from: false,
                id: peer.into(),
            })
            .await?;
        Ok(Ack {
            text: format!("unblocked {target}"),
        })
    }
}