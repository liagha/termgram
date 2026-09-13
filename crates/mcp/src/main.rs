use rmcp::{
    handler::server::wrapper::Parameters,
    schemars, tool, tool_router,
    transport::stdio,
    ServiceExt,
};
use serde::Deserialize;
use std::future::Future;

use termgram::Client;

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct Empty {}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TargetArgs {
    pub target: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MessageArgs {
    pub target: String,
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PollArgs {
    pub target: String,
    pub question: String,
    pub options: Vec<String>,
    #[serde(default)]
    pub anon: bool,
    #[serde(default)]
    pub multi: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TopicsArgs {
    pub target: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WatchArgs {
    pub target: Option<String>,
    pub limit: Option<i32>,
    #[serde(default)]
    pub unread: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SignArgs {
    pub target: String,
    pub id: i32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SyncArgs {
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchArgs {
    pub query: String,
    pub chat: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UploadArgs {
    pub target: String,
    pub path: String,
    pub caption: Option<termgram::Text>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DownloadArgs {
    pub target: String,
    pub id: Option<i32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EditArgs {
    pub target: String,
    pub id: i32,
    pub text: termgram::Text,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteArgs {
    pub target: String,
    pub ids: Vec<i32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ForwardArgs {
    pub from: String,
    pub to: String,
    pub ids: Vec<i32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PinArgs {
    pub target: String,
    pub id: i32,
    pub remove: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReactArgs {
    pub target: String,
    pub id: Option<i32>,
    pub emoji: Option<String>,
    pub remove: bool,
    #[serde(default)]
    pub big: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReactionArgs {
    pub target: String,
    pub id: i32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FolderNewArgs {
    pub title: String,
    pub include: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FolderRmArgs {
    pub id: i32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UserArgs {
    pub target: String,
    pub user: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PromoteArgs {
    pub target: String,
    pub user: String,
    pub rank: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ScheduleArgs {
    pub target: String,
    pub text: termgram::Text,
    pub at: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DraftArgs {
    pub target: String,
    pub text: Option<termgram::Text>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NameArgs {
    pub first: String,
    pub last: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BioArgs {
    pub about: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PhotoArgs {
    pub path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchallArgs {
    pub query: String,
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchinArgs {
    pub target: String,
    pub query: String,
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AlbumArgs {
    pub target: String,
    pub paths: Vec<String>,
    pub caption: Option<termgram::Text>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct VoiceArgs {
    pub target: String,
    pub path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CachedArgs {
    pub target: String,
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExportArgs {
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ImportArgs {
    pub path: String,
}

struct Server {
    client: Client,
}

impl Server {
    fn json<T>(value: T) -> String
    where
        T: serde::Serialize,
    {
        match serde_json::to_string(&value) {
            Ok(text) => text,
            Err(err) => format!("error: {err}"),
        }
    }

    async fn run<T, F>(&self, future: F) -> String
    where
        T: serde::Serialize,
        F: Future<Output = anyhow::Result<T>>,
    {
        match future.await {
            Ok(value) => Self::json(value),
            Err(err) => format!("error: {err}"),
        }
    }
}

#[tool_router(server_handler)]
impl Server {
    #[tool(description = "Check login state; interactive OTP login only via `termgram login` in a terminal")]
    async fn login(&self, _: Parameters<Empty>) -> String {
        match self.client.me().await {
            Ok(me) => Self::json(me),
            Err(_) => "not logged in - run `termgram login` in a terminal first".to_string(),
        }
    }

    #[tool(description = "Log out the current session")]
    async fn logout(&self, _: Parameters<Empty>) -> String {
        self.run(self.client.logout()).await
    }

    #[tool(description = "Show the logged-in account identity")]
    async fn me(&self, _: Parameters<Empty>) -> String {
        self.run(self.client.me()).await
    }

    #[tool(description = "List chats with unread count and last message preview")]
    async fn dialogs(&self, _: Parameters<Empty>) -> String {
        self.run(self.client.dialogs()).await
    }

    #[tool(description = "Show recent messages in a chat, newest last")]
    async fn messages(&self, Parameters(args): Parameters<MessageArgs>) -> String {
        let limit = args.limit.unwrap_or(20) as usize;
        self.run(self.client.messages(&args.target, limit)).await
    }

    #[tool(description = "Create a poll in a chat with 2 to 10 answer options")]
    async fn poll(&self, Parameters(args): Parameters<PollArgs>) -> String {
        self.run(
            self.client
                .poll(&args.target, &args.question, &args.options, args.anon, args.multi),
        )
        .await
    }

    #[tool(description = "List forum topics in a forum-enabled chat")]
    async fn topics(&self, Parameters(args): Parameters<TopicsArgs>) -> String {
        self.run(self.client.topics(&args.target)).await
    }

    #[tool(description = "Snapshot the last messages of a chat (or the latest dialogs when target is omitted, or only chats with unread messages when unread is set); does not follow a live stream")]
    async fn watch(&self, Parameters(args): Parameters<WatchArgs>) -> String {
        match args.target {
            Some(t) => self
                .run(self.client.messages(&t, args.limit.unwrap_or(20).max(1) as usize))
                .await,
            None => {
                if args.unread {
                    self.run(self.client.unread()).await
                } else {
                    self.run(self.client.dialogs()).await
                }
            }
        }
    }

    #[tool(description = "Send text and/or files to a chat (one file sends a photo, document, or voice note by type; several send an album). text.format: plain, markdown, or html. text.dates: exact date text in the message to render as tappable chips. at: future unix timestamp to schedule")]
    async fn send(&self, Parameters(args): Parameters<termgram::Send>) -> String {
        self.run(self.client.send(&args)).await
    }

    #[tool(description = "Mark a chat as read")]
    async fn read(&self, Parameters(args): Parameters<TargetArgs>) -> String {
        self.run(self.client.mark_as_read(&args.target)).await
    }

    #[tool(description = "Sync dialog list and recent messages into the local search mirror")]
    async fn sync(&self, Parameters(args): Parameters<SyncArgs>) -> String {
        let limit = args.limit.unwrap_or(200) as usize;
        self.run(self.client.sync(limit)).await
    }

    #[tool(description = "Search the local sync mirror for messages")]
    async fn search(&self, Parameters(args): Parameters<SearchArgs>) -> String {
        self.run(self.client.search(&args.query, args.chat.as_deref()))
            .await
    }

    #[tool(description = "Upload a file to a chat (sends a photo, document, or voice note by type; caption is rich text)")]
    async fn upload(&self, Parameters(args): Parameters<UploadArgs>) -> String {
        let send = termgram::Send {
            target: args.target,
            text: args.caption,
            files: vec![args.path],
            ..Default::default()
        };
        self.run(self.client.send(&send)).await
    }

    #[tool(description = "Download media from a chat message to the local media dir")]
    async fn download(&self, Parameters(args): Parameters<DownloadArgs>) -> String {
        self.run(self.client.download(&args.target, args.id)).await
    }

    #[tool(description = "Edit a message you sent (format: plain, md, or html; dates: exact date text in the message to render as tappable chips)")]
    async fn edit(&self, Parameters(args): Parameters<EditArgs>) -> String {
        self.run(self.client.edit(&args.target, args.id, &args.text))
            .await
    }

    #[tool(description = "Delete messages from a chat")]
    async fn delete(&self, Parameters(args): Parameters<DeleteArgs>) -> String {
        self.run(self.client.delete(&args.target, &args.ids)).await
    }

    #[tool(description = "Forward messages between chats")]
    async fn forward(&self, Parameters(args): Parameters<ForwardArgs>) -> String {
        self.run(self.client.forward(&args.from, &args.to, &args.ids))
            .await
    }

    #[tool(description = "Pin or unpin a message in a chat")]
    async fn pin(&self, Parameters(args): Parameters<PinArgs>) -> String {
        self.run(self.client.pin(&args.target, args.id, args.remove))
            .await
    }

    #[tool(description = "React to a message with an emoji (omit id to use the last message in the chat), or remove the reaction")]
    async fn react(&self, Parameters(args): Parameters<ReactArgs>) -> String {
        self.run(
            self.client
                .react(&args.target, args.id, args.emoji.as_deref(), args.remove, args.big),
        )
        .await
    }

    #[tool(description = "List the reactions on a message and how many times each was used")]
    async fn reactions(&self, Parameters(args): Parameters<ReactionArgs>) -> String {
        self.run(self.client.reactions(&args.target, args.id)).await
    }

    #[tool(description = "List contacts of the account")]
    async fn contacts(&self, _: Parameters<Empty>) -> String {
        self.run(self.client.contacts()).await
    }

    #[tool(description = "List chat folders")]
    async fn folders(&self, _: Parameters<Empty>) -> String {
        self.run(self.client.folders()).await
    }

    #[tool(description = "Create a chat folder with the given chats")]
    async fn folder_new(&self, Parameters(args): Parameters<FolderNewArgs>) -> String {
        self.run(self.client.folder_new(&args.title, &args.include))
            .await
    }

    #[tool(description = "Delete a chat folder")]
    async fn folder_rm(&self, Parameters(args): Parameters<FolderRmArgs>) -> String {
        self.run(self.client.folder_rm(args.id)).await
    }

    #[tool(description = "List pinned messages of a chat")]
    async fn pinned(&self, Parameters(args): Parameters<MessageArgs>) -> String {
        let limit = args.limit.unwrap_or(20);
        self.run(self.client.pinned(&args.target, limit)).await
    }

    #[tool(description = "List members of a group or channel")]
    async fn members(&self, Parameters(args): Parameters<TargetArgs>) -> String {
        self.run(self.client.members(&args.target)).await
    }

    #[tool(description = "Kick a user from a group")]
    async fn kick(&self, Parameters(args): Parameters<UserArgs>) -> String {
        self.run(self.client.kick(&args.target, &args.user)).await
    }

    #[tool(description = "Ban a user in a channel")]
    async fn ban(&self, Parameters(args): Parameters<UserArgs>) -> String {
        self.run(self.client.ban(&args.target, &args.user)).await
    }

    #[tool(description = "Unban a user in a channel")]
    async fn unban(&self, Parameters(args): Parameters<UserArgs>) -> String {
        self.run(self.client.unban(&args.target, &args.user)).await
    }

    #[tool(description = "Promote a user to admin in a channel")]
    async fn promote(&self, Parameters(args): Parameters<PromoteArgs>) -> String {
        self.run(
            self.client
                .promote(&args.target, &args.user, args.rank.as_deref()),
        )
        .await
    }

    #[tool(description = "Broadcast a typing action in a chat")]
    async fn typing(&self, Parameters(args): Parameters<TargetArgs>) -> String {
        self.run(self.client.typing(&args.target)).await
    }

    #[tool(description = "Show a user's online status")]
    async fn status(&self, Parameters(args): Parameters<TargetArgs>) -> String {
        self.run(self.client.status(&args.target)).await
    }

    #[tool(description = "Schedule a rich text message for a future unix timestamp")]
    async fn schedule(&self, Parameters(args): Parameters<ScheduleArgs>) -> String {
        let send = termgram::Send {
            target: args.target,
            text: Some(args.text),
            at: Some(args.at as u64),
            ..Default::default()
        };
        self.run(self.client.send(&send)).await
    }

    #[tool(description = "List scheduled messages of a chat")]
    async fn scheduled(&self, Parameters(args): Parameters<TargetArgs>) -> String {
        self.run(self.client.scheduled(&args.target)).await
    }

    #[tool(description = "Cancel a scheduled message of a chat")]
    async fn cancel(&self, Parameters(args): Parameters<SignArgs>) -> String {
        self.run(self.client.cancel(&args.target, args.id)).await
    }

    #[tool(description = "Save or clear (empty text) a draft in a chat — the draft text is rich (format, dates)")]
    async fn draft(&self, Parameters(args): Parameters<DraftArgs>) -> String {
        self.run(self.client.draft(&args.target, args.text.as_ref())).await
    }

    #[tool(description = "List all saved drafts")]
    async fn drafts(&self, _: Parameters<Empty>) -> String {
        self.run(self.client.drafts()).await
    }

    #[tool(description = "Show a user's full profile")]
    async fn profile(&self, Parameters(args): Parameters<TargetArgs>) -> String {
        self.run(self.client.profile(&args.target)).await
    }

    #[tool(description = "Set your account first and last name")]
    async fn setname(&self, Parameters(args): Parameters<NameArgs>) -> String {
        self.run(self.client.setname(&args.first, &args.last)).await
    }

    #[tool(description = "Set your account bio")]
    async fn setbio(&self, Parameters(args): Parameters<BioArgs>) -> String {
        self.run(self.client.setbio(&args.about)).await
    }

    #[tool(description = "Set your account profile photo from a local file")]
    async fn setphoto(&self, Parameters(args): Parameters<PhotoArgs>) -> String {
        self.run(self.client.setphoto(&args.path)).await
    }

    #[tool(description = "Block a user")]
    async fn block(&self, Parameters(args): Parameters<TargetArgs>) -> String {
        self.run(self.client.block(&args.target)).await
    }

    #[tool(description = "Unblock a user")]
    async fn unblock(&self, Parameters(args): Parameters<TargetArgs>) -> String {
        self.run(self.client.unblock(&args.target)).await
    }

    #[tool(description = "Search all chats in the local sync mirror")]
    async fn searchall(&self, Parameters(args): Parameters<SearchallArgs>) -> String {
        let limit = args.limit.unwrap_or(20) as usize;
        self.run(self.client.searchall(&args.query, limit)).await
    }

    #[tool(description = "Search messages within one chat")]
    async fn searchin(&self, Parameters(args): Parameters<SearchinArgs>) -> String {
        let limit = args.limit.unwrap_or(20) as usize;
        self.run(self.client.searchin(&args.target, &args.query, limit))
            .await
    }

    #[tool(description = "Send a photo to a chat (caption is rich text)")]
    async fn photo(&self, Parameters(args): Parameters<UploadArgs>) -> String {
        let send = termgram::Send {
            target: args.target,
            text: args.caption,
            files: vec![args.path],
            ..Default::default()
        };
        self.run(self.client.send(&send)).await
    }

    #[tool(description = "Send multiple files as one media album (caption is plain text)")]
    async fn album(&self, Parameters(args): Parameters<AlbumArgs>) -> String {
        let send = termgram::Send {
            target: args.target,
            text: args.caption,
            files: args.paths,
            ..Default::default()
        };
        self.run(self.client.send(&send)).await
    }

    #[tool(description = "Send an audio file as a voice message")]
    async fn voice(&self, Parameters(args): Parameters<VoiceArgs>) -> String {
        let send = termgram::Send {
            target: args.target,
            files: vec![args.path],
            ..Default::default()
        };
        self.run(self.client.send(&send)).await
    }

    #[tool(description = "Read cached messages of a chat from the local mirror")]
    async fn cached(&self, Parameters(args): Parameters<CachedArgs>) -> String {
        let limit = args.limit.unwrap_or(20) as usize;
        match termgram::Mirror::open(&self.client.mirror()).await {
            Ok(mirror) => self
                .run(mirror.lines(&args.target, limit))
                .await,
            Err(err) => format!("error: {err}"),
        }
    }

    #[tool(description = "Download every media of a chat into the local media dir")]
    async fn grab(&self, Parameters(args): Parameters<TargetArgs>) -> String {
        self.run(self.client.grab(&args.target)).await
    }

    #[tool(description = "Export the account session, mirror and config to a folder")]
    async fn export(&self, Parameters(args): Parameters<ExportArgs>) -> String {
        let path = args.path.as_deref().unwrap_or("termgram-export");
        self.run(self.client.export(path)).await
    }

    #[tool(description = "Import an exported account folder into this session")]
    async fn import(&self, Parameters(args): Parameters<ImportArgs>) -> String {
        self.run(self.client.import(&args.path)).await
    }

    #[tool(description = "Delete the session, mirror and media of the current account")]
    async fn wipe(&self, _: Parameters<Empty>) -> String {
        self.run(self.client.wipe()).await
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = termgram::Config::load()?;
    let client = Client::new(&cfg, "default").await?;
    if client.raw.is_authorized().await.unwrap_or(false) {
        eprintln!("termgram-mcp serving");
    } else {
        eprintln!("termgram-mcp: not logged in - run `termgram login` in a terminal first");
    }
    let service = Server { client }.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}