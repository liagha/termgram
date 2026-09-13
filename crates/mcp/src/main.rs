use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CustomNotification, ServerNotification},
    schemars, tool, tool_router,
    transport::stdio,
    ServiceExt,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashSet;
use std::future::Future;
use std::sync::{Arc, Mutex};

use termgram::{Client, Update};

#[derive(Default)]
pub struct Watch {
    pub on: bool,
    pub chats: HashSet<i64>,
}

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
pub struct WaitArgs {
    pub target: Option<String>,
    pub after: Option<i32>,
    pub timeout: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NotifyArgs {
    pub on: bool,
    pub target: Option<String>,
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
pub struct PhoneContactArgs {
    pub phone: String,
    pub first: String,
    pub last: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ImportArgs2 {
    pub contacts: Vec<termgram::PhoneContact>,
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
    watch: Arc<Mutex<Watch>>,
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

    #[tool(description = "Wait (push, no polling) until a new message arrives - returns instantly on Telegram push. target: chat to watch (omit for any chat). after: only messages with id greater than this. timeout: seconds to wait (default 60, max 110)")]
    async fn wait(&self, Parameters(args): Parameters<WaitArgs>) -> String {
        let timeout = args.timeout.unwrap_or(60).clamp(1, 110);
        self.run(
            self.client
                .wait(args.target.as_deref(), args.after, timeout),
        )
        .await
    }

    #[tool(description = "Control the server-to-client push filter for new-message notifications. on=true with target: push only that chat. on=true: push every chat. on=false with target: stop pushing that chat. on=false: stop all pushes")]
    async fn notify(&self, Parameters(args): Parameters<NotifyArgs>) -> String {
        let chat_id = match args.target.as_deref() {
            Some(slot) => match self.client.resolve(slot).await {
                Ok(peer) => peer.id.bot_api_dialog_id().unwrap_or(0),
                Err(err) => return format!("error: {err}"),
            },
            None => 0,
        };
        let mut watch = self.watch.lock().unwrap();
        match args.on {
            true => match args.target {
                Some(_) => {
                    watch.chats.insert(chat_id);
                    watch.on = true;
                }
                None => {
                    watch.chats.clear();
                    watch.on = true;
                }
            },
            false => match args.target {
                Some(_) => {
                    watch.chats.remove(&chat_id);
                    watch.on = !watch.chats.is_empty();
                }
                None => {
                    watch.on = false;
                    watch.chats.clear();
                }
            },
        }
        let chats = watch.chats.iter().copied().collect::<Vec<_>>();
        serde_json::json!({ "on": watch.on, "chats": chats }).to_string()
    }

    #[tool(description = "Send text and/or files to a chat. files: one file sends a photo, document, or voice note by type, several send an album. text.format: plain, markdown, or html. text.dates: exact date text in the message to render as tappable chips. reply: message id to reply to. topic: forum topic id to send into. at: future unix timestamp to schedule the send")]
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

    #[tool(description = "Add a contact by phone number")]
    async fn add_contact(&self, Parameters(args): Parameters<PhoneContactArgs>) -> String {
        self.run(self.client.add_contact(&args.phone, &args.first, &args.last))
            .await
    }

    #[tool(description = "Delete a contact")]
    async fn delete_contact(&self, Parameters(args): Parameters<TargetArgs>) -> String {
        self.run(self.client.delete_contact(&args.target)).await
    }

    #[tool(description = "Export all contacts of the account")]
    async fn export_contacts(&self, _: Parameters<Empty>) -> String {
        self.run(self.client.export_contacts()).await
    }

    #[tool(description = "Import contacts by phone number (each with first and last name)")]
    async fn import_contacts(&self, Parameters(args): Parameters<ImportArgs2>) -> String {
        self.run(self.client.import_contacts(&args.contacts)).await
    }

    #[tool(description = "List blocked users")]
    async fn block_list(&self, _: Parameters<Empty>) -> String {
        self.run(self.client.block_list()).await
    }

    #[tool(description = "Remove the profile photo of a chat")]
    async fn del_photo(&self, Parameters(args): Parameters<TargetArgs>) -> String {
        self.run(self.client.del_photo(&args.target)).await
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

const CATALOG: &[(&str, &str)] = &[
    ("login", "Check login state; interactive OTP login only via `termgram login` in a terminal"),
    ("logout", "Log out the current session"),
    ("me", "Show the logged-in account identity"),
    ("dialogs", "List chats with unread count and last message preview"),
    ("messages", "Show recent messages in a chat, newest last"),
    ("poll", "Create a poll in a chat with 2 to 10 answer options"),
    ("topics", "List forum topics in a forum-enabled chat"),
    ("watch", "Snapshot the last messages of a chat, or the latest dialogs when target is omitted"),
    ("wait", "Wait (push, no polling) until a new message arrives"),
    ("notify", "Control the server-to-client push filter for new-message notifications"),
    ("send", "Send text and/or files to a chat"),
    ("read", "Mark a chat as read"),
    ("sync", "Sync dialog list and recent messages into the local search mirror"),
    ("search", "Search the local sync mirror for messages"),
    ("upload", "Upload a file to a chat (sends a photo, document, or voice note by type; caption is rich text)"),
    ("download", "Download media from a chat message to the local media dir"),
    ("edit", "Edit a message you sent"),
    ("delete", "Delete messages from a chat"),
    ("forward", "Forward messages between chats"),
    ("pin", "Pin or unpin a message in a chat"),
    ("react", "React to a message with an emoji, or remove the reaction"),
    ("reactions", "List the reactions on a message"),
    ("contacts", "List contacts of the account"),
    ("add_contact", "Add a contact by phone number"),
    ("delete_contact", "Delete a contact"),
    ("export_contacts", "Export all contacts of the account"),
    ("import_contacts", "Import contacts by phone number"),
    ("block_list", "List blocked users"),
    ("del_photo", "Remove the profile photo of a chat"),
    ("folders", "List chat folders"),
    ("folder_new", "Create a chat folder with the given chats"),
    ("folder_rm", "Delete a chat folder"),
    ("pinned", "List pinned messages of a chat"),
    ("members", "List members of a group or channel"),
    ("kick", "Kick a user from a group"),
    ("ban", "Ban a user in a channel"),
    ("unban", "Unban a user in a channel"),
    ("promote", "Promote a user to admin in a channel"),
    ("typing", "Broadcast a typing action in a chat"),
    ("status", "Show a user's online status"),
    ("schedule", "Schedule a rich text message for a future unix timestamp"),
    ("scheduled", "List scheduled messages of a chat"),
    ("cancel", "Cancel a scheduled message of a chat"),
    ("draft", "Save or clear a draft in a chat"),
    ("drafts", "List all saved drafts"),
    ("profile", "Show a user's full profile"),
    ("setname", "Set your account first and last name"),
    ("setbio", "Set your account bio"),
    ("setphoto", "Set your account profile photo from a local file"),
    ("block", "Block a user"),
    ("unblock", "Unblock a user"),
    ("searchall", "Search all chats in the local sync mirror"),
    ("searchin", "Search messages within one chat"),
    ("photo", "Send a photo to a chat (caption is rich text)"),
    ("album", "Send multiple files as one media album (caption is plain text)"),
    ("voice", "Send an audio file as a voice message"),
    ("cached", "Read cached messages of a chat from the local mirror"),
    ("grab", "Download every media of a chat into the local media dir"),
    ("export", "Export the account session, mirror and config to a folder"),
    ("import", "Import an exported account folder into this session"),
    ("wipe", "Delete the session, mirror and media of the current account"),
];

async fn catalog() -> anyhow::Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
    let tools = CATALOG
        .iter()
        .map(|(name, description)| {
            json!({
                "name": name,
                "description": description,
                "inputSchema": { "type": "object", "properties": {} }
            })
        })
        .collect::<Vec<_>>();
    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();
    let mut stdout = tokio::io::stdout();
    while let Some(line) = stdin.next_line().await? {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        let id = msg.get("id").cloned();
        let Some(method) = msg.get("method").and_then(|m| m.as_str()) else {
            continue;
        };
        let result = match method {
            "initialize" => json!({
                "protocolVersion": "2025-06-18",
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": {
                    "name": "termgram-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
            "ping" => json!(null),
            "tools/list" => json!({ "tools": &tools }),
            "resources/list" => json!({ "resources": [] }),
            "resources/templates/list" => json!({ "resourceTemplates": [] }),
            "prompts/list" => json!({ "prompts": [] }),
            _ => {
                let Some(id) = id else { continue };
                let error = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32601, "message": "method not found" }
                });
                stdout
                    .write_all(serde_json::to_string(&error)?.as_bytes())
                    .await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
                continue;
            }
        };
        if let Some(id) = id {
            let out = json!({ "jsonrpc": "2.0", "id": id, "result": result });
            stdout.write_all(serde_json::to_string(&out)?.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::args().any(|a| a == "--catalog") {
        return catalog().await;
    }
    let cfg = termgram::Config::load()?;
    let client = Client::new(&cfg, "default").await?;
    if client.raw.is_authorized().await.unwrap_or(false) {
        eprintln!("termgram-mcp serving");
    } else {
        eprintln!("termgram-mcp: not logged in - run `termgram login` in a terminal first");
    }
    let mut events = client.signals();
    let watch = Arc::new(Mutex::new(Watch::default()));
    let service = Server { client, watch: watch.clone() }.serve(stdio()).await?;
    let peer = service.peer().clone();
    tokio::spawn(async move {
        while let Ok(update) = events.recv().await {
            let Update::NewMessage(msg) = update else {
                continue;
            };
            let chat = msg.peer_id().bot_api_dialog_id().unwrap_or(0);
            {
                let w = watch.lock().unwrap();
                if !w.on || (!w.chats.is_empty() && !w.chats.contains(&chat)) {
                    continue;
                }
            }
            let line = json!({
                "chat": chat,
                "id": msg.id(),
                "at": msg.date().timestamp(),
                "out": msg.outgoing(),
                "from": msg.sender().and_then(|s| s.name()),
                "text": msg.text().to_string(),
                "media": msg.media().as_ref().and_then(termgram::Label::kind),
            });
            let notif = ServerNotification::CustomNotification(CustomNotification::new(
                "notifications/termgram/message",
                Some(line),
            ));
            if peer.send_notification(notif).await.is_err() {
                break;
            }
        }
    });
    service.waiting().await?;
    Ok(())
}