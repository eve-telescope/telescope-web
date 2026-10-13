//! The websocket server's Pusher protocol, frame for frame as the desktop
//! clients and laravel-echo expect it, its settings, and a client that
//! behaves like the v0.5 desktop app's `telescope-core` realtime module.

use std::time::Duration;

use axum::body::Body;
use axum::http::Method;
use serde_json::{Value, json};

use crate::broadcasting_auth::signature;
use crate::common::socket::{Next, PusherClient};
use crate::common::{HOST, PUSHER_KEY, PUSHER_SECRET, TestApp, header_map};

struct Setup {
    app: TestApp,
    network: i64,
    token: String,
    member_token: String,
}

async fn setup() -> Setup {
    let app = TestApp::new().await;
    let owner = app.user().await;
    let member = app.user().await;
    let network = app.create_network("Alpha", owner.id).await;
    app.grant_owner(network, &owner).await;
    app.grant_user(network, &member, "member").await;
    Setup {
        network,
        token: app.token(&owner).await,
        member_token: app.token(&member).await,
        app,
    }
}

async fn subscribe(client: &mut PusherClient, socket_id: &str, channel: &str) {
    client
        .send_json(&json!({
            "event": "pusher:subscribe",
            "data": { "auth": signature(socket_id, channel), "channel": channel },
        }))
        .await;
    assert_eq!(
        client.text().await,
        format!(
            r#"{{"event":"pusher_internal:subscription_succeeded","data":"{{}}","channel":"{channel}"}}"#
        )
    );
}

/// Presence channels are refused, even with a correct presence signature.
#[tokio::test]
async fn a_presence_channel_is_refused() {
    let s = setup().await;
    let address = s.app.serve().await;
    let (mut client, _) = PusherClient::established(address, PUSHER_KEY).await;
    let socket_id = client.socket_id.clone().expect("a socket id");
    let channel = format!("presence-intel-network.{}", s.network);
    let channel_data = r#"{"user_id":"1","user_info":true}"#;
    let mac_input = format!("{socket_id}:{channel}:{channel_data}");
    let auth = {
        use hmac::{Hmac, KeyInit, Mac};
        let mut mac =
            Hmac::<sha2::Sha256>::new_from_slice(PUSHER_SECRET.as_bytes()).expect("a key");
        mac.update(mac_input.as_bytes());
        format!("{PUSHER_KEY}:{}", hex::encode(mac.finalize().into_bytes()))
    };
    client
        .send_json(&json!({
            "event": "pusher:subscribe",
            "data": { "auth": auth, "channel_data": channel_data, "channel": channel },
        }))
        .await;
    assert_eq!(
        client.text().await,
        r#"{"event":"pusher:error","data":"{\"code\":4009,\"message\":\"Connection is unauthorized\"}"}"#
    );
}

/// A member's client event reaches nobody.
#[tokio::test]
async fn client_events_are_not_relayed() {
    let s = setup().await;
    let address = s.app.serve().await;
    let channel = format!("private-intel-network.{}", s.network);
    let (mut sender, _) = PusherClient::established(address, PUSHER_KEY).await;
    let (mut listener, _) = PusherClient::established(address, PUSHER_KEY).await;
    let sender_id = sender.socket_id.clone().expect("a socket id");
    let listener_id = listener.socket_id.clone().expect("a socket id");
    subscribe(&mut sender, &sender_id, &channel).await;
    subscribe(&mut listener, &listener_id, &channel).await;
    sender
        .send_json(&json!({ "event": "client-typing", "data": { "x": 1 }, "channel": channel }))
        .await;
    listener.assert_silent(Duration::from_millis(800)).await;
    sender.assert_silent(Duration::from_millis(200)).await;
}

/// The announced activity timeout and the size limit come from the
/// settings.
#[tokio::test]
async fn follows_its_settings() {
    let mut app = TestApp::new().await;
    app.reconfigure(|config| {
        config.pusher.activity_timeout = 120;
        config.pusher.max_message_size = 100;
    });
    let address = app.serve().await;
    let (mut client, frame) = PusherClient::established(address, PUSHER_KEY).await;
    assert!(frame.contains(r#"\"activity_timeout\":120"#), "{frame}");
    client
        .send(&format!(
            r#"{{"event":"pusher:ping","data":{{"pad":"{}"}}}}"#,
            "x".repeat(100)
        ))
        .await;
    assert_eq!(
        client.next_within(Duration::from_secs(5)).await,
        Next::Closed(Some((1009, "Maximum message size exceeded".to_owned())))
    );
}

/// What the v0.5 desktop client does (telescope-core's realtime.rs):
/// connect with its query, ask `/broadcasting/auth` with its bearer token
/// and a form body, subscribe, match events by their last name segment
/// and read ids from the JSON string `data`; answer the server's ping.
#[tokio::test]
async fn the_v05_desktop_client_gets_its_events() {
    let mut s = setup().await;
    s.app
        .reconfigure(|config| config.pusher.ping_interval = Duration::from_secs(1));
    let address = s.app.serve().await;
    let mut client = PusherClient::connect_path(
        address,
        &format!("/app/{PUSHER_KEY}?protocol=7&client=telescope-rust&version=0.5.1&flash=false"),
    )
    .await;
    let established: Value = serde_json::from_str(&client.text().await).expect("JSON");
    let data: Value =
        serde_json::from_str(established["data"].as_str().expect("string data")).expect("JSON");
    let socket_id = data["socket_id"].as_str().expect("a socket id").to_owned();
    assert_eq!(data["activity_timeout"], 30);
    let channel = format!("private-intel-network.{}", s.network);
    let form = format!("socket_id={socket_id}&channel_name={channel}");
    let auth = s
        .app
        .request(
            Method::POST,
            "/broadcasting/auth",
            header_map(&[
                ("host", HOST),
                ("accept", "application/json"),
                ("content-type", "application/x-www-form-urlencoded"),
                ("authorization", &format!("Bearer {}", s.member_token)),
            ]),
            Body::from(form),
        )
        .await;
    auth.assert_status(200);
    let auth = auth.json()["auth"].as_str().expect("an auth").to_owned();
    client
        .send(
            &json!({ "event": "pusher:subscribe", "data": { "auth": auth, "channel": channel } })
                .to_string(),
        )
        .await;
    let subscribed: Value = serde_json::from_str(&client.text().await).expect("JSON");
    assert_eq!(
        subscribed["event"],
        "pusher_internal:subscription_succeeded"
    );
    assert_eq!(subscribed["channel"], channel.as_str());

    s.app
        .post_json(
            &s.token,
            &format!("/api/networks/{}/entries", s.network),
            &json!({ "entity_type": "character", "entity_id": 7, "entity_name": "Seven" }),
        )
        .await
        .assert_status(201);
    let event: Value = serde_json::from_str(&client.text().await).expect("JSON");
    let name = event["event"].as_str().expect("an event");
    assert_eq!(name.rsplit(['\\', '.']).next(), Some("IntelEntryCreated"));
    assert_eq!(event["channel"], channel.as_str());
    let payload: Value =
        serde_json::from_str(event["data"].as_str().expect("string data")).expect("JSON");
    assert_eq!(payload, json!({ "network_id": s.network, "entry_id": 1 }));

    // The server pings an idle subscriber; a pong keeps it.
    assert_eq!(client.text().await, r#"{"event":"pusher:ping"}"#);
    client.send(r#"{"event":"pusher:pong","data":{}}"#).await;
    assert_eq!(client.text().await, r#"{"event":"pusher:ping"}"#);
}

/// A socket that stays silent after the server's ping is dropped with
/// error 4201; one that never subscribed is left alone.
#[tokio::test]
async fn a_silent_subscriber_is_dropped() {
    let mut s = setup().await;
    s.app
        .reconfigure(|config| config.pusher.ping_interval = Duration::from_secs(1));
    let address = s.app.serve().await;
    let (mut client, _) = PusherClient::established(address, PUSHER_KEY).await;
    let (mut idle, _) = PusherClient::established(address, PUSHER_KEY).await;
    let socket_id = client.socket_id.clone().expect("a socket id");
    subscribe(
        &mut client,
        &socket_id,
        &format!("private-intel-network.{}", s.network),
    )
    .await;
    assert_eq!(client.text().await, r#"{"event":"pusher:ping"}"#);
    assert_eq!(
        client.text().await,
        r#"{"event":"pusher:error","data":"{\"code\":4201,\"message\":\"Pong reply not received in time\"}"}"#
    );
    assert_eq!(
        client.next_within(Duration::from_secs(5)).await,
        Next::Dropped
    );
    idle.assert_silent(Duration::from_millis(500)).await;
}

/// Every subscriber of a channel gets its events.
#[tokio::test]
async fn every_subscriber_gets_the_event() {
    let s = setup().await;
    let address = s.app.serve().await;
    let channel = format!("private-intel-network.{}", s.network);
    let mut clients = Vec::new();
    for _ in 0..3 {
        let (mut client, _) = PusherClient::established(address, PUSHER_KEY).await;
        let socket_id = client.socket_id.clone().expect("a socket id");
        subscribe(&mut client, &socket_id, &channel).await;
        clients.push(client);
    }
    s.app
        .post_json(
            &s.token,
            &format!("/api/networks/{}/scans", s.network),
            &json!({ "scan_type": "local", "raw_text": "Pilot" }),
        )
        .await
        .assert_status(201);
    for client in &mut clients {
        assert_eq!(
            client.text().await,
            format!(
                r#"{{"event":"App\\Events\\ScanShared","data":"{{\"network_id\":{},\"scan_id\":1}}","channel":"{channel}"}}"#,
                s.network
            )
        );
    }
}

const ECHO_QUERY: &str = "?protocol=7&client=js&version=8.4.0&flash=false";

fn established(socket_id: &str) -> String {
    format!(
        r#"{{"event":"pusher:connection_established","data":"{{\"socket_id\":\"{socket_id}\",\"activity_timeout\":30}}"}}"#
    )
}

fn error(code: u16, message: &str) -> String {
    format!(r#"{{"event":"pusher:error","data":"{{\"code\":{code},\"message\":\"{message}\"}}"}}"#)
}

const UNAUTHORIZED: &str = r#"{"event":"pusher:error","data":"{\"code\":4009,\"message\":\"Connection is unauthorized\"}"}"#;
const INVALID: &str =
    r#"{"event":"pusher:error","data":"{\"code\":4200,\"message\":\"Invalid message format\"}"}"#;

/// Connects with laravel-echo's query (or `path`) and reads the
/// established frame, checking it in full.
async fn connect(address: std::net::SocketAddr, path: &str) -> PusherClient {
    let mut client = PusherClient::connect_path(address, path).await;
    let frame = client.text().await;
    let parsed: Value = serde_json::from_str(&frame).expect("JSON");
    let data: Value = serde_json::from_str(parsed["data"].as_str().expect("data")).expect("JSON");
    let socket_id = data["socket_id"].as_str().expect("a socket id").to_owned();
    assert_eq!(frame, established(&socket_id));
    client.socket_id = Some(socket_id);
    client
}

async fn echo_client(address: std::net::SocketAddr) -> PusherClient {
    connect(address, &format!("/app/{PUSHER_KEY}{ECHO_QUERY}")).await
}

/// A socket id is two random numbers up to a billion.
#[tokio::test]
async fn a_socket_id_is_two_random_numbers() {
    let app = TestApp::new().await;
    let address = app.serve().await;
    let client = echo_client(address).await;
    let socket_id = client.socket_id.expect("a socket id");
    let (first, second) = socket_id.split_once('.').expect("two parts");
    for part in [first, second] {
        let number: u64 = part.parse().expect("a number");
        assert!((1..=1_000_000_000).contains(&number), "{socket_id}");
    }
}

/// A connection is established with or without the client's query, and
/// nothing else is sent on its own.
#[tokio::test]
async fn connects_with_or_without_a_query() {
    let app = TestApp::new().await;
    let address = app.serve().await;
    let mut client = echo_client(address).await;
    client.assert_silent(Duration::from_millis(1000)).await;
    connect(address, &format!("/app/{PUSHER_KEY}")).await;
}

/// An unknown key gets error 4001 and the connection is dropped.
#[tokio::test]
async fn an_unknown_key_is_refused() {
    let app = TestApp::new().await;
    let address = app.serve().await;
    let mut client =
        PusherClient::connect_path(address, &format!("/app/wrongkey{ECHO_QUERY}")).await;
    assert_eq!(
        client.text().await,
        error(4001, "Application does not exist")
    );
    assert_eq!(
        client.next_within(Duration::from_secs(5)).await,
        Next::Dropped
    );
}

/// `pusher:ping` is answered with or without data; a pong is not.
#[tokio::test]
async fn answers_pings() {
    let app = TestApp::new().await;
    let address = app.serve().await;
    let mut client = echo_client(address).await;
    client.send(r#"{"event":"pusher:ping","data":{}}"#).await;
    assert_eq!(client.text().await, r#"{"event":"pusher:pong"}"#);
    client.send(r#"{"event":"pusher:ping"}"#).await;
    assert_eq!(client.text().await, r#"{"event":"pusher:pong"}"#);
    client.send(r#"{"event":"pusher:pong","data":{}}"#).await;
    client.assert_silent(Duration::from_millis(1000)).await;
}

/// Anything but a known event gets error 4200, and the connection stays.
#[tokio::test]
async fn refuses_malformed_and_unknown_messages() {
    let app = TestApp::new().await;
    let address = app.serve().await;
    let mut client = echo_client(address).await;
    for message in [
        "not json",
        r#"{"event":"foo","data":{}}"#,
        r#"{"event":"pusher:foo","data":{}}"#,
        r#"{"data":{}}"#,
        "[1,2]",
        r#"{"event":"pusher:subscribe","data":{}}"#,
    ] {
        client.send(message).await;
        assert_eq!(client.text().await, INVALID, "{message}");
    }
    client.send(r#"{"event":"pusher:ping","data":{}}"#).await;
    assert_eq!(client.text().await, r#"{"event":"pusher:pong"}"#);
}

/// A message over the size limit closes the connection with 1009.
#[tokio::test]
async fn an_oversized_message_closes_the_connection() {
    let app = TestApp::new().await;
    let address = app.serve().await;
    let mut client = echo_client(address).await;
    client
        .send(&format!(
            r#"{{"event":"pusher:ping","data":{{"pad":"{}"}}}}"#,
            "x".repeat(20_000)
        ))
        .await;
    assert_eq!(
        client.next_within(Duration::from_secs(5)).await,
        Next::Closed(Some((1009, "Maximum message size exceeded".to_owned())))
    );
}

/// A private channel takes the signature of this socket; subscribing
/// again succeeds again.
#[tokio::test]
async fn subscribes_to_a_private_channel() {
    let s = setup().await;
    let address = s.app.serve().await;
    let mut client = echo_client(address).await;
    let socket_id = client.socket_id.clone().expect("a socket id");
    let channel = format!("private-intel-network.{}", s.network);
    subscribe(&mut client, &socket_id, &channel).await;
    client.assert_silent(Duration::from_millis(1000)).await;
    subscribe(&mut client, &socket_id, &channel).await;
}

/// A missing, wrong or another socket's signature is refused with 4009,
/// and the connection stays.
#[tokio::test]
async fn refuses_a_private_channel_without_its_signature() {
    let s = setup().await;
    let address = s.app.serve().await;
    let channel = format!("private-intel-network.{}", s.network);
    let other = echo_client(address).await;
    let other_signature = signature(other.socket_id.as_deref().expect("a socket id"), &channel);
    for data in [
        json!({ "auth": format!("{PUSHER_KEY}:bad"), "channel": channel }),
        json!({ "channel": channel }),
        json!({ "auth": other_signature, "channel": channel }),
        json!({
            "auth": format!("{PUSHER_KEY}:bad"),
            "channel_data": r#"{"user_id":1}"#,
            "channel": format!("presence-intel-network.{}", s.network),
        }),
    ] {
        let mut client = echo_client(address).await;
        client
            .send_json(&json!({ "event": "pusher:subscribe", "data": data }))
            .await;
        assert_eq!(client.text().await, UNAUTHORIZED, "{data}");
        client.assert_silent(Duration::from_millis(300)).await;
    }
}

/// A public channel needs no signature; `data` may also be a JSON string.
#[tokio::test]
async fn subscribes_to_a_public_channel() {
    let app = TestApp::new().await;
    let address = app.serve().await;
    let subscribed = r#"{"event":"pusher_internal:subscription_succeeded","data":"{}","channel":"public-updates"}"#;
    let mut client = echo_client(address).await;
    client
        .send(r#"{"event":"pusher:subscribe","data":{"channel":"public-updates"}}"#)
        .await;
    assert_eq!(client.text().await, subscribed);
    let mut client = echo_client(address).await;
    client
        .send(r#"{"event":"pusher:subscribe","data":"{\"channel\":\"public-updates\"}"}"#)
        .await;
    assert_eq!(client.text().await, subscribed);
}

/// `pusher:unsubscribe` is silent, and the channel's events stop.
#[tokio::test]
async fn unsubscribing_stops_the_events() {
    let s = setup().await;
    let address = s.app.serve().await;
    let mut client = echo_client(address).await;
    let socket_id = client.socket_id.clone().expect("a socket id");
    let channel = format!("private-intel-network.{}", s.network);
    subscribe(&mut client, &socket_id, &channel).await;
    client
        .send_json(&json!({ "event": "pusher:unsubscribe", "data": { "channel": channel } }))
        .await;
    client.assert_silent(Duration::from_millis(500)).await;
    s.app
        .post_json(
            &s.token,
            &format!("/api/networks/{}/entries", s.network),
            &json!({ "entity_type": "character", "entity_id": 7, "entity_name": "Seven" }),
        )
        .await
        .assert_status(201);
    client.assert_silent(Duration::from_millis(1000)).await;
}

/// A client event from a socket that is not on the channel gets 4009.
#[tokio::test]
async fn a_client_event_off_the_channel_is_refused() {
    let s = setup().await;
    let address = s.app.serve().await;
    let mut client = echo_client(address).await;
    client
        .send_json(&json!({
            "event": "client-typing",
            "data": { "x": 1 },
            "channel": format!("private-intel-network.{}", s.network),
        }))
        .await;
    assert_eq!(
        client.text().await,
        error(4009, "The client is not a member of the specified channel.")
    );
}
