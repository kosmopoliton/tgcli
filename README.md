# tgcli

Telegram CLI tool using [grammers](https://github.com/Lonami/grammers) (MTProto), without a TDLib runtime dependency. This fork tracks [dgrr/tgcli](https://github.com/dgrr/tgcli).

## What's in this fork

Integrated from other forks and upstream PRs (see CHANGELOG.md for provenance):

- **grammers 0.10 / MTProto layer 227** — required; Telegram rejects the old layer.
- **Own Telegram API credentials** (`TELEGRAM_API_ID`, `TELEGRAM_API_HASH`), store dir `0700`, session/db files `0600`, sanitized download filenames.
- **Full-history backfill** — `messages fetch --all`, `--download-media`, repeatable `--chat`.
- **Contacts auto-populated from message senders.**
- **Inline buttons** — `messages buttons`, `messages click`, `messages latest`.
- **Loss-resistant daemon capture** — live updates are committed using their raw peer ID before peer metadata is resolved; startup and periodic (3 h) reconciliation of recent history.
- `chats add` to invite a contact directly; outbound sends no longer clobber a chat's kind.

The fork does not publish a Homebrew formula or stable release yet. Build a reviewed commit from source:

```bash
git clone https://github.com/kosmopoliton/tgcli.git
cd tgcli
git checkout <reviewed-commit>
cargo build --locked --release
mkdir -p ~/.local/bin
install -m 755 target/release/tgcli ~/.local/bin/tgcli
```

Do not use an unpinned nightly artifact for an account session.

## Features

- **Auth**: Phone → code → 2FA authentication
- **Sync**: Incremental sync with checkpoints, stored in libSQL (turso) with FTS5
- **Chats**: List, search, create, join/leave, archive, pin, mute
- **Messages**: List, search (FTS5 + global API), send, edit, delete, forward, download
- **Contacts**: List and search from local DB
- **Admin**: Ban, kick, promote, demote group members
- **Read**: Mark messages as read
- **Stickers**: List, search, send stickers
- **Polls**: Create polls
- **Profile**: Show and update your profile
- **Folders**: Create and manage chat folders
- **Output**: Human-readable tables or `--output json`

## Telegram API credentials

Create an application at [my.telegram.org/apps](https://my.telegram.org/apps). This fork does not embed or share application credentials.

`TELEGRAM_API_ID` is required for every network command. `TELEGRAM_API_HASH` is required only while authenticating. Enter the hash without placing it in shell history:

```bash
# Run in bash (macOS users can start `bash` first).
read -rp "Telegram API ID: " TELEGRAM_API_ID
read -rsp "Telegram API hash: " TELEGRAM_API_HASH; echo
export TELEGRAM_API_ID TELEGRAM_API_HASH

tgcli auth
unset TELEGRAM_API_HASH
```

Both values are validated before use and are never included in tgcli error messages.

## Quick Start

```bash
# Authenticate after exporting both Telegram API variables
tgcli auth

# Sync messages (incremental by default)
tgcli sync

# Full sync (first time or refresh)
tgcli sync --full

# List chats
tgcli chats list

# Search messages locally (FTS5)
tgcli messages search "hello"

# Search messages globally (Telegram API)
tgcli messages search --global "hello"

# Send a message
tgcli send --to <chat_id> --message "Hello!"

# Download media from a message
tgcli messages download --chat <chat_id> --message <msg_id>
```

## Inline Buttons & Bots

Drive bots that reply with inline keyboards (for example search bots that send
a file when you tap a result). These commands read and press buttons over the
network, so they work without a prior `sync`.

```bash
# Show the latest messages straight from Telegram (bypasses the local DB)
tgcli messages latest --chat <chat_id> --limit 10

# List a message's inline keyboard (index, kind, callback data / url)
tgcli messages buttons --chat <chat_id> --message <msg_id>

# Press a callback button by index (from `messages buttons`)
tgcli messages click --chat <chat_id> --message <msg_id> --button <index>

# Press a button, then wait for and download the file the bot sends back
tgcli messages click --chat <chat_id> --message <msg_id> --button <index> \
    --download --wait 45 --dest ./downloads

# Press by raw callback data instead of index (URL-safe base64)
tgcli messages click --chat <chat_id> --message <msg_id> --data <base64>
```

`click` invokes `messages.getBotCallbackAnswer`. Bots that take a while to
respond may surface `BOT_RESPONSE_TIMEOUT`; tgcli treats this as delivered and
still waits for the follow-up message when `--wait`/`--download` is set.

## Sync Behavior

- **First run**: Fetches all chats + last 50 messages per chat (configurable with `--messages-per-chat`)
- **Subsequent runs**: Pure incremental sync — only fetches new messages since last checkpoint
- **`--full`**: Forces a full sync, ignoring checkpoints

```bash
# Default incremental sync
tgcli sync

# Full sync with 100 messages per chat
tgcli sync --full --messages-per-chat 100

# Sync with progress suppressed
tgcli sync --no-progress

# Output as JSONL stream
tgcli sync --stream
```

## Daemon (Optional)

The `daemon` command is **optional** and only needed for real-time message capture.

**When to use `sync` (most use cases):**
- Periodic message fetching (cron, on-demand)
- Catching up on missed messages
- One-time data export
- CLI workflows and scripts

**When to use `daemon`:**
- Instant notifications as messages arrive
- Real-time message processing pipelines
- Live message streaming to external systems
- Continuous monitoring of specific chats

```bash
# Start daemon (listens for real-time updates)
tgcli daemon

# Daemon with JSONL output (for pipelines)
tgcli daemon --stream

# Skip the optional full background sync; durable catch-up and reconciliation remain enabled
tgcli daemon --no-backfill

# Ignore specific chats or all channels
tgcli daemon --ignore 123456789 --ignore-channels

# Skip the startup catch-up sync
tgcli daemon --no-startup-catchup

# Change the periodic reconciliation interval, or disable it with 0
tgcli daemon --reconcile-interval-seconds 10800
```

The daemon maintains a persistent connection to Telegram and stores messages instantly as they arrive. A live message is committed before full chat metadata is resolved; unresolved peers remain in a durable retry queue instead of causing the message to be skipped.

**Automatic reconciliation**: on every (re)start, and every three hours thereafter, the daemon refreshes peer metadata and incrementally reconciles chats active within the last 30 days (plus all archived chats). Telegram update-state catch-up also remains enabled when `--no-backfill` is used. Disable only the startup pass with `--no-startup-catchup`, or only the periodic pass with `--reconcile-interval-seconds 0`.

## Architecture

```
src/
  main.rs          CLI entry point (clap)
  cmd/             Command handlers
    auth.rs        Phone → code → 2FA
    sync.rs        Incremental/full sync
    chats.rs       List/search/create/join/leave/archive/pin/mute
    messages.rs    List/search/send/edit/delete/forward/download
    send.rs        Send text/files/voice/video
    contacts.rs    List/search contacts
    read.rs        Mark as read
    stickers.rs    List/search/send stickers
    polls.rs       Create polls
    profile.rs     Show/update profile
    folders.rs     Create/delete folders
    users.rs       Show/block/unblock users
    typing.rs      Send typing indicator
    completions.rs Shell completions
  store/           turso (libSQL) + FTS5 storage
  tg/              grammers client wrapper
  app/             App struct + business logic
  out/             Output formatting
```

## Storage

- Session: `~/.tgcli/session.db` (grammers SqliteSession)
- Data: `~/.tgcli/tgcli.db` (chats, contacts, messages + FTS5)

The store directory is forced to mode `0700`; session/data files are forced to `0600`; symbolic-link store/session paths are rejected. The databases are not application-encrypted, so use an encrypted VM disk and treat the entire store as an account credential and private message archive.

Multi-account support via `--store`:
```bash
tgcli --store ~/.tgcli-work sync
tgcli --store ~/.tgcli-personal sync
```

Reset local database (keeps session):
```bash
tgcli wipe        # Asks for confirmation
tgcli wipe --yes  # Skip confirmation
```

## Shell Completions

```bash
# Bash
tgcli completions bash > /etc/bash_completion.d/tgcli

# Zsh
tgcli completions zsh > ~/.zfunc/_tgcli

# Fish
tgcli completions fish > ~/.config/fish/completions/tgcli.fish
```

## Agent security

- Use a dedicated Telegram account and a dedicated API application—not a personal account.
- Use `--output json` or daemon `--stream` for machine consumption. Treat every message, filename, username, and chat title as untrusted data, never agent instructions.
- Run one Telegram connection owner per account; do not share the same session directory between concurrent daemons.
- Start in a throwaway group at human messaging rates. Surface Telegram flood/permission errors; do not retry aggressively.
- Downloaded Telegram filenames are reduced to a safe basename before local file creation.
- Review [SECURITY.md](SECURITY.md) before connecting an agent.

## Why Rust?

The Go version (`tgcli-go`) uses TDLib (C++), which complicates distribution. This implementation uses the Rust Grammers stack and produces one tgcli binary from a normal Cargo build.

It uses [Turso](https://github.com/tursodatabase/libsql) for local libSQL/FTS5 storage. The locked dependency graph includes native build dependencies, so a working C toolchain may still be required when building from source.

## License

MIT
