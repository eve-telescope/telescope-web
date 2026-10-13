//! `POST /api/tokens` and `DELETE /api/tokens/{tokenId}`: a user's own
//! personal access tokens.

use serde_json::json;
use sha2::{Digest, Sha256};

use crate::common::{NOW_Z, TestApp, assert_json_eq};

#[tokio::test]
async fn creates_a_token() {
    let app = TestApp::new().await;
    let user = app.user().await;
    let token = app.token(&user).await;
    let response = app
        .post_json(&token, "/api/tokens", &json!({ "name": "Laptop" }))
        .await;
    response.assert_status(201);
    let body = response.json();
    let created = body["token"].as_str().expect("a token").to_owned();
    assert_json_eq(
        &json!({ "token": created, "name": "Laptop", "id": 2 }),
        &body,
    );
    // `<id>|<40 random><crc32b of them>`, stored as its hash.
    let (id, secret) = created.split_once('|').expect("a pipe");
    assert_eq!(id, "2");
    assert_eq!(secret.len(), 48);
    let (random, checksum) = secret.split_at(40);
    assert!(random.chars().all(|c| c.is_ascii_alphanumeric()));
    assert_eq!(
        checksum,
        format!("{:08x}", crc32fast::hash(random.as_bytes()))
    );
    let row = app
        .row("personal_access_tokens", 2)
        .await
        .expect("the token");
    assert_eq!(row["token"], hex::encode(Sha256::digest(secret.as_bytes())));
    assert_eq!(row["abilities"], "[\"*\"]");
    assert_eq!(row["tokenable_type"], "App\\Models\\User");
    assert_eq!(row["tokenable_id"], user.id);
    assert_eq!(row["last_used_at"], serde_json::Value::Null);
    assert_eq!(row["created_at"], "2026-10-13T08:30:00+00:00");
    app.get_json(&created, "/api/user").await.assert_status(200);
    assert_eq!(NOW_Z, "2026-10-13T08:30:00.000000Z");
}

#[tokio::test]
async fn validates_the_name() {
    let app = TestApp::new().await;
    let user = app.user().await;
    let token = app.token(&user).await;
    for (body, message) in [
        (json!({}), "The name field is required."),
        (json!({ "name": "" }), "The name field is required."),
        (
            json!({ "name": "x".repeat(256) }),
            "The name field must not be greater than 255 characters.",
        ),
        (json!({ "name": 42 }), "The name field must be a string."),
    ] {
        let response = app.post_json(&token, "/api/tokens", &body).await;
        response.assert_status(422);
        assert_json_eq(
            &json!({ "message": message, "errors": { "name": [message] } }),
            &response.json(),
        );
    }
}

#[tokio::test]
async fn revokes_an_own_token() {
    let app = TestApp::new().await;
    let user = app.user().await;
    let token = app.token(&user).await;
    let second = app.token_named(&user, "CLI").await;
    let response = app.delete_json(&token, "/api/tokens/2").await;
    response.assert_status(200);
    assert_json_eq(&json!({ "message": "Token revoked" }), &response.json());
    assert_eq!(app.row("personal_access_tokens", 2).await, None);
    app.get_json(&second, "/api/user").await.assert_status(401);
}

#[tokio::test]
async fn revoking_the_current_token_signs_out() {
    let app = TestApp::new().await;
    let user = app.user().await;
    let token = app.token(&user).await;
    app.delete_json(&token, "/api/tokens/1")
        .await
        .assert_status(200);
    app.get_json(&token, "/api/user").await.assert_status(401);
}

/// Another user's token, or one that does not exist: nothing happens,
/// and the answer is the same.
#[tokio::test]
async fn revoking_someone_elses_token_changes_nothing() {
    let app = TestApp::new().await;
    let user = app.user().await;
    let other = app.user().await;
    let token = app.token(&user).await;
    app.token(&other).await;
    for path in ["/api/tokens/2", "/api/tokens/999"] {
        let response = app.delete_json(&token, path).await;
        response.assert_status(200);
        assert_json_eq(&json!({ "message": "Token revoked" }), &response.json());
    }
    assert_eq!(app.count("personal_access_tokens").await, 2);
}

/// FIXES.md: a TypeError (500) in Laravel.
#[tokio::test]
async fn revoking_a_token_by_a_non_numeric_id_is_not_found() {
    let app = TestApp::new().await;
    let user = app.user().await;
    let token = app.token(&user).await;
    let response = app.delete_json(&token, "/api/tokens/abc").await;
    response.assert_status(404);
    assert_eq!(app.count("personal_access_tokens").await, 1);
}
