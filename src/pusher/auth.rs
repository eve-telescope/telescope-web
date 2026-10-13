//! `/broadcasting/auth`: a user who may see an intel network gets the
//! signature of its private channel for a socket,
//! `<key>:<hex HMAC-SHA256(secret, "<socket_id>:<channel>")>`. Anything
//! else is 403 with an empty message, presence channels and names without
//! `private-` included.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use hmac::{Hmac, KeyInit, Mac};
use serde_json::json;
use sha2::Sha256;

use crate::auth::tokens::ApiUser;
use crate::input::Input;
use crate::intel::permission;
use crate::server::AppState;
use crate::server::errors;

/// `HMAC-SHA256(secret, text)` in hex.
pub fn sign(secret: &str, text: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("any key length");
    mac.update(text.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// The `auth` value a socket presents to subscribe to a private channel.
pub fn channel_auth(key: &str, secret: &str, socket_id: &str, channel: &str) -> String {
    format!("{key}:{}", sign(secret, &format!("{socket_id}:{channel}")))
}

/// A socket id as Pusher writes it: two numbers joined by a dot.
pub fn is_socket_id(text: &str) -> bool {
    text.split_once('.').is_some_and(|(a, b)| {
        !a.is_empty()
            && !b.is_empty()
            && a.bytes().all(|c| c.is_ascii_digit())
            && b.bytes().all(|c| c.is_ascii_digit())
    })
}

/// The network of `private-intel-network.<id>`.
fn network_of(channel: &str) -> Option<i64> {
    let id = channel.strip_prefix("private-intel-network.")?;
    (!id.is_empty() && id.bytes().all(|c| c.is_ascii_digit()))
        .then(|| id.parse().ok())
        .flatten()
}

fn denied() -> Response {
    errors::json_error(StatusCode::FORBIDDEN, "")
}

pub async fn authenticate(
    State(state): State<AppState>,
    caller: ApiUser,
    input: Input,
) -> Response {
    let (Some(channel), Some(socket_id)) = (input.str("channel_name"), input.str("socket_id"))
    else {
        return denied();
    };
    if !is_socket_id(socket_id) {
        return denied();
    }
    let Some(network) = network_of(channel) else {
        return denied();
    };
    let now = state.clock.now();
    match permission::of(&state.pool, network, &caller.user, now).await {
        Ok(Some(_)) => {
            let pusher = &state.config.pusher;
            Json(json!({
                "auth": channel_auth(&pusher.key, &pusher.secret, socket_id, channel),
            }))
            .into_response()
        }
        Ok(None) => denied(),
        Err(error) => errors::server_error(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_socket_ids_and_channels() {
        assert!(is_socket_id("123.456"));
        assert!(!is_socket_id("abc"));
        assert!(!is_socket_id("1.2.3"));
        assert!(!is_socket_id(".2"));
        assert_eq!(network_of("private-intel-network.12"), Some(12));
        assert_eq!(network_of("private-intel-network.abc"), None);
        assert_eq!(network_of("presence-intel-network.12"), None);
        assert_eq!(network_of("intel-network.12"), None);
    }
}
