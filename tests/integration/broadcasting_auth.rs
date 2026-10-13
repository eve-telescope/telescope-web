//! `/broadcasting/auth`: a user who may see an intel
//! network gets the signature for its private channel,
//! `<key>:<hex HMAC-SHA256(secret, "<socket_id>:<channel>")>`, which the
//! websocket server checks on subscribe.

use axum::body::Body;
use axum::http::Method;
use hmac::{Hmac, KeyInit, Mac};
use serde_json::json;
use sha2::Sha256;

use crate::common::factories::{ALLIANCE, CORPORATION, NewUser, USER};
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
    assert_eq!(response.header("content-type"), Some("application/json"));
}

/// The signature for a known secret, socket and channel, written out.
#[tokio::test]
async fn the_signature_is_the_hmac_of_socket_and_channel() {
    let mut s = setup().await;
    assert_eq!(s.network, 1);
    s.app
        .reconfigure(|config| config.pusher.secret = "reference-reverb-secret".to_owned());
    let response = auth(
        &s.app,
        Some(&s.owner),
        "socket_id=123.456&channel_name=private-intel-network.1",
    )
    .await;
    response.assert_status(200);
    assert_eq!(
        response.text(),
        r#"{"auth":"04pcwy13bvcyjoio6mf6:ee15aa967fba401c006ef004e5296e20049206b0d8a2275114ecde47f8c7e884"}"#
    );
}

/// The form works without an Accept header too.
#[tokio::test]
async fn answers_json_without_an_accept_header() {
    let s = setup().await;
    let channel = format!("private-intel-network.{}", s.network);
    let response = s
        .app
        .request(
            Method::POST,
            "/broadcasting/auth",
            header_map(&[
                ("host", HOST),
                ("content-type", "application/x-www-form-urlencoded"),
                ("authorization", &format!("Bearer {}", s.owner)),
            ]),
            Body::from(format!("socket_id=123.456&channel_name={channel}")),
        )
        .await;
    response.assert_status(200);
    assert_json_eq(
        &json!({ "auth": signature("123.456", &channel) }),
        &response.json(),
    );
}

/// Viewers, members of the user's corporation and of its alliance may
/// listen too.
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
    s.app.create_alliance(99_000_001, "Alliance", "ALLY").await;
    s.app
        .create_corporation(98_000_002, "Ally Corp", "ALLY", Some(99_000_001))
        .await;
    let ally = s
        .app
        .create_user(NewUser {
            corporation_id: Some(98_000_002),
            alliance_id: Some(99_000_001),
            ..NewUser::default()
        })
        .await;
    s.app
        .grant(s.network, ALLIANCE, 99_000_001, "member", false, None)
        .await;
    let viewer = s.app.user().await;
    s.app.grant_user(s.network, &viewer, "viewer").await;
    let channel = format!("private-intel-network.{}", s.network);
    for user in [&mate, &ally, &viewer] {
        let token = s.app.token(user).await;
        let response = auth(
            &s.app,
            Some(&token),
            &format!("socket_id=1.2&channel_name={channel}"),
        )
        .await;
        response.assert_status(200);
        assert_json_eq(
            &json!({ "auth": signature("1.2", &channel) }),
            &response.json(),
        );
    }
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
    assert_json_eq(&json!({ "message": "" }), &without_channel.json());
}

/// A channel whose network id is not a number is forbidden.
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

/// A missing or malformed socket id is forbidden, never signed.
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

/// Only private channels are signed: presence channels and channels
/// without the `private-` prefix are refused.
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
