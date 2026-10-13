//! The broadcast events: each mutation publishes
//! its event on the network's private channel right after it is stored,
//! the sender included, and nothing is published when it fails.

use std::time::Duration;

use serde_json::json;

use crate::broadcasting_auth::signature;
use crate::common::socket::PusherClient;
use crate::common::{PUSHER_KEY, TestApp};

struct Setup {
    app: TestApp,
    network: i64,
    other_network: i64,
    member: String,
    viewer: String,
    client: PusherClient,
    other: PusherClient,
}

async fn setup() -> Setup {
    let app = TestApp::new().await;
    let owner = app.user().await;
    let member = app.user().await;
    let viewer = app.user().await;
    let network = app.create_network("Alpha", owner.id).await;
    let other_network = app.create_network("Bravo", owner.id).await;
    app.grant_owner(network, &owner).await;
    app.grant_owner(other_network, &owner).await;
    app.grant_user(network, &member, "member").await;
    app.grant_user(network, &viewer, "viewer").await;
    let address = app.serve().await;
    let client = listen(address, network).await;
    let other = listen(address, other_network).await;
    Setup {
        network,
        other_network,
        member: app.token(&member).await,
        viewer: app.token(&viewer).await,
        client,
        other,
        app,
    }
}

async fn listen(address: std::net::SocketAddr, network: i64) -> PusherClient {
    let (mut client, _) = PusherClient::established(address, PUSHER_KEY).await;
    let socket_id = client.socket_id.clone().expect("a socket id");
    let channel = format!("private-intel-network.{network}");
    client
        .send_json(&json!({
            "event": "pusher:subscribe",
            "data": { "auth": signature(&socket_id, &channel), "channel": channel },
        }))
        .await;
    client.text().await;
    client
}

fn frame(event: &str, network: i64, key: &str, id: i64) -> String {
    format!(
        r#"{{"event":"App\\Events\\{event}","data":"{{\"network_id\":{network},\"{key}\":{id}}}","channel":"private-intel-network.{network}"}}"#
    )
}

#[tokio::test]
async fn entry_changes_are_published() {
    let mut s = setup().await;
    let path = format!("/api/networks/{}/entries", s.network);
    s.app
        .post_json(
            &s.member,
            &path,
            &json!({ "entity_type": "character", "entity_id": 7, "entity_name": "Seven" }),
        )
        .await
        .assert_status(201);
    assert_eq!(
        s.client.text().await,
        frame("IntelEntryCreated", s.network, "entry_id", 1)
    );
    // The upsert of an existing entry is announced as created, too.
    s.app
        .post_json(
            &s.member,
            &path,
            &json!({ "entity_type": "character", "entity_id": 7, "entity_name": "Seven", "label": "X" }),
        )
        .await
        .assert_status(201);
    assert_eq!(
        s.client.text().await,
        frame("IntelEntryCreated", s.network, "entry_id", 1)
    );
    s.app
        .put_json(&s.member, &format!("{path}/1"), &json!({ "label": "Y" }))
        .await
        .assert_status(200);
    assert_eq!(
        s.client.text().await,
        frame("IntelEntryUpdated", s.network, "entry_id", 1)
    );
    s.app
        .delete_json(&s.member, &format!("{path}/1"))
        .await
        .assert_status(200);
    assert_eq!(
        s.client.text().await,
        frame("IntelEntryDeleted", s.network, "entry_id", 1)
    );
    s.other.assert_silent(Duration::from_millis(300)).await;
}

#[tokio::test]
async fn a_shared_scan_is_published() {
    let mut s = setup().await;
    s.app
        .post_json(
            &s.member,
            &format!("/api/networks/{}/scans", s.network),
            &json!({ "scan_type": "dscan", "raw_text": "1\tRifter\tRifter\t-" }),
        )
        .await
        .assert_status(201);
    assert_eq!(
        s.client.text().await,
        frame("ScanShared", s.network, "scan_id", 1)
    );
}

#[tokio::test]
async fn nothing_is_published_when_a_mutation_fails() {
    let mut s = setup().await;
    let path = format!("/api/networks/{}/entries", s.network);
    s.app
        .post_json(
            &s.viewer,
            &path,
            &json!({ "entity_type": "character", "entity_id": 7, "entity_name": "Seven" }),
        )
        .await
        .assert_status(403);
    s.app
        .post_json(&s.member, &path, &json!({ "entity_type": "ship" }))
        .await
        .assert_status(422);
    s.app
        .put_json(&s.member, &format!("{path}/999"), &json!({ "label": "Y" }))
        .await
        .assert_status(404);
    s.app
        .post_json(
            &s.member,
            &format!("/api/networks/{}/scans", s.network),
            &json!({ "scan_type": "local" }),
        )
        .await
        .assert_status(422);
    s.client.assert_silent(Duration::from_millis(500)).await;
}

/// An entry of another network is neither changed
/// nor announced.
#[tokio::test]
async fn a_foreign_entry_is_not_announced() {
    let mut s = setup().await;
    let owner_entry = s.app.entry(s.other_network, 9, "SECRET", 1).await;
    s.app
        .put_json(
            &s.member,
            &format!("/api/networks/{}/entries/{owner_entry}", s.network),
            &json!({ "label": "PWNED" }),
        )
        .await
        .assert_status(404);
    s.client.assert_silent(Duration::from_millis(300)).await;
    s.other.assert_silent(Duration::from_millis(300)).await;
}
