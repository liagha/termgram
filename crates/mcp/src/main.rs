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
pub struct SendArgs {
    pub target: String,
    pub text: String,
    pub reply: Option<i32>,
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
    pub caption: Option<String>,
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
    pub text: String,
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
    pub id: i32,
    pub emoji: Option<String>,
    pub remove: bool,
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

    #[tool(description = "Send a text message to a chat")]
    async fn send(&self, Parameters(args): Parameters<SendArgs>) -> String {
        self.run(self.client.send(&args.target, &args.text, args.reply))
            .await
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

    #[tool(description = "Upload a file to a chat")]
    async fn upload(&self, Parameters(args): Parameters<UploadArgs>) -> String {
        self.run(
            self.client
                .upload(&args.target, &args.path, args.caption.as_deref()),
        )
        .await
    }

    #[tool(description = "Download media from a chat message to the local media dir")]
    async fn download(&self, Parameters(args): Parameters<DownloadArgs>) -> String {
        self.run(self.client.download(&args.target, args.id)).await
    }

    #[tool(description = "Edit a message you sent")]
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

    #[tool(description = "React to a message with an emoji, or remove the reaction")]
    async fn react(&self, Parameters(args): Parameters<ReactArgs>) -> String {
        self.run(
            self.client
                .react(&args.target, args.id, args.emoji.as_deref(), args.remove),
        )
        .await
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = termgram::Config::load()?;
    let client = Client::new(&cfg).await?;
    if client.raw.is_authorized().await.unwrap_or(false) {
        eprintln!("termgram-mcp serving");
    } else {
        eprintln!("termgram-mcp: not logged in - run `termgram login` in a terminal first");
    }
    let service = Server { client }.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}