use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "termgram", version, about = "a telegram client for your terminal")]
pub struct Cli {
    #[arg(long, global = true, default_value = "default")]
    pub account: String,
    #[arg(long, global = true)]
    pub json: bool,
    #[arg(long, global = true)]
    pub full: bool,
    #[command(subcommand)]
    pub op: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Login,
    Logout,
    Me,
    Dialogs,
    Messages {
        /// who: @username, chat id, or contact name
        target: String,
        #[arg(default_value_t = 20)]
        limit: usize,
    },
    Send {
        /// who: @username, chat id, or contact name
        target: String,
        /// text body, or media caption when --file is used
        text: Option<String>,
        /// file path(s): one sends a photo, document, or voice note by type; several send an album
        #[arg(long = "file", action = clap::ArgAction::Append)]
        files: Vec<String>,
        #[arg(long)]
        reply: Option<i32>,
        /// forum topic id to send into
        #[arg(long)]
        topic: Option<i32>,
        /// plain, md, or html
        #[arg(long)]
        format: Option<String>,
        /// exact date text in message to render as a tappable chip; repeat for more
        #[arg(long = "date", action = clap::ArgAction::Append)]
        dates: Vec<String>,
        /// send now, HH:MM, or YYYY-MM-DD HH:MM
        #[arg(long)]
        at: Option<String>,
    },
    Read {
        /// who: @username, chat id, or contact name
        target: String,
    },
    Poll {
        /// who: @username, chat id, or contact name
        target: String,
        question: String,
        /// 2 to 10 answer choices
        #[arg(value_name = "OPTION", action = clap::ArgAction::Append)]
        options: Vec<String>,
        /// hide who voted for what
        #[arg(long)]
        anon: bool,
        /// allow more than one answer
        #[arg(long)]
        multi: bool,
    },
    Topics {
        /// who: @username, chat id, or contact name
        target: String,
    },
    Watch {
        /// who: @username, chat id, or contact name; omit to follow every chat
        #[arg(long)]
        target: Option<String>,
        /// print the last messages of the chat and exit instead of following
        #[arg(long)]
        once: bool,
    },
    Sync {
        #[arg(default_value_t = 200)]
        limit: usize,
    },
    Search {
        query: String,
        /// limit the search to one chat (@username, id, or contact name)
        #[arg(short, long)]
        chat: Option<String>,
    },
    Download {
        /// who: @username, chat id, or contact name
        target: String,
        #[arg(short, long)]
        id: Option<i32>,
    },
    Edit {
        /// who: @username, chat id, or contact name
        target: String,
        id: i32,
        text: String,
        /// plain, md, or html
        #[arg(long)]
        format: Option<String>,
        /// exact date text in text to render as a tappable chip; repeat for more
        #[arg(long = "date", action = clap::ArgAction::Append)]
        dates: Vec<String>,
    },
    Delete {
        /// who: @username, chat id, or contact name
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
        /// who: @username, chat id, or contact name
        target: String,
        id: i32,
        #[arg(long)]
        remove: bool,
    },
    React {
        /// who: @username, chat id, or contact name
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
        /// who: @username, chat id, or contact name
        target: String,
        #[arg(default_value_t = 20)]
        limit: i32,
    },
    Members {
        /// who: @username, chat id, or contact name
        target: String,
    },
    Kick {
        /// group: @username, chat id, or contact name
        target: String,
        user: String,
    },
    Ban {
        /// group: @username, chat id, or contact name
        target: String,
        user: String,
    },
    Unban {
        /// group: @username, chat id, or contact name
        target: String,
        user: String,
    },
    Promote {
        /// group: @username, chat id, or contact name
        target: String,
        user: String,
        #[arg(long)]
        rank: Option<String>,
    },
    Typing {
        /// who: @username, chat id, or contact name
        target: String,
    },
    Status {
        /// who: @username, chat id, or contact name
        target: String,
    },
    Scheduled {
        /// who: @username, chat id, or contact name
        target: String,
    },
    Cancel {
        /// who: @username, chat id, or contact name
        target: String,
        id: i32,
    },
    Draft {
        /// who: @username, chat id, or contact name
        target: String,
        text: Option<String>,
        /// plain, md, or html
        #[arg(long)]
        format: Option<String>,
        /// exact date text in text to render as a tappable chip; repeat for more
        #[arg(long = "date", action = clap::ArgAction::Append)]
        dates: Vec<String>,
    },
    Drafts,
    Profile {
        /// who: @username, chat id, or contact name
        target: String,
    },
    Set {
        #[command(subcommand)]
        what: What,
    },
    Block {
        /// who: @username, chat id, or contact name
        target: String,
    },
    Unblock {
        /// who: @username, chat id, or contact name
        target: String,
    },
    Searchall {
        query: String,
        #[arg(default_value_t = 20)]
        limit: usize,
    },
    Searchin {
        /// who: @username, chat id, or contact name
        target: String,
        query: String,
        #[arg(default_value_t = 20)]
        limit: usize,
    },
    Cached {
        /// who: @username, chat id, or contact name
        target: String,
        #[arg(default_value_t = 20)]
        limit: usize,
    },
    Grab {
        /// who: @username, chat id, or contact name
        target: String,
    },
    Export {
        path: Option<String>,
    },
    Import {
        path: String,
    },
    Wipe,
}

#[derive(Subcommand)]
pub enum What {
    Name {
        first: String,
        last: String,
    },
    Bio {
        about: String,
    },
    Photo {
        path: String,
    },
}