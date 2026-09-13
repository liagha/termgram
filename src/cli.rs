use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "termgram", version, about = "a telegram client for your terminal")]
pub struct Cli {
    #[command(subcommand)]
    pub op: Op,
}

#[derive(Subcommand)]
pub enum Op {
    Login,
    Logout,
    Me,
    Dialogs,
    Messages {
        target: String,
        #[arg(default_value_t = 20)]
        limit: usize,
    },
    Send {
        target: String,
        text: String,
        #[arg(long)]
        reply: Option<i32>,
    },
    Read {
        target: String,
    },
    Watch,
    Sync {
        #[arg(default_value_t = 200)]
        limit: usize,
    },
    Search {
        query: String,
        #[arg(short, long)]
        chat: Option<String>,
    },
    Upload {
        target: String,
        path: String,
        #[arg(short)]
        caption: Option<String>,
    },
    Download {
        target: String,
        #[arg(short, long)]
        id: Option<i32>,
    },
    Edit {
        target: String,
        id: i32,
        text: String,
    },
    Delete {
        target: String,
        ids: Vec<i32>,
    },
    Forward {
        #[arg(value_name = "FROM")]
        from: String,
        #[arg(value_name = "TO")]
        to: String,
        ids: Vec<i32>,
    },
    Pin {
        target: String,
        id: i32,
        #[arg(long)]
        remove: bool,
    },
    React {
        target: String,
        id: i32,
        emoji: Option<String>,
        #[arg(long)]
        remove: bool,
    },
    Contacts,
    Folders,
    FolderNew {
        title: String,
        #[arg(long = "include", action = clap::ArgAction::Append)]
        include: Vec<String>,
    },
    FolderRm {
        id: i32,
    },
    Pinned {
        target: String,
        #[arg(default_value_t = 20)]
        limit: i32,
    },
    Members {
        target: String,
    },
    Kick {
        target: String,
        user: String,
    },
    Ban {
        target: String,
        user: String,
    },
    Unban {
        target: String,
        user: String,
    },
    Promote {
        target: String,
        user: String,
        #[arg(long)]
        rank: Option<String>,
    },
}