//! The token guard: personal access tokens in Sanctum's format
//! (`<id>|<secret>`, stored as the SHA-256 of the secret), checked on
//! every request with `last_used_at` stamped, as every desktop client
//! holds one. And `GET /api/user`.

use axum::body::Body;
use axum::http::Method;
use serde_json::{Value, json};
use time::Duration;
use time::macros::datetime;

use crate::common::factories::{NewUser, sanctum_token};
use crate::common::{HOST, NOW_Z, TestApp, assert_json_eq, header_map};

#[tokio::test]
async fn answers_the_user_of_the_token() {
    let app = TestApp::new().await;
    app.create_corporation(98_000_001, "Test Corp", "TSTC", None)
        .await;
    let user = app
        .create_user(NewUser {
            character_name: Some("José Ørsted".to_owned()),
            corporation_id: Some(98_000_001),
            ..NewUser::default()
        })
        .await;
    let token = app.token(&user).await;
    app.clock.advance(Duration::minutes(10));
    let response = app.get_json(&token, "/api/user").await;
    response.assert_status(200);
    // The owner hash stays hidden.
    assert_json_eq(
        &json!({
            "id": user.id,
            "character_id": user.character_id,
            "character_name": "José Ørsted",
            "corporation_id": 98_000_001,
            "alliance_id": null,
            "last_active_at": null,
            "created_at": NOW_Z,
            "updated_at": NOW_Z,
        }),
        &response.json(),
    );
    // The token's use is stamped (and so its updated_at).
    let row = app
        .row("personal_access_tokens", 1)
        .await
        .expect("the token");
    assert_eq!(row["last_used_at"], "2026-10-13T08:40:00+00:00");
    assert_eq!(row["updated_at"], "2026-10-13T08:40:00+00:00");
}

/// A token row in Sanctum's format, with a secret of the form it
/// creates, signs in: the tokens the desktop clients already hold.
#[tokio::test]
async fn a_sanctum_token_row_signs_in() {
    let app = TestApp::new().await;
    let user = app.user().await;
    let (plain, hash) = sanctum_token(41);
    sqlx::query(
        "insert into personal_access_tokens (id, tokenable_type, tokenable_id, name, token,
            abilities, created_at, updated_at)
         values (41, 'App\\Models\\User', $1, 'Telescope Desktop', $2, '[\"*\"]', $3, $3)",
    )
    .bind(user.id)
    .bind(hash)
    .bind(datetime!(2026-04-01 10:00:00 UTC))
    .execute(&app.pool)
    .await
    .expect("token inserts");
    app.get_json(&plain, "/api/user").await.assert_status(200);
    // Without the id, the secret alone is looked up by its hash.
    let secret = plain.split_once('|').expect("a pipe").1;
    app.get_json(secret, "/api/user").await.assert_status(200);
}

#[tokio::test]
async fn a_wrong_token_is_unauthenticated() {
    let app = TestApp::new().await;
    let user = app.user().await;
    let other = app.user().await;
    let token = app.token(&user).await;
    app.token(&other).await;
    let secret = token.split_once('|').expect("a pipe").1.to_owned();
    for wrong in [
        "1|not-the-token".to_owned(),
        format!("2|{secret}"),
        format!("999|{secret}"),
        format!("abc|{secret}"),
        "1|".to_owned(),
        "".to_owned(),
    ] {
        let response = app.get_json(&wrong, "/api/user").await;
        response.assert_status(401);
        assert_json_eq(&json!({ "message": "Unauthenticated." }), &response.json());
    }
}

#[tokio::test]
async fn an_expired_token_is_unauthenticated() {
    let app = TestApp::new().await;
    let user = app.user().await;
    let token = app.token(&user).await;
    sqlx::query("update personal_access_tokens set expires_at = $1")
        .bind(app.now())
        .execute(&app.pool)
        .await
        .expect("expiry set");
    let response = app.get_json(&token, "/api/user").await;
    response.assert_status(401);
    assert_json_eq(&json!({ "message": "Unauthenticated." }), &response.json());
    // A rejected token is kept and not stamped.
    let row = app
        .row("personal_access_tokens", 1)
        .await
        .expect("the token");
    assert_eq!(row["last_used_at"], Value::Null);
    assert_eq!(row["updated_at"], "2026-10-13T08:30:00+00:00");
    sqlx::query("update personal_access_tokens set expires_at = $1")
        .bind(app.now() + Duration::seconds(1))
        .execute(&app.pool)
        .await
        .expect("expiry set");
    app.get_json(&token, "/api/user").await.assert_status(200);
    let row = app
        .row("personal_access_tokens", 1)
        .await
        .expect("the token");
    assert_eq!(row["last_used_at"], "2026-10-13T08:30:00+00:00");
}

#[tokio::test]
async fn a_guest_gets_401() {
    let app = TestApp::new().await;
    let response = app.json(Method::GET, "/api/networks", None, None).await;
    response.assert_status(401);
    assert_json_eq(&json!({ "message": "Unauthenticated." }), &response.json());
}

/// The guard answers JSON whatever the client accepts.
#[tokio::test]
async fn a_guest_without_a_json_accept_header_gets_401() {
    let app = TestApp::new().await;
    for path in ["/api/user", "/api/networks", "/api/search?query=ab"] {
        let response = app.get(path).await;
        response.assert_status(401);
        assert_json_eq(&json!({ "message": "Unauthenticated." }), &response.json());
    }
    let response = app
        .request(
            Method::POST,
            "/broadcasting/auth",
            header_map(&[
                ("host", HOST),
                ("content-type", "application/x-www-form-urlencoded"),
            ]),
            Body::from("socket_id=1.2&channel_name=private-intel-network.1"),
        )
        .await;
    response.assert_status(401);
}

/// The guard runs before the network is looked up.
#[tokio::test]
async fn a_guest_is_rejected_before_anything_else() {
    let app = TestApp::new().await;
    let response = app
        .json(
            Method::PUT,
            "/api/networks/999/entries/999",
            None,
            Some(&json!({})),
        )
        .await;
    response.assert_status(401);
}
