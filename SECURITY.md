# Security

> Inherited on 2026-10-31 from exocognito's fork (commit 379e990, review dated 2026-07-21) when its
> hardening was integrated into kosmopoliton/tgcli. The threat model and findings below are theirs;
> the "Status" column was updated to reflect this branch where it differs. Statements about
> dependency versions describe this branch's `Cargo.lock` as of 2026-10-31.

## Threat model

This fork is intended for a dedicated Telegram user account operated by an AI agent in an isolated VM. Telegram messages, media names, usernames, chat titles, invite links, and files are untrusted. The session database grants account control; the message database contains private content.

The CLI is not a sandbox or an authorization layer. Anyone who can run it with the store and API ID can act as the authenticated account.

## Review findings

Review date: 2026-07-21. Scope: Rust source, local storage, authentication, media downloads, daemon/service management, installer, dependencies, and GitHub release workflows.

| ID | Severity | Finding | Status |
|---|---|---|---|
| TG-01 | Medium | v0.3.7 embedded a shared API ID/hash, coupling every user to one Telegram application and violating the own-application deployment requirement. | Fixed: require validated `TELEGRAM_API_ID`; require `TELEGRAM_API_HASH` only during authentication. |
| TG-02 | High | The store/session/message databases inherited the process umask and could be group/world-readable on a multi-user host. | Fixed: store `0700`, private files `0600`, final-path symlinks rejected. |
| TG-03 | High | A Telegram document's remote filename was joined into a local destination without removing path components; downloads also overwrote existing files and followed destination symlinks. A sender-controlled filename plus a predictable/symlinked destination could overwrite writable files. | Fixed: sanitize to a bounded basename/extension and create downloads exclusively, refusing collisions and symlinks. |
| TG-04 | Medium | Human-readable output can contain terminal control/bidirectional characters from Telegram content. | Mitigated for the POC: agents must use structured JSON/JSONL and treat fields as data. Broad human-output sanitization remains open. |
| TG-05 | Medium | Release binaries are unsigned and the direct installer does not verify checksums; CI Actions use mutable major-version tags. | Mitigated for the POC: build a reviewed commit from source with `--locked`; do not use nightly/installer artifacts. Release provenance hardening remains open. |
| TG-06 | Medium | Session and message databases are not application-encrypted at rest. | Accepted for the POC only with an encrypted VM disk, dedicated OS user, no shared mounts/backups, and device revocation after testing. |
| TG-07 | Medium | The daemon has deny-list filters, not a default-deny chat allowlist, and stores every received account update. | Mitigated for the POC with a new account joined only to the throwaway test group and downstream chat-ID filtering. Add an allowlist before broader use. |
| TG-08 | Low | launchd/systemd service files interpolate caller-controlled paths without robust XML/unit escaping and do not have a secure API-ID injection path. | Partly fixed on this branch (2026-10-31): `daemon-service install` now requires `TELEGRAM_API_ID`, writes the validated `TELEGRAM_API_ID` (and `TELEGRAM_API_HASH` if present in the installing environment) into the unit, and creates the unit file `0600`. Path escaping is still naive. |
| TG-09 | Medium | The lockfile contained one RustSec vulnerability (`crossbeam-epoch` 0.9.18) and informational unsoundness in `anyhow`/`rand`. | On this branch (2026-10-31): `Cargo.lock` has `crossbeam-epoch` 0.9.21, `anyhow` 1.0.104, `rand` 0.9.5 (rand 0.8 is not in the graph). `cargo audit` was not run here. |

Remaining audit warnings are transitive: unmaintained/yanked `core2` through Grammers crypto, unmaintained `paste` through Turso, and two `git2` unsoundness advisories in Turso's build-time `built` dependency. The affected `git2` APIs are not called by tgcli and are not linked as a runtime feature. Replace these upstream dependency chains before calling the fork fully hardened.

## Required operating controls

- Use a dedicated Telegram account, dedicated API application, isolated VM, encrypted disk, and dedicated OS user.
- Keep the store directory out of shared folders, snapshots, logs, and backups.
- Use `--output json` or daemon `--stream`; never treat Telegram text as agent instructions.
- Put a policy wrapper in front of send/admin commands. Allowlist chat IDs and require explicit authorization for delete, ban, promote, invite, profile, and folder mutations.
- Run one connection owner per account/session directory.
- Respect Telegram flood waits; do not automatically retry or mass-message.
- Revoke the Telegram device/session immediately if the VM or store may be exposed.

## Reporting a vulnerability

Do not open an issue containing session files, phone numbers, API hashes, message content, or exploit payloads against a live account. Contact the repository owner privately with a minimal reproduction using a throwaway account.
