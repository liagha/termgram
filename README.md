# termgram

**Telegram in your terminal — a complete MTProto client, an MCP server, and a
push stream, all in one.**

termgram is a fast, lean Telegram client built directly on the MTProto layer
(via the `grammers` stack). It talks to Telegram natively — no Bot API, no
ffmpeg, no services. One unified command sends text, rich media, tappable date
chips and scheduled messages; a local mirror makes search instant; and two
integration surfaces let your own tools react to new messages the moment they
land:

- **MCP server** (`termgram-mcp`) — every CLI command becomes a tool your AI
  client can call, and the server pushes new messages to the client over
  server→client notifications.
- **SSE stream** (`termgram-notify`) — a plain HTTP endpoint any script can
  subscribe to.

Built for people who live in the terminal and want AI on top of their chats.

## Highlights

- **One unified send pipeline** — text, `plain`/`markdown`/`html` formatting
  parsed into real Telegram entities, tappable date chips, photos, documents,
  voice notes, albums, replies, scheduling, forum topics — one `send` command.
- **Local search mirror** — `sync` once, search offline-instant across every
  chat. Missing chats auto-sync on demand, with a live API fallback.
- **Full chat tooling** — polls, reactions, drafts, pins, folders, contacts,
  members, kick/ban/promote, export/import/wipe.
- **Push, not polling** — new messages reach you via a broadcast signal;
  `wait` blocks server-side on a real Telegram push and returns instantly.
- **Per-chat notification filter** — tell the MCP server which chats to push
  and it only pushes those, or push everything.
- **One auth for everything** — a single session drives the CLI, MCP server
  and notifier. Multiple accounts supported.

## Getting started

### 1. Register an app

Telegram needs an API ID and hash per client. Create one at
[my.telegram.org](https://my.telegram.org/apps) (any value works — it's your
own account).

### 2. Configure

termgram keeps everything under `~/.config/termgram/`. On first run it writes
a `config.toml` template; fill it in:

```toml
# ~/.config/termgram/config.toml
api_id = 123456
api_hash = "0123456789abcdef0123456789abcdef"
```

### 3. Login

```sh
termgram login            # enter your phone number, receive a code
termgram login 12345      # verify the code
```

Your session is stored in `~/.config/termgram/session.db`. One login is shared
by the CLI, the MCP server and the notifier.

### 4. Install the binaries

Build the workspace and copy the three binaries into your `PATH`:

```sh
cargo build --release
cp target/release/termgram ~/.local/bin/
cp target/release/termgram-mcp ~/.local/bin/
cp target/release/termgram-notify ~/.local/bin/
```

Or install each crate individually with `cargo install --path crates/cli`,
`cargo install --path crates/mcp`, `cargo install --path crates/notify`.

## CLI

### Send anything, one command

```sh
termgram send saved 'hello'                      # plain text
termgram send saved '**bold** text' --format md  # rich text
termgram send saved 'see you' --date '14/09 17:00' --date '15/09 09:00'
termgram send saved 'photo' --file shot.png      # photo / document / voice
termgram send saved --file a.png --file b.png    # album
termgram send amir --at '16:00'                  # scheduled
termgram send saved 'into a topic' --topic 42    # forum topic
termgram send saved 'reply me' --reply 1108131   # reply
```

`--date` repeats to add more tappable date chips. Formats are `plain`, `md`,
and `html`.

### Everyday commands

```sh
termgram dialogs                 # all chats, unread counts, last message
termgram messages saved          # recent messages, newest last
termgram watch saved             # tail a chat live
termgram watch --unread          # survey chats with unread messages
termgram read saved              # mark as read
termgram sync                    # index everything locally
termgram search 'query'          # search the local mirror
termgram search --chat amir 'query'
termgram poll saved 'which one' red green blue --multi --anon
termgram react saved 1108131 🎉  # Premium required to send
termgram profile amir            # full profile
termgram status amir             # online status
termgram kick groupname user     # and: ban, unban, promote, members, pinned
termgram export ~/termgram-backup
```

Full surface: `me men dialogs messages read watch send edit delete forward pin
react reactions drafts draft profile set block unblock typing status scheduled
cancel contacts folders folder-new folder-rm pinned members kick ban unban
promote grab export import wipe poll topics search searchin searchall sync
cached`.

### Text formats

Markdown and HTML are parsed into real Telegram entities before sending.
Dates rendered this way become tappable chips, not raw text.

### Local search mirror

```sh
termgram sync              # index dialogs + recent messages
termgram search 'query'    # offline search across all chats
termgram cached amir 50    # last 50 cached messages
termgram searchall 'x'     # global Telegram search (API)
termgram searchin amir 'x' # search inside one chat (API)
```

Chat names match case-insensitively (`amir` = `AMIR`). Searching a chat that
isn't mirrored auto-syncs it; if the mirror still has nothing, it falls back to
a live API search.

## MCP server

`termgram-mcp` speaks the Model Context Protocol over stdio. Register it in
your MCP client — for opencode:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "termgram": {
      "type": "local",
      "command": ["/home/you/.local/bin/termgram-mcp"],
      "enabled": true
    }
  }
}
```

Or in any standard MCP client:

```json
{
  "mcpServers": {
    "termgram": {
      "command": "/home/you/.local/bin/termgram-mcp",
      "args": []
    }
  }
}
```

### Tools

Every CLI command is exposed as a tool. `send` accepts the same unified model
(`text`, `files`, `format`, `dates`, `reply`, `topic`, `at`), plus `poll`,
`topics`, `watch`, `upload`, `album`, `voice`, `schedule`, `scheduled`,
`cancel`, `edit`, `draft`, `drafts`, `react`, `reactions`, `search` /
`searchin` / `searchall`, `sync`, mirror access, admin tools, and
`export` / `import` / `wipe`.

### Push notifications

The MCP server pushes every new incoming message to the client as a
server→client custom notification:

```
method: notifications/termgram/message
params: {
  "chat":  "-1002155849905",   // dialog id (negative = group/channel)
  "id":    49437,
  "at":    1770000000,          // unix timestamp
  "out":   false,               // true when the message is your own
  "from":  "Jazz/Blues music",  // sender display name
  "text":  "…",
  "media": "photo"              // photo|document|sticker|contact|poll|…, null if none
}
```

Pushes are off by default. Control them with the `notify` tool:

| Call | Effect |
|---|---|
| `notify { on: true, target: "@someone" }` | push only that chat |
| `notify { on: true }` | push every chat |
| `notify { on: false, target: "@someone" }` | stop pushing that chat |
| `notify { on: false }` | stop all pushes |

## SSE stream

`termgram-notify` is a tiny standalone HTTP server (no extra deps, one
process). It streams the same new-message events as `text/event-stream`:

```sh
termgram-notify &          # binds 127.0.0.1:6837
curl -N http://127.0.0.1:6837/events                  # all chats
curl -N 'http://127.0.0.1:6837/events?chat=178220800' # one chat
```

Each event is a single `data:` line — the same JSON shape as the MCP
notification — followed by a blank line. A `: ping` comment heartbeat arrives
every 15 seconds to keep the connection alive. Override the port with
`TERMGRAM_NOTIFY_PORT`, bind filter with the query string, and connect from a
browser, `curl`, or any SSE client.

### A note on streams

An MCP or SSE connection streams the account's incoming events. Messages tied
to an exchange that happens on a different live session — for example another
terminal's running `termgram` process replying to a bot — can stay on that
session's feed instead of reaching your stream. In the common case the same
process that sends also pushes, so nothing is missed. The MTProto layer
guarantees constant streaming, not per-session lockstep.

## How it's built

Single Rust workspace, four crates:

| Crate | Binary | Role |
|---|---|---|
| `crates/core` | — | MTProto client, signals, config, mirror, media, every operation |
| `crates/cli` | `termgram` | the terminal client |
| `crates/mcp` | `termgram-mcp` | MCP server over stdio + push notifications |
| `crates/notify` | `termgram-notify` | SSE HTTP push server |

All three binaries share one session and config. The core exposes a broadcast
`Update` stream so the CLI, MCP server and notifier all wake instantly on new
messages.

## Security

- Credentials (`api_id`, `api_hash`) and your session live only in
  `~/.config/termgram/` — never in this repository.
- The notifier binds to `127.0.0.1` only.
- Nothing in the push payloads stores or forwards your secrets.

## Notes

- Reacting with new emoji requires Telegram Premium; reading reactions works
  for everyone.
- Two termgram processes on the same account share one `session.db`. Run
  long-lived watchers on their own; concurrent write commands may surface
  `database is locked`.

## Contributing

This project is **actively looking for maintainers**. The current maintainer
uses termgram daily but isn't adding features — if you want it to keep
evolving, the maintainer seat can be yours: report bugs, open issues for
direction, send PRs, or volunteer to co-maintain. Start with
[CONTRIBUTING.md](CONTRIBUTING.md). Small PRs merge fast; big ones get
discussed in an issue first.

## License

[MIT](LICENSE)