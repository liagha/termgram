# termgram

A telegram client for your terminal. One unified send pipeline for text,
files, media, captions, date chips and scheduling, a searchable local mirror,
rich formats, polls, forum topics, drafts, and a full MCP server.

## Build

```sh
cargo build --release
```

## Login

```sh
termgram login            # send a code to your phone
termgram login 12345      # verify it
```

Sessions and config live in `~/.config/termgram/`.

## Send anything, one command

```sh
termgram send saved 'hello'                        # plain text
termgram send saved '**bold** text' --format md    # rich text
termgram send saved 'see 14/09 at 17:00' --date '14/09 17:00'
termgram send saved 'photo' --file shot.png        # photo / document / voice note by type
termgram send saved --file a.png --file b.png      # album (plain caption only)
termgram send amir.a 'in an hour' --at '16:00'     # scheduled
termgram send saved 'in a topic' --topic 42        # forum topic
```

`--date` repeats for multiple tappable date chips. `--reply <id>` replies to a
message. Formats are `plain`, `md`, and `html`.

## Text formats

Markdown and HTML are parsed into real Telegram entities. Dates rendered this
way are tappable chips, not raw text.

## Polls

```sh
termgram poll saved 'which one' red green blue
termgram poll saved 'any number' one two three --multi --anon
```

Needs 2 to 10 options.

## Forum topics

```sh
termgram topics <chat>      # list topics as id + title
```

Combine with `send --topic <id>`.

## Reactions

```sh
termgram react saved <id> 🎉           # react with an emoji
termgram react saved <id> ❤️ --big     # big animated reaction
termgram react saved                   # react to the last message
termgram reactions saved <id>          # list who reacted with what
```

Reacting requires Telegram Premium; reading reactions works regardless.

## Local search mirror

```sh
termgram sync                # index dialogs + messages
termgram search 'query'      # search everything
termgram search --chat <chat> 'query'
termgram cached <chat> <n>   # last n cached messages
termgram searchall           # global Telegram search (API)
termgram searchin <chat>     # search inside one chat (API)
```

Chat names are matched case-insensitively (`amir.a` = `AMIR.A`). Searching a
chat that is not mirrored auto-syncs it first; if the mirror still has nothing
for that chat, falls back to a live API search.

## More commands

```
me men dialogs messages read watch
send edit delete forward pin react reactions
drafts draft profile set block unblock
typing status scheduled cancel
contacts folders folder-new folder-rm pinned members
kick ban unban promote
grab export import wipe
poll topics
```

`watch <target>` tails a chat live; `watch <target> --once` prints a snapshot.
`watch --unread` surveys chats with unread messages (refreshes every 30s by
default, `--every <secs>` to change).

## MCP server

Run `termgram-mcp` under stdio and register it with your MCP client:

```json
{
  "mcpServers": {
    "termgram": {
      "command": "/path/to/termgram-mcp",
      "args": []
    }
  }
}
```

Every CLI command is exposed as a tool: `send` accepts the same unified model
(`text`, `files`, `format`, `dates`, `reply`, `topic`, `at`), plus
`poll`, `topics`, `watch` (snapshot or unread survey), `upload`, `album`,
`voice`, `schedule`, `scheduled`, `cancel`, `edit`, `draft`, `react`,
`reactions`, `search`/`searchin`, `sync`, mirror access, admin tools, and
`export`/`import`/`wipe`.

## Session lock note

Two termgram processes on the same account at once share one session.db.
Run `watch` in follow mode by itself; concurrent write commands may hit
`database is locked`.