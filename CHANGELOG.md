# Changelog

## 0.4.0-kosmopoliton.1 (2026-10-31) — integration of community forks

Base: upstream dgrr/tgcli v0.3.7 (f7686ad). Every commit below was read in full before it was
cherry-picked; see `notes/tgcli-integration-log.md` in the orchestration repo for the per-commit
summaries, trust verdicts and conflict notes.

### Integrated

- **grammers 0.8.1 → 0.10.0, MTProto layer 227** — rafanate20, upstream PR #13 (d6f890f).
  Telegram rejects layer 216, so this is required for anything to work. The session store moves to
  libsql: back up `~/.tgcli/session.db` before the first run.
- **Own Telegram API credentials + hardened local data** — exocognito 379e990 (Sam Collins).
  `TELEGRAM_API_ID` is required for every network command, `TELEGRAM_API_HASH` only for `auth`.
  Store dir forced to `0700`, `session.db`/`tgcli.db` to `0600`, symlinked store paths rejected,
  downloaded filenames sanitized and created with `create_new`. Adds `SECURITY.md`.
  Fork-specific parts (toolchain pin, install.sh/Cargo.toml branding, docs/vm-poc.md) not taken;
  `auth status` keeps upstream's script-friendly `{"authenticated": false, "error": …}` on connect failure.
- **Outbound sends no longer clobber a chat's kind** — exocognito 002487d (fixes PEER_ID_INVALID
  after sending to a group).
- **`chats add --chat --user`** — exocognito 68ae837 (channels.InviteToChannel).
- **Contacts auto-populated from message senders** — Edouard-m-333 a77c322, upstream PR #9.
- **`messages fetch --all`, `--download-media`, repeatable `--chat`** — Edouard-m-333 bd5cf78.
  JSON output of `messages fetch` now has `chats[]` and `total_fetched`.
- **Inline buttons: `messages buttons` / `click` / `latest`** — zhiyue 7f61eb0, c2df11d, 03b72d5,
  upstream PR #11. Use `--output json` when an agent drives these.
- **Daemon startup catch-up** — Edouard-m-333 047d8de (`--no-startup-catchup`).
- **Loss-resistant daemon capture + periodic reconciliation** — Edouard-m-333 8605634.
  Live messages are committed with their raw peer ID before metadata is resolved (new table
  `peer_resolution_queue`); checkpoints are not advanced by partial fetches; every 3 h
  (`--reconcile-interval-seconds`, 0 disables) dialogs are refreshed into grammers' peer cache and
  recent history is reconciled. Two changes to the original: the archived flag comes from the dialog's
  folder instead of a hard-coded `false`, and a store error is logged instead of killing the daemon.

### Not taken

- lrobot 7f4c004 (message archiving + SOCKS5 proxy): monolithic re-do of PR #13 with ~40 new crates,
  env-var proxy selection that breaks when `HTTP(S)_PROXY` is an `http://` proxy. Deferred; a SOCKS5
  option may be re-implemented later as an explicit flag.
- Edouard-m-333 487b6e7, 505c277 ("Northstar"): third-party product hooks and a file-driven control
  channel in the store dir. Rejected.
- exocognito f9cc616 (daemon poll backstop): superseded by 8605634; its `--catch-up` flag could not be
  turned off and it polled every 15 s.
- dragetd PR #8 (928ec71), Edouard-m-333 3dec23c, 5707c26: superseded by 379e990.
- zhiyue release bumps, Edouard-m-333/exocognito fork README/SKILL/docs housekeeping.
