//! Reverb's Pusher protocol frames, recorded from the reference
//! (tests/fixtures/golden/pusher, scripts/laravel-reference/pusher.mjs),
//! replayed against the api's websocket with a real client and compared
//! byte for byte, socket ids aside (N4). Subscriptions are signed for the
//! replay's socket ids; HTTP steps run against the same app, so the event
//! frames they cause are compared too. FIXES.md lists the frames that
//! differ on purpose (presence channels, client events).

use std::collections::HashMap;
use std::time::Duration;

use axum::http::Method;
use serde_json::Value;
use time::macros::datetime;

use crate::broadcasting_auth::signature;
use crate::common::TestApp;
use crate::common::golden::{dataset_token, fixture_names, read_json};
use crate::common::socket::{Next, PusherClient};

/// Fixtures whose answer differs on purpose (FIXES.md, the realtime
/// server is part of the api), with the test that pins the new one.
const CHANGED: [(&str, &str); 2] = [
    (
        "subscribe_presence",
        "pusher_protocol::a_presence_channel_is_refused",
    ),
    (
        "client_event",
        "pusher_protocol::client_events_are_not_relayed",
    ),
];

/// The longest a replay waits where Reverb stayed silent.
const SILENCE: Duration = Duration::from_millis(1200);

fn fill(template: &str, socket_ids: &HashMap<String, String>, client: &str) -> String {
    let mut text = template.replace("<20000 x>", &"x".repeat(20_000));
    // `<auth:b:channel>` signs for another client's socket.
    while let Some(start) = text.find("<auth:") {
        let end = text[start..].find('>').expect("a closing >") + start;
        let inner = &text[start + "<auth:".len()..end];
        let (socket_client, channel) = match inner.split_once(':') {
            Some((other, channel)) if socket_ids.contains_key(other) => (other, channel),
            _ => (client, inner),
        };
        let signed = signature(&socket_ids[socket_client], channel);
        text.replace_range(start..=end, &signed);
    }
    text
}

fn normalize(text: &str, socket_ids: &HashMap<String, String>) -> String {
    let mut text = text.to_owned();
    for (client, socket_id) in socket_ids {
        text = text.replace(socket_id.as_str(), &format!("<socket_id:{client}>"));
    }
    text
}

async fn replay(name: &str) {
    let fixture = read_json(&format!("golden/pusher/{name}.json"));
    let mut app = TestApp::with_dataset(datetime!(2026-10-13 12:00:00 UTC)).await;
    if name.starts_with("server_ping") {
        // Reverb pinged on a 60 second timer with a 3 second interval;
        // the replay uses a 1 second interval for both.
        app.reconfigure(|config| config.pusher.ping_interval = Duration::from_secs(1));
    }
    let address = app.serve().await;
    let mut clients: HashMap<String, PusherClient> = HashMap::new();
    let mut socket_ids: HashMap<String, String> = HashMap::new();
    let mut closed: std::collections::HashSet<String> = std::collections::HashSet::new();
    for step in fixture["steps"].as_array().expect("steps") {
        if let Some(request) = step.get("http") {
            let method: Method = request["method"]
                .as_str()
                .expect("a method")
                .parse()
                .expect("a method");
            let token = dataset_token(request["token"].as_i64().expect("a token"));
            let body = Some(&request["body"]).filter(|body| !body.is_null());
            let response = app
                .json(
                    method,
                    request["path"].as_str().expect("a path"),
                    Some(&token),
                    body,
                )
                .await;
            assert_eq!(
                response.status.as_u16() as u64,
                step["status"].as_u64().expect("a status"),
                "{name}: {}",
                response.text()
            );
            continue;
        }
        let Some(client) = step["client"].as_str() else {
            continue;
        };
        if let Some(path) = step["connect"].as_str() {
            clients.insert(
                client.to_owned(),
                PusherClient::connect_path(address, path).await,
            );
        } else if let Some(text) = step["send"].as_str() {
            let text = fill(text, &socket_ids, client);
            clients.get_mut(client).expect("a client").send(&text).await;
        } else if let Some(expected) = step["receive"].as_str() {
            let socket = clients.get_mut(client).expect("a client");
            let next = socket.next_within(Duration::from_secs(5)).await;
            let Next::Text(frame) = next else {
                panic!("{name}: expected {expected}, got {next:?}");
            };
            if let Ok(parsed) = serde_json::from_str::<Value>(&frame)
                && parsed["event"] == "pusher:connection_established"
            {
                let data: Value =
                    serde_json::from_str(parsed["data"].as_str().expect("data")).expect("JSON");
                socket_ids.insert(
                    client.to_owned(),
                    data["socket_id"].as_str().expect("a socket id").to_owned(),
                );
            }
            assert_eq!(normalize(&frame, &socket_ids), expected, "{name}");
        } else if step.get("silent").is_some() {
            let socket = clients.get_mut(client).expect("a client");
            let next = socket.next_within(SILENCE).await;
            // After its close frame Reverb left the connection open; the
            // rewrite ends it, which is just as silent.
            let closed_before = closed.contains(client);
            assert!(
                next == Next::Silent || (closed_before && next == Next::Dropped),
                "{name}: expected silence, got {next:?}"
            );
        } else if let Some(closed_step) = step.get("closed") {
            closed.insert(client.to_owned());
            let closed = closed_step;
            let socket = clients.get_mut(client).expect("a client");
            let next = socket.next_within(Duration::from_secs(5)).await;
            let code = closed["code"].as_u64().expect("a code") as u16;
            if code == 1006 {
                // The connection dropped without a close frame.
                assert_eq!(next, Next::Dropped, "{name}");
            } else {
                assert_eq!(
                    next,
                    Next::Closed(Some((
                        code,
                        closed["reason"].as_str().expect("a reason").to_owned()
                    ))),
                    "{name}"
                );
            }
        }
    }
}

/// A socket id is two random numbers, as Reverb made them.
#[tokio::test]
async fn socket_ids_look_like_reverbs() {
    let app = TestApp::new().await;
    let address = app.serve().await;
    let (_, frame) = PusherClient::established(address, crate::common::PUSHER_KEY).await;
    let parsed: Value = serde_json::from_str(&frame).expect("JSON");
    let data: Value = serde_json::from_str(parsed["data"].as_str().expect("data")).expect("JSON");
    let socket_id = data["socket_id"].as_str().expect("a socket id");
    let (first, second) = socket_id.split_once('.').expect("two parts");
    for part in [first, second] {
        let number: u64 = part.parse().expect("a number");
        assert!((1..=1_000_000_000).contains(&number), "{socket_id}");
    }
}

macro_rules! golden {
    ($($name:ident),* $(,)?) => {
        $(
            #[tokio::test]
            async fn $name() {
                replay(stringify!($name)).await;
            }
        )*

        /// Every recorded fixture has its test above, or is listed as
        /// changed with the test that pins its new answer.
        #[test]
        fn every_fixture_is_replayed() {
            let tested = [$(stringify!($name)),*];
            for name in fixture_names("pusher") {
                assert!(
                    tested.contains(&name.as_str())
                        || CHANGED.iter().any(|(changed, _)| *changed == name),
                    "no golden test for {name}"
                );
            }
        }
    };
}

golden!(
    client_event_unsubscribed,
    connect,
    connect_without_query,
    entry_events,
    invalid_json,
    no_event_on_failure,
    oversized_message,
    ping,
    scan_event,
    server_ping,
    server_ping_answered,
    server_ping_unsubscribed,
    subscribe_presence_bad_auth,
    subscribe_private,
    subscribe_private_bad_auth,
    subscribe_private_other_socket,
    subscribe_private_twice,
    subscribe_private_without_auth,
    subscribe_public,
    subscribe_string_data,
    subscribe_without_channel,
    unknown_event,
    unsubscribe,
    wrong_key,
);
