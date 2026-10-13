//! The websocket server beyond the recorded frames: the deliberate
//! differences from Reverb (FIXES.md), its settings, and a client that
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

/// FIXES.md (deliberate): presence channels are refused, even with a
/// correct presence signature.
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

/// FIXES.md (deliberate): a member's client event reaches nobody; one
/// from a socket that is not subscribed gets Reverb's error.
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
/// Reverb's error; one that never subscribed is left alone.
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
