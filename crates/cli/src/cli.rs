use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "termgram",
    version,
    about = "A Telegram client for your terminal.",
    long_about = "termgram talks to Telegram natively over MTProto. No Bot API, no external services.\n\n\
    Get started:\n\
      termgram login           # log in with your phone number\n\
      termgram dialogs         # list your chats\n\
      termgram send me 'hi'    # send a message to your Saved Messages\n\
    \n\
    A chat target can be a contact name, @username, or chat ID. 'me' and 'saved'\n\
    both mean your Saved Messages."
)]
pub struct Cli {
    #[arg(
        long,
        global = true,
        default_value = "default",
        help = "Telegram account to use (matches a folder under ~/.config/termgram/)"
    )]
    pub account: String,
    #[arg(long, global = true, help = "Print results as JSON instead of human-readable text")]
    pub json: bool,
    #[arg(long, global = true, help = "Show full timestamps and details instead of compact output")]
    pub full: bool,
    #[command(subcommand)]
    pub op: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Log in with your phone number and the code Telegram sends you
    Login,
    /// Log out the current session
    Logout,
    /// Show your own account identity
    Me,
    /// List all your chats with unread counts and last message
    Dialogs,
    /// Show recent messages in a chat
    Messages {
        /// Chat to read: contact name, @username, chat ID, or 'me'/'saved' for Saved Messages
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// How many messages to show (default 20)
        #[arg(default_value_t = 20, short, long)]
        limit: usize,
    },
    /// Send a message, file, or album to a chat
    Send {
        /// Chat to send to: contact name, @username, chat ID, or 'me'/'saved' for Saved Messages
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Text body to send (omitted when sending files only)
        #[arg(help = "Message text, or media caption when --file is used")]
        text: Option<String>,
        /// File path(s): one file sends a photo/document/voice; several send an album
        #[arg(long = "file", short = 'f', action = clap::ArgAction::Append, help = "File to send (repeat for an album)")]
        files: Vec<String>,
        /// Reply to a specific message ID in the chat
        #[arg(long, short = 'r', help = "Reply to message ID (use `termgram messages <chat>` to find IDs)")]
        reply: Option<i32>,
        /// Forum topic ID to send into
        #[arg(long, help = "Forum topic ID (for forum-enabled chats/groups)")]
        topic: Option<i32>,
        /// Text format: plain, md (markdown), or html (default: plain)
        #[arg(long, help = "Message format: plain, md, or html")]
        format: Option<String>,
        /// Exact date text in the message to render as a tappable chip (repeat for more)
        #[arg(long = "date", action = clap::ArgAction::Append, help = "Date text to make tappable (e.g. '13/09 17:00')")]
        dates: Vec<String>,
        /// Schedule the send: 'now', 'HH:MM', or 'YYYY-MM-DD HH:MM'
        #[arg(long, help = "Send time: now, HH:MM, or YYYY-MM-DD HH:MM")]
        at: Option<String>,
    },
    /// Mark a chat as read
    Read {
        /// Chat to mark as read: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
    },
    /// Create a poll in a chat
    Poll {
        /// Chat to send the poll to
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Poll question
        question: String,
        /// 2 to 10 answer choices (repeat for each)
        #[arg(value_name = "OPTION", action = clap::ArgAction::Append, help = "Poll answer option (repeat for more)")]
        options: Vec<String>,
        /// Hide who voted for what
        #[arg(long, help = "Make voters anonymous")]
        anon: bool,
        /// Allow more than one answer per person
        #[arg(long, help = "Allow multiple answers")]
        multi: bool,
    },
    /// List forum topics in a forum-enabled chat
    Topics {
        /// Chat to list topics for
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
    },
    /// Follow a chat live, or survey unread chats
    Watch {
        /// Chat to follow (omit to follow every chat)
        #[arg(long, help = "Chat to watch (omit to watch every chat)")]
        target: Option<String>,
        /// Print recent messages and exit instead of following live
        #[arg(long, help = "Show recent messages and exit")]
        once: bool,
        /// Survey chats with unread messages instead of following one chat
        #[arg(long, help = "List chats with unread messages instead of following")]
        unread: bool,
        /// Seconds between unread surveys (default 30)
        #[arg(long, default_value_t = 30, help = "Seconds between unread surveys")]
        every: u64,
    },
    /// Sync your chats and messages into the local search mirror
    Sync {
        /// How many messages per chat to sync (default 200)
        #[arg(default_value_t = 200, help = "Messages per chat to sync")]
        limit: usize,
    },
    /// Search your local message mirror
    Search {
        /// Search query
        query: String,
        /// Limit search to one chat
        #[arg(short, long, help = "Only search within this chat")]
        chat: Option<String>,
    },
    /// Download media from a chat message
    Download {
        /// Chat containing the message
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Message ID to download (defaults to the latest message with media)
        #[arg(short, long, help = "Message ID to download")]
        id: Option<i32>,
    },
    /// Edit a message you sent
    Edit {
        /// Chat containing the message
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Message ID to edit
        id: i32,
        /// New text body
        text: String,
        /// Text format: plain, md (markdown), or html
        #[arg(long, help = "Message format: plain, md, or html")]
        format: Option<String>,
        /// Exact date text to render as a tappable chip (repeat for more)
        #[arg(long = "date", action = clap::ArgAction::Append, help = "Date text to make tappable")]
        dates: Vec<String>,
    },
    /// Delete messages from a chat
    Delete {
        /// Chat to delete from
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Message IDs to delete
        #[arg(value_name = "ID", help = "Message ID(s) to delete")]
        ids: Vec<i32>,
    },
    /// Forward messages between chats
    Forward {
        /// Source chat
        #[arg(value_name = "FROM", help = "Source chat: contact name, @username, chat ID, or 'me'/'saved'")]
        from: String,
        /// Destination chat
        #[arg(value_name = "TO", help = "Destination chat: contact name, @username, chat ID, or 'me'/'saved'")]
        to: String,
        /// Message IDs to forward
        #[arg(value_name = "ID", help = "Message ID(s) to forward")]
        ids: Vec<i32>,
    },
    /// Pin or unpin a message in a chat
    Pin {
        /// Chat to pin in
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Message ID to pin
        id: i32,
        /// Unpin instead of pin
        #[arg(long, help = "Unpin the message instead")]
        remove: bool,
    },
    /// React to a message with an emoji
    React {
        /// Chat containing the message
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Message ID to react to (omits to use the last message)
        #[arg(help = "Message ID (omit to react to the last message)")]
        id: Option<i32>,
        /// Emoji to react with
        #[arg(help = "Emoji to react with (e.g. 🎉)")]
        emoji: Option<String>,
        /// Remove your reaction instead of adding one
        #[arg(long, help = "Remove your reaction instead of adding one")]
        remove: bool,
        /// Make the reaction animate big
        #[arg(long, help = "Make the reaction appear large")]
        big: bool,
    },
    /// List reactions on a message
    Reactions {
        /// Chat containing the message
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Message ID to list reactions for
        id: i32,
    },
    /// List your contacts
    Contacts,
    /// Add a contact by phone number
    AddContact {
        /// International phone number, e.g. +15551234567
        phone: String,
        /// First name
        first: String,
        /// Last name
        last: String,
    },
    /// Delete a contact
    DeleteContact {
        /// Contact to delete: name, @username, or chat ID
        #[arg(help = "Contact: name, @username, or chat ID")]
        target: String,
    },
    /// Export all your contacts
    ExportContacts,
    /// Import contacts from CSV-style arguments
    ImportContacts {
        /// Contact as "phone,first,last" (repeat for more)
        #[arg(
            long = "contact",
            action = clap::ArgAction::Append,
            value_name = "PHONE,FIRST,LAST",
            help = "Contact as 'phone,first,last' (repeatable)"
        )]
        contact: Vec<String>,
    },
    /// List blocked users
    BlockList,
    /// Remove a chat's profile photo
    DelPhoto {
        /// Chat whose photo to remove
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
    },
    /// List your chat folders
    Folders,
    /// Create a chat folder
    FolderNew {
        /// Folder title
        title: String,
        /// Chats to include in the folder (repeat for more)
        #[arg(long = "include", action = clap::ArgAction::Append, help = "Chat to include (repeatable)")]
        include: Vec<String>,
    },
    /// Create a group chat
    CreateGroup {
        /// Group title
        title: String,
        /// Users to add: @username, ID, or contact name (repeatable)
        #[arg(long = "user", action = clap::ArgAction::Append, value_name = "USER", help = "User to add (repeatable)")]
        users: Vec<String>,
    },
    /// Create a channel or supergroup
    CreateChannel {
        /// Channel title
        title: String,
        /// Channel description
        about: String,
        /// Users to add: @username, ID, or contact name (repeatable)
        #[arg(long = "user", action = clap::ArgAction::Append, value_name = "USER", help = "User to add (repeatable)")]
        users: Vec<String>,
    },
    /// Leave a chat, group, or channel
    Leave {
        /// Chat to leave: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
    },
    /// Join a chat via invite link
    Join {
        /// Invite link (t.me/joinchat/... or https://t.me/+...)
        link: String,
    },
    /// Delete a chat folder
    FolderRm {
        /// Folder ID (from `termgram folders`)
        id: i32,
    },
    /// List pinned messages of a chat
    Pinned {
        /// Chat to list pinned messages for
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// How many to show (default 20)
        #[arg(default_value_t = 20, help = "Maximum pinned messages to show")]
        limit: i32,
    },
    /// List members of a group or channel
    Members {
        /// Chat to list members for
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
    },
    /// Kick a user from a group
    Kick {
        /// Group to kick from
        #[arg(help = "Group: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// User to kick: username, ID, or contact name
        #[arg(help = "User: @username, ID, or contact name")]
        user: String,
    },
    /// Ban a user in a channel
    Ban {
        /// Channel to ban in
        #[arg(help = "Channel: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// User to ban: username, ID, or contact name
        #[arg(help = "User: @username, ID, or contact name")]
        user: String,
    },
    /// Unban a user in a channel
    Unban {
        /// Channel to unban in
        #[arg(help = "Channel: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// User to unban: username, ID, or contact name
        #[arg(help = "User: @username, ID, or contact name")]
        user: String,
    },
    /// Promote a user to admin in a channel
    Promote {
        /// Channel to promote in
        #[arg(help = "Channel: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// User to promote: username, ID, or contact name
        #[arg(help = "User: @username, ID, or contact name")]
        user: String,
        /// Admin title/rank
        #[arg(long, help = "Admin title (rank)")]
        rank: Option<String>,
    },
    /// Broadcast a typing action in a chat
    Typing {
        /// Chat to show typing in
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
    },
    /// Show a user's online status
    Status {
        /// User to check status for
        #[arg(help = "User: @username, ID, or contact name")]
        target: String,
    },
    /// List scheduled messages of a chat
    Scheduled {
        /// Chat to list scheduled messages for
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
    },
    /// Cancel a scheduled message
    Cancel {
        /// Chat containing the scheduled message
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Scheduled message ID to cancel
        id: i32,
    },
    /// Save or clear a draft in a chat
    Draft {
        /// Chat to save the draft in
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Draft text (omit to clear the draft)
        text: Option<String>,
        /// Text format: plain, md (markdown), or html
        #[arg(long, help = "Message format: plain, md, or html")]
        format: Option<String>,
        /// Exact date text to render as a tappable chip (repeat for more)
        #[arg(long = "date", action = clap::ArgAction::Append, help = "Date text to make tappable")]
        dates: Vec<String>,
    },
    /// List all saved drafts
    Drafts,
    /// Show a user's full profile
    Profile {
        /// User to view profile for
        #[arg(help = "User: @username, ID, or contact name")]
        target: String,
    },
    /// Set your account name, bio, or photo
    Set {
        #[command(subcommand)]
        what: What,
    },
    /// Block a user
    Block {
        /// User to block: @username, ID, or contact name
        #[arg(help = "User: @username, ID, or contact name")]
        target: String,
    },
    /// Unblock a user
    Unblock {
        /// User to unblock: @username, ID, or contact name
        #[arg(help = "User: @username, ID, or contact name")]
        target: String,
    },
    /// Search all chats globally (via Telegram API)
    Searchall {
        /// Search query
        query: String,
        /// Maximum results (default 20)
        #[arg(default_value_t = 20, help = "Maximum results to return")]
        limit: usize,
    },
    /// Search messages within one chat (via Telegram API)
    Searchin {
        /// Chat to search within
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Search query
        query: String,
        /// Maximum results (default 20)
        #[arg(default_value_t = 20, help = "Maximum results to return")]
        limit: usize,
    },
    /// Read cached messages of a chat from the local mirror
    Cached {
        /// Chat to read cached messages for
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// How many messages to show (default 20)
        #[arg(default_value_t = 20, help = "Maximum cached messages to show")]
        limit: usize,
    },
    /// Download all media from a chat
    Grab {
        /// Chat to download media from
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
    },
    /// Export your session, mirror, and config to a folder
    Export {
        /// Destination folder (default: termgram-export)
        #[arg(help = "Destination folder (default: termgram-export)")]
        path: Option<String>,
    },
    /// Import an exported account folder
    Import {
        /// Folder to import from
        path: String,
    },
    /// Mute or unmute chat notifications
    Mute {
        /// Chat to mute/unmute: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Mute forever (omit to unmute)
        #[arg(long, help = "Mute forever (omit to unmute)")]
        forever: bool,
    },
    /// List admins of a group or channel
    Admins {
        /// Chat to list admins for: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
    },
    /// List banned users in a channel
    Banned {
        /// Channel to list banned users for: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Channel: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
    },
    /// Demote a user from admin in a channel
    Demote {
        /// Channel to demote in: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Channel: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// User to demote: @username, ID, or contact name
        #[arg(help = "User: @username, ID, or contact name")]
        user: String,
    },
    /// Invite users to a group or channel
    Invite {
        /// Chat to invite users to: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Users to invite: @username, ID, or contact name (repeatable)
        #[arg(long = "user", action = clap::ArgAction::Append, value_name = "USER", help = "User to invite (repeatable)")]
        users: Vec<String>,
    },
    /// Rename a group or channel
    Rename {
        /// Chat to rename: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// New title
        title: String,
    },
    /// Set the description of a group or channel
    About {
        /// Chat to set description for: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// New description
        about: String,
    },
    /// Change the photo of a group or channel
    ChatPhoto {
        /// Chat to change photo for: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Path to image file
        path: String,
    },
    /// Export an invite link for a chat
    InviteLink {
        /// Chat to export invite link for: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
    },
    /// Clear history of a chat
    Flush {
        /// Chat to clear history in: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Also remove history for other participants
        #[arg(long, help = "Revoke history for everyone")]
        revoke: bool,
    },
    /// Send a sticker to a chat
    SendSticker {
        /// Chat to send sticker to: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Path to sticker file (webp)
        path: String,
    },
    /// List installed sticker packs
    StickerSets,
    /// Show media info of a message
    MediaInfo {
        /// Chat containing the message: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Message ID
        id: i32,
    },
    /// Unpin all messages in a chat
    UnpinAll {
        /// Chat to unpin all messages in: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
    },
    /// Show messages around a given message ID
    Context {
        /// Chat containing the message: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Message ID to get context around
        id: i32,
        /// How many messages before and after (default 5)
        #[arg(default_value_t = 5, help = "Messages before and after")]
        size: i32,
    },
    /// Wait for a new message (push, no polling)
    Wait {
        /// Chat to wait in (omit for any chat)
        #[arg(long, help = "Chat to wait in (omit for any chat)")]
        target: Option<String>,
        /// Only return messages with id greater than this
        #[arg(long, help = "Only messages with id greater than this")]
        after: Option<i32>,
        /// Seconds to wait (default 60, max 110)
        #[arg(long, default_value_t = 60, help = "Seconds to wait")]
        timeout: u64,
    },
    /// Send a location to a chat
    Location {
        /// Chat to send location to: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Latitude
        lat: f64,
        /// Longitude
        lon: f64,
    },
    /// Send a venue to a chat
    Venue {
        /// Chat to send venue to: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Venue name
        title: String,
        /// Venue address
        address: String,
        /// Latitude
        lat: f64,
        /// Longitude
        lon: f64,
    },
    /// Send a contact card to a chat
    Contact {
        /// Chat to send contact to: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Phone number
        phone: String,
        /// First name
        first: String,
        /// Last name
        last: String,
    },
    /// Send a dice to a chat
    Dice {
        /// Chat to send dice to: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
    },
    /// Stop a poll in a chat
    StopPoll {
        /// Chat containing the poll: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Message ID of the poll
        id: i32,
    },
    /// Set your account username (omit to remove)
    Username {
        /// New username (omit to remove)
        #[arg(help = "New username (omit to remove)")]
        username: Option<String>,
    },
    /// List connected sessions (authorized devices)
    Sessions,
    /// Terminate a connected session
    TerminateSession {
        /// Session hash (from `termgram sessions`)
        hash: i64,
    },
    /// Archive or unarchive a chat
    Archive {
        /// Chat to archive/unarchive: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Archive (true) or unarchive (false)
        #[arg(long, default_value_t = true, help = "Archive (true) or unarchive (false)")]
        archive: bool,
    },
    /// Pin or unpin a chat in the dialog list
    PinDialog {
        /// Chat to pin/unpin: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Pin (true) or unpin (false)
        #[arg(long, default_value_t = true, help = "Pin (true) or unpin (false)")]
        pin: bool,
    },
    /// Enable or disable slow mode in a group or channel
    SlowMode {
        /// Chat to set slow mode in: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
        /// Seconds of slow mode (0 to disable)
        seconds: i32,
    },
    /// Create a sticker pack
    CreateStickerSet {
        /// Sticker pack title
        title: String,
        /// Sticker pack short name
        short_name: String,
        /// Sticker files (webp) to add (repeatable)
        #[arg(long = "file", action = clap::ArgAction::Append, value_name = "FILE", help = "Sticker file (repeatable)")]
        files: Vec<String>,
    },
    /// Set your animated emoji status (omit emoji to clear)
    EmojiStatus {
        /// Emoji to set as status (omit to clear)
        #[arg(help = "Emoji to set as status (omit to clear)")]
        emoji: Option<String>,
    },
    /// Toggle close friends for a user
    CloseFriend {
        /// User to toggle: @username, ID, or contact name
        #[arg(help = "User: @username, ID, or contact name")]
        target: String,
    },
    /// Migrate a basic group to a supergroup
    Migrate {
        /// Chat to migrate: contact name, @username, chat ID, or 'me'/'saved'
        #[arg(help = "Chat: contact name, @username, chat ID, or 'me'/'saved'")]
        target: String,
    },
    /// Wipe your session, mirror, and media
    Wipe,
    /// Proxy discovery and management
    Proxy {
        #[command(subcommand)]
        op: ProxyOp,
    },
}

#[derive(Subcommand)]
pub enum ProxyOp {
    /// Discover and test MTProto proxies from public sources
    Discover {
        /// Timeout per proxy in seconds (default 10)
        #[arg(long, default_value_t = 10, help = "Timeout per proxy in seconds")]
        timeout: u64,
        /// Concurrent checks (default 50)
        #[arg(long, default_value_t = 50, help = "Concurrent proxy checks")]
        concurrency: usize,
    },
    /// List cached proxies with latency
    List,
    /// Use a specific proxy for the connection
    Use {
        /// Proxy server (hostname or IP)
        server: String,
        /// Proxy port
        port: u16,
        /// MTProto secret (hex, base64, or FAST format)
        secret: String,
    },
    /// Clear the configured proxy (use direct connection)
    Clear,
    /// Show the currently configured proxy
    Status,
}

#[derive(Subcommand)]
pub enum What {
    /// Set your account first and last name
    Name {
        /// First name
        first: String,
        /// Last name
        last: String,
    },
    /// Set your account bio
    Bio {
        /// Bio text
        about: String,
    },
    /// Set your account profile photo from a local file
    Photo {
        /// Path to image file
        path: String,
    },
    /// Manage two-factor authentication password
    Password {
        #[command(subcommand)]
        what: PasswordWhat,
    },
}

#[derive(Subcommand)]
pub enum PasswordWhat {
    /// Show two-factor authentication password hint
    Hint,
    /// Set or change your two-factor authentication password
    Set {
        /// New password
        password: String,
        /// Password hint
        hint: String,
    },
    /// Remove your two-factor authentication password
    Remove {
        /// Current password
        password: String,
    },
}
