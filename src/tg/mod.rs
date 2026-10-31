use crate::security;
use anyhow::{Context, Result};
use grammers_client::peer::{Group, Peer, User};
use grammers_client::Client;
use grammers_mtsender::SenderPool;
use grammers_session::storages::SqliteSession;
use grammers_session::types::{PeerAuth, PeerRef};
use grammers_session::updates::UpdatesLike;
use grammers_tl_types as tl;
use std::sync::Arc;
use tokio::sync::mpsc;

pub const API_ID_ENV: &str = "TELEGRAM_API_ID";
pub const API_HASH_ENV: &str = "TELEGRAM_API_HASH";

pub fn api_id_from_env() -> Result<i32> {
    parse_api_id(std::env::var(API_ID_ENV).ok().as_deref())
}

pub fn api_hash_from_env() -> Result<String> {
    parse_api_hash(std::env::var(API_HASH_ENV).ok().as_deref())
}

fn parse_api_id(value: Option<&str>) -> Result<i32> {
    let api_id = value
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("{API_ID_ENV} is required"))?
        .parse::<i32>()
        .map_err(|_| anyhow::anyhow!("{API_ID_ENV} must be a positive integer"))?;

    if api_id <= 0 {
        anyhow::bail!("{API_ID_ENV} must be a positive integer");
    }

    Ok(api_id)
}

fn parse_api_hash(value: Option<&str>) -> Result<String> {
    let api_hash = value
        .filter(|value| value.len() == 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| anyhow::anyhow!("{API_HASH_ENV} must be 32 hexadecimal characters"))?;

    Ok(api_hash.to_string())
}

/// A connected Telegram client with its pool runner handle.
pub struct TgClient {
    pub client: Client,
    #[allow(dead_code)]
    pub session: Arc<SqliteSession>,
    pool_handle: tokio::task::JoinHandle<()>,
}

impl TgClient {
    /// Connect with updates support.
    /// Returns the client and an updates receiver.
    pub async fn connect_with_updates(
        session_path: &str,
        api_id: i32,
    ) -> Result<(Self, mpsc::UnboundedReceiver<UpdatesLike>)> {
        security::prepare_private_file(std::path::Path::new(session_path))?;
        let session = Arc::new(SqliteSession::open(session_path).await.with_context(|| {
            format!(
                "Failed to open session database at '{}'. Check file permissions.",
                session_path
            )
        })?);

        let pool = SenderPool::new(Arc::clone(&session) as Arc<SqliteSession>, api_id);
        let client = Client::new(pool.handle);

        let SenderPool {
            runner, updates, ..
        } = pool;

        let pool_handle = tokio::spawn(async move {
            runner.run().await;
        });

        Ok((
            TgClient {
                client,
                session,
                pool_handle,
            },
            updates,
        ))
    }
}

/// Resolve a [`Peer`] to a [`PeerRef`].
///
/// Prefers grammers' own resolution ([`Peer::to_ref`]), which uses the peer's
/// non-`min` auth when it has one and otherwise consults the session cache.
/// When neither is available, fall back to whatever access hash the raw peer
/// carries, which is what the pre-0.10 `PeerRef::from(&peer)` did.
pub async fn peer_to_ref_async(peer: &Peer) -> Result<PeerRef> {
    let cached = peer
        .to_ref()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to look up peer in the session cache: {e}"))?;

    Ok(cached.unwrap_or_else(|| peer_to_ref(peer)))
}

/// Build a [`PeerRef`] from the access hash carried by the raw peer object,
/// falling back to ambient auth.
///
/// Only used when [`Peer::to_ref`] cannot produce a reference; this replicates
/// the pre-0.10 `PeerRef::from(&peer)` behavior.
fn peer_to_ref(peer: &Peer) -> PeerRef {
    let auth = match peer {
        Peer::User(user) => user_auth(user),
        Peer::Channel(channel) => channel_auth(&channel.raw),
        Peer::Group(group) => group_auth(group),
    };
    PeerRef {
        id: peer.id(),
        auth,
    }
}

fn user_auth(user: &User) -> PeerAuth {
    match &user.raw {
        tl::enums::User::User(inner) => inner
            .access_hash
            .map(PeerAuth::from_hash)
            .unwrap_or_default(),
        _ => PeerAuth::default(),
    }
}

/// A [`Group`] wraps `tl::enums::Chat`, which is either a basic group (addressed
/// by ID alone) or a megagroup/gigagroup channel that does need an access hash.
fn group_auth(group: &Group) -> PeerAuth {
    match &group.raw {
        tl::enums::Chat::Channel(channel) => channel_auth(channel),
        tl::enums::Chat::ChannelForbidden(channel) => PeerAuth::from_hash(channel.access_hash),
        _ => PeerAuth::default(),
    }
}

fn channel_auth(channel: &tl::types::Channel) -> PeerAuth {
    channel
        .access_hash
        .map(PeerAuth::from_hash)
        .unwrap_or_default()
}

impl Drop for TgClient {
    fn drop(&mut self) {
        self.client.disconnect();
        self.pool_handle.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_id_requires_a_positive_integer() {
        assert_eq!(parse_api_id(Some("12345")).unwrap(), 12345);

        for value in [None, Some(""), Some("abc"), Some("0"), Some("-1")] {
            let error = parse_api_id(value).unwrap_err().to_string();
            assert!(error.contains("TELEGRAM_API_ID"));
        }
    }

    #[test]
    fn api_hash_requires_32_hexadecimal_characters_without_echoing_it() {
        let valid = "0123456789abcdef0123456789ABCDEF";
        assert_eq!(parse_api_hash(Some(valid)).unwrap(), valid);

        for value in [None, Some(""), Some("not-a-valid-secret")] {
            let error = parse_api_hash(value).unwrap_err().to_string();
            assert!(error.contains("TELEGRAM_API_HASH"));
            if let Some(secret) = value.filter(|secret| !secret.is_empty()) {
                assert!(!error.contains(secret));
            }
        }
    }
}
