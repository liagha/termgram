use grammers_client::tl;
use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Local, Utc};
use grammers_session::types::PeerKind;

use crate::{Ack, Status};

impl crate::Client {
    pub async fn typing(&self, target: &str) -> Result<Ack> {
        let peer = self.resolve(target).await?;
        self.raw
            .invoke(&tl::functions::messages::SetTyping {
                peer: peer.into(),
                top_msg_id: None,
                action: tl::enums::SendMessageAction::SendMessageTypingAction,
            })
            .await?;
        Ok(Ack { text: "typing".into() })
    }

    pub async fn status(&self, target: &str) -> Result<Status> {
        let peer = self.resolve(target).await?;
        if peer.id.kind() != PeerKind::User {
            bail!("{target} is not a user");
        }
        let res: Vec<tl::enums::User> = self
            .raw
            .invoke(&tl::functions::users::GetUsers {
                id: vec![(&peer).into()],
            })
            .await?;
        let tl::enums::User::User(user) = res
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("{target} not found"))?
        else {
            bail!("{target} not found");
        };
        Ok(Status {
            who: who(&user),
            state: Self::state_of(user.status.as_ref()),
        })
    }

    pub(crate) fn state_of(status: Option<&tl::enums::UserStatus>) -> String {
        match status {
            Some(tl::enums::UserStatus::Online(_)) => "online".into(),
            Some(tl::enums::UserStatus::Offline(state)) => {
                format!("offline since {}", stamp(state.was_online as i64))
            }
            Some(tl::enums::UserStatus::Recently(_)) => "recently".into(),
            Some(tl::enums::UserStatus::LastWeek(_)) => "last week".into(),
            Some(tl::enums::UserStatus::LastMonth(_)) => "last month".into(),
            _ => "hidden".into(),
        }
    }
}

fn stamp(ts: i64) -> String {
    DateTime::<Utc>::from_timestamp(ts, 0)
        .map(|time| time.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| ts.to_string())
}

pub(crate) fn who(user: &tl::types::User) -> String {
    format!(
        "{} {}",
        user.first_name.as_deref().unwrap_or_default(),
        user.last_name.as_deref().unwrap_or_default()
    )
    .trim()
    .to_string()
}