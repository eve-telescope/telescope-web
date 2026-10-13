//! `/broadcasting/auth` (routes/channels.php): a user who may see an intel
//! network gets the signature for its private channel,
//! `<key>:<hex HMAC-SHA256(secret, "<socket_id>:<channel>")>`, which the
//! websocket server checks on subscribe.

use axum::body::Body;
use axum::http::Method;
use hmac::{Hmac, KeyInit, Mac};
use serde_json::json;
use sha2::Sha256;

use crate::common::factories::{CORPORATION, NewUser, USER};
use crate::common::{HOST, PUSHER_KEY, PUSHER_SECRET, TestApp, assert_json_eq, header_map};

/// The signature the websocket server expects.
pub fn signature(socket_id: &str, channel: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(PUSHER_SECRET.as_bytes()).expect("an HMAC key");
    mac.update(format!("{socket_id}:{channel}").as_bytes());
    format!("{PUSHER_KEY}:{}", hex::encode(mac.finalize().into_bytes()))
}

struct Setup {
    app: TestApp,
    network: i64,
    owner: String,
    outsider: String,
}

async fn setup() -> Setup {
    let app = TestApp::new().await;
    let owner = app.user().await;
    let outsider = app.user().await;
    let network = app.create_network("Alpha", owner.id).await;
    app.grant_owner(network, &owner).await;
    Setup {
        network,
        owner: app.token(&owner).await,
        outsider: app.token(&outsider).await,
        app,
    }
}

/// The form pusher-js and the v0.5 client post.
async fn auth(app: &TestApp, token: Option<&str>, form: &str) -> crate::common::TestResponse {
    let authorization = token.map(|token| format!("Bearer {token}"));
    let mut headers = vec![
        ("host", HOST),
        ("accept", "application/json"),
        ("content-type", "application/x-www-form-urlencoded"),
    ];
    if let Some(authorization) = &authorization {
        headers.push(("authorization", authorization));
    }
    app.request(
        Method::POST,
        "/broadcasting/auth",
        header_map(&headers),
        Body::from(form.to_owned()),
    )
    .await
}

#[tokio::test]
async fn signs_the_channel_of_a_network_the_user_may_see() {
    let s = setup().await;
    let channel = format!("private-intel-network.{}", s.network);
    let response = auth(
        &s.app,
        Some(&s.owner),
        &format!("socket_id=123.456&channel_name={channel}"),
    )
    .await;
    response.assert_status(200);
    assert_json_eq(
        &json!({ "auth": signature("123.456", &channel) }),
        &response.json(),
    );
    assert_eq!(response.header("access-control-allow-origin"), Some("*"));
}

/// Viewers and members of the user's corporation may listen too.
#[tokio::test]
async fn any_permission_may_listen() {
    let s = setup().await;
    s.app
        .create_corporation(98_000_001, "Corp", "CORP", None)
        .await;
    let mate = s
        .app
        .create_user(NewUser {
            corporation_id: Some(98_000_001),
            ..NewUser::default()
        })
        .await;
    s.app
        .grant(s.network, CORPORATION, 98_000_001, "viewer", false, None)
        .await;
    let token = s.app.token(&mate).await;
    let channel = format!("private-intel-network.{}", s.network);
    let response = auth(
        &s.app,
        Some(&token),
        &format!("socket_id=1.2&channel_name={channel}"),
    )
    .await;
    response.assert_status(200);
}

#[tokio::test]
async fn others_may_not_listen() {
    let s = setup().await;
    let lapsed = s.app.user().await;
    s.app
        .grant(
            s.network,
            USER,
            lapsed.character_id,
            "member",
            false,
            Some(s.app.now()),
        )
        .await;
    let lapsed = s.app.token(&lapsed).await;
    for (token, channel) in [
        (&s.outsider, format!("private-intel-network.{}", s.network)),
        (&lapsed, format!("private-intel-network.{}", s.network)),
        (&s.owner, "private-intel-network.999".to_owned()),
        (&s.owner, "private-somewhere.1".to_owned()),
    ] {
        let response = auth(
            &s.app,
            Some(token),
            &format!("socket_id=1.2&channel_name={channel}"),
        )
        .await;
        response.assert_status(403);
        assert_json_eq(&json!({ "message": "" }), &response.json());
    }
    let without_channel = auth(&s.app, Some(&s.owner), "socket_id=1.2").await;
    without_channel.assert_status(403);
}

/// FIXES.md: a TypeError (500) in Laravel.
#[tokio::test]
async fn a_malformed_network_id_is_forbidden() {
    let s = setup().await;
    for channel in [
        "private-intel-network.abc",
        "private-intel-network.1.2",
        "private-intel-network.",
    ] {
        let response = auth(
            &s.app,
            Some(&s.owner),
            &format!("socket_id=1.2&channel_name={channel}"),
        )
        .await;
        response.assert_status(403);
    }
}

/// FIXES.md: pusher-php-server threw on a socket id it could not read
/// (500), or signed a missing one.
#[tokio::test]
async fn a_malformed_socket_id_is_forbidden() {
    let s = setup().await;
    let channel = format!("private-intel-network.{}", s.network);
    for form in [
        format!("channel_name={channel}"),
        format!("socket_id=abc&channel_name={channel}"),
        format!("socket_id=1.2.3&channel_name={channel}"),
    ] {
        auth(&s.app, Some(&s.owner), &form).await.assert_status(403);
    }
}

/// FIXES.md (deliberate): presence channels are refused; Laravel signed
/// them (and any channel without the `private-` prefix) as presence
/// channels.
#[tokio::test]
async fn a_presence_channel_is_refused() {
    let s = setup().await;
    for channel in [
        format!("presence-intel-network.{}", s.network),
        format!("intel-network.{}", s.network),
    ] {
        let response = auth(
            &s.app,
            Some(&s.owner),
            &format!("socket_id=1.2&channel_name={channel}"),
        )
        .await;
        response.assert_status(403);
    }
}

#[tokio::test]
async fn a_guest_is_unauthenticated() {
    let s = setup().await;
    let response = auth(
        &s.app,
        None,
        &format!(
            "socket_id=1.2&channel_name=private-intel-network.{}",
            s.network
        ),
    )
    .await;
    response.assert_status(401);
    assert_json_eq(&json!({ "message": "Unauthenticated." }), &response.json());
}

/// GET with the query, and a JSON body, work like the form.
#[tokio::test]
async fn reads_the_query_and_json_too() {
    let s = setup().await;
    let channel = format!("private-intel-network.{}", s.network);
    let get = s
        .app
        .get_json(
            &s.owner,
            &format!("/broadcasting/auth?socket_id=5.6&channel_name={channel}"),
        )
        .await;
    get.assert_status(200);
    assert_eq!(get.json()["auth"], signature("5.6", &channel));
    let posted = s
        .app
        .post_json(
            &s.owner,
            "/broadcasting/auth",
            &json!({ "socket_id": "5.6", "channel_name": channel }),
        )
        .await;
    posted.assert_status(200);
    assert_eq!(posted.json()["auth"], signature("5.6", &channel));
}
