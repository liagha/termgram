# Contributing

termgram works and is in use every day, but it is a side project: the
maintainer's time mostly goes to using it, not building it. If you want the
project to keep moving, drive it.

## Looking for maintainers

- Report bugs with repro steps (`cmd`, chat, expected vs actual) and paste the
  `RUST_BACKTRACE=1` output if there is one.
- Open issues before big changes so the direction is agreed first; small PRs
  are merged fast.
- Interested in co-maintaining, releases, or CI? Say so on any issue or PR.

## Dev setup

```sh
cargo build --release
cargo clippy --all -- -D warnings
```

Rust stable, edition 2024. There is no test suite yet; verification is manual.
A PR that adds tests is the most useful thing you can send.

## Project rules

These keep the codebase small, fast and consistent. Please respect them in
every change.

- **One unified `Send` model.** The CLI and MCP server share the same send
  pipeline (`crates/core/src/send.rs`): same args, same formats
  (`plain`/`md`/`html`), same date chips, same scheduling. Never fork it for
  one surface.
- **CLI ⇄ MCP parity.** Every new tool in `termgram-mcp` should be a CLI
  command too, and vice versa. The MCP layer is a thin wrapper, not a second
  product.
- **Keep it lean.** No new heavyweight deps for small jobs (see
  `crates/notify`: the SSE server is hand-rolled HTTP in ~150 lines). Prefer
  the simplest thing that makes the feature correct.
- **Push, not polling.** New-message delivery flows through the core
  `Update` broadcast channel. Wire into it; don't add poll loops.
- **No secrets.** Credentials and sessions stay in a user's
  `~/.config/termgram/`. Nothing in the repo, nothing in logs.

## Commits

Small, focused commits, one logical change each. Message style:
`termgram: <what changed>`.

## License

MIT; see [LICENSE](LICENSE).