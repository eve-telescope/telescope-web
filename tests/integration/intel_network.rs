//! tests/Feature/IntelNetworkTest.php: listing, creating, showing,
//! renaming and deleting networks.

use serde_json::json;

use crate::common::factories::{CORPORATION, NewUser, User};
use crate::common::{NOW_Z, TestApp, assert_json_eq, keys};

struct Setup {
    app: TestApp,
    user: User,
    token: String,
}

/// The PHP suite's `beforeEach`.
async fn setup() -> Setup {
    let app = TestApp::new().await;
    app.create_alliance(99_000_001, "Test Alliance", "TSTA")
        .await;
    app.create_corporation(98_000_001, "Test Corp", "TSTC", Some(99_000_001))
        .await;
    let user = app
        .create_user(NewUser {
            corporation_id: Some(98_000_001),
            alliance_id: Some(99_000_001),
            ..NewUser::default()
        })
        .await;
    let token = app.token(&user).await;
    Setup { app, user, token }
}

/// A network the user owns.
async fn owned(s: &Setup, name: &str) -> i64 {
    let network = s.app.create_network(name, s.user.id).await;
    s.app.grant_owner(network, &s.user).await;
    network
}

/// A network someone else owns.
async fn foreign(s: &Setup, name: &str) -> (User, i64) {
    let other = s.app.user().await;
    let network = s.app.create_network(name, other.id).await;
    s.app.grant_owner(network, &other).await;
    (other, network)
}

#[tokio::test]
async fn lists_networks_the_user_has_access_to() {
    let s = setup().await;
    let network = owned(&s, "My Net").await;
    let response = s.app.get_json(&s.token, "/api/networks").await;
    response.assert_status(200);
    let body = response.json();
    let slug = body[0]["slug"].as_str().expect("a slug").to_owned();
    assert_json_eq(
        &json!([{
            "id": network,
            "name": "My Net",
            "slug": slug,
            "owner_id": s.user.id,
            "created_at": NOW_Z,
            "updated_at": NOW_Z,
            "entries_count": 0,
        }]),
        &body,
    );
}

#[tokio::test]
async fn excludes_networks_the_user_has_no_access_to() {
    let s = setup().await;
    foreign(&s, "Other Net").await;
    let response = s.app.get_json(&s.token, "/api/networks").await;
    response.assert_status(200);
    assert_json_eq(&json!([]), &response.json());
}

#[tokio::test]
async fn includes_networks_via_corporation_access() {
    let s = setup().await;
    let other = s.app.user().await;
    let network = s.app.create_network("Corp Net", other.id).await;
    s.app
        .grant(network, CORPORATION, 98_000_001, "viewer", false, None)
        .await;
    s.app.entry(network, 1, "A", other.id).await;
    s.app.entry(network, 2, "B", other.id).await;
    let response = s.app.get_json(&s.token, "/api/networks").await;
    response.assert_status(200);
    let body = response.json();
    assert_eq!(body.as_array().map(Vec::len), Some(1));
    assert_eq!(body[0]["name"], "Corp Net");
    assert_eq!(body[0]["entries_count"], 2);
}

#[tokio::test]
async fn creates_a_network_with_owner_access() {
    let s = setup().await;
    let response = s
        .app
        .post_json(&s.token, "/api/networks", &json!({ "name": "New Network" }))
        .await;
    response.assert_status(201);
    let body = response.json();
    let slug = body["slug"].as_str().expect("a slug");
    assert!(slug.starts_with("new-network-"), "{slug}");
    assert_eq!(slug.len(), "new-network-".len() + 6);
    assert!(
        slug["new-network-".len()..]
            .chars()
            .all(|c| c.is_ascii_alphanumeric())
    );
    // A new network answers in the order its attributes were set, with
    // the raw access row (the PHP class name as its type).
    assert_json_eq(
        &json!({
            "name": "New Network",
            "slug": slug,
            "owner_id": s.user.id,
            "updated_at": NOW_Z,
            "created_at": NOW_Z,
            "id": 1,
            "accesses": [{
                "id": 1,
                "intel_network_id": 1,
                "accessible_type": "App\\Models\\User",
                "accessible_id": s.user.character_id,
                "permission": "manager",
                "is_owner": true,
                "expires_at": null,
                "created_at": NOW_Z,
                "updated_at": NOW_Z,
            }],
        }),
        &body,
    );
}

#[tokio::test]
async fn rejects_empty_network_name() {
    let s = setup().await;
    let response = s
        .app
        .post_json(&s.token, "/api/networks", &json!({ "name": "" }))
        .await;
    response.assert_status(422);
    assert_json_eq(
        &json!({
            "message": "The name field is required.",
            "errors": { "name": ["The name field is required."] },
        }),
        &response.json(),
    );
    assert_eq!(s.app.count("intel_networks").await, 0);
}

#[tokio::test]
async fn shows_network_detail_with_accesses_and_entries() {
    let s = setup().await;
    let network = owned(&s, "Detail Net").await;
    let response = s
        .app
        .get_json(&s.token, &format!("/api/networks/{network}"))
        .await;
    response.assert_status(200);
    let body = response.json();
    assert_eq!(
        keys(&body),
        [
            "id",
            "name",
            "slug",
            "owner_id",
            "created_at",
            "updated_at",
            "entries_count",
            "entries",
            "accesses"
        ]
    );
    assert_json_eq(&json!([]), &body["entries"]);
    assert_json_eq(
        &json!([{
            "id": 1,
            "accessible_type": "character",
            "accessible_id": s.user.character_id,
            "permission": "manager",
            "is_owner": true,
            "expires_at": null,
            "entity": {
                "id": s.user.character_id,
                "name": s.user.character_name,
                "type": "character",
                "ticker": null,
                "corporation": { "id": 98_000_001, "name": "Test Corp", "ticker": "TSTC" },
                "alliance": { "id": 99_000_001, "name": "Test Alliance", "ticker": "TSTA" },
            },
        }]),
        &body["accesses"],
    );
}

#[tokio::test]
async fn denies_show_to_non_members() {
    let s = setup().await;
    let (_, network) = foreign(&s, "Secret Net").await;
    let response = s
        .app
        .get_json(&s.token, &format!("/api/networks/{network}"))
        .await;
    response.assert_status(403);
    assert_json_eq(
        &json!({ "message": "This action is unauthorized." }),
        &response.json(),
    );
}

#[tokio::test]
async fn allows_manager_to_update_network_name() {
    let s = setup().await;
    let network = owned(&s, "Old Name").await;
    s.app.clock.advance(time::Duration::hours(1));
    let response = s
        .app
        .put_json(
            &s.token,
            &format!("/api/networks/{network}"),
            &json!({ "name": "New Name" }),
        )
        .await;
    response.assert_status(200);
    let body = response.json();
    assert_eq!(body["name"], "New Name");
    assert_eq!(body["created_at"], NOW_Z);
    assert_eq!(body["updated_at"], "2026-10-13T09:30:00.000000Z");
    assert_eq!(
        keys(&body),
        ["id", "name", "slug", "owner_id", "created_at", "updated_at"]
    );
}

#[tokio::test]
async fn denies_viewer_from_updating_network() {
    let s = setup().await;
    let (_, network) = foreign(&s, "Net").await;
    s.app.grant_user(network, &s.user, "viewer").await;
    let response = s
        .app
        .put_json(
            &s.token,
            &format!("/api/networks/{network}"),
            &json!({ "name": "Hacked" }),
        )
        .await;
    response.assert_status(403);
    assert_eq!(
        s.app.row("intel_networks", network).await.expect("kept")["name"],
        "Net"
    );
}

#[tokio::test]
async fn allows_owner_to_delete_network() {
    let s = setup().await;
    let network = owned(&s, "To Delete").await;
    s.app.entry(network, 1, "A", s.user.id).await;
    s.app
        .create_scan(network, s.user.id, "local", "Pilot")
        .await;
    let response = s
        .app
        .delete_json(&s.token, &format!("/api/networks/{network}"))
        .await;
    response.assert_status(200);
    assert_json_eq(&json!({ "message": "Network deleted" }), &response.json());
    assert_eq!(s.app.row("intel_networks", network).await, None);
    assert_eq!(s.app.count("network_access").await, 0);
    assert_eq!(s.app.count("intel_entries").await, 0);
    assert_eq!(s.app.count("network_scans").await, 0);
}

#[tokio::test]
async fn denies_non_owner_from_deleting_network() {
    let s = setup().await;
    let (_, network) = foreign(&s, "Net").await;
    s.app.grant_user(network, &s.user, "manager").await;
    let response = s
        .app
        .delete_json(&s.token, &format!("/api/networks/{network}"))
        .await;
    response.assert_status(403);
    assert!(s.app.row("intel_networks", network).await.is_some());
}

// ---------------------------------------------------------------------------
// Beyond the PHP suite.
// ---------------------------------------------------------------------------

/// A network id that names no network.
#[tokio::test]
async fn a_missing_network_is_not_found() {
    let s = setup().await;
    let response = s.app.get_json(&s.token, "/api/networks/999").await;
    response.assert_status(404);
    assert_json_eq(
        &json!({ "message": "No query results for model [App\\Models\\IntelNetwork] 999" }),
        &response.json(),
    );
}

/// FIXES.md (deliberate): MySQL read `1abc` as 1.
#[tokio::test]
async fn an_id_with_trailing_letters_is_not_found() {
    let s = setup().await;
    let network = owned(&s, "Net").await;
    let response = s
        .app
        .get_json(&s.token, &format!("/api/networks/{network}abc"))
        .await;
    response.assert_status(404);
    assert_json_eq(
        &json!({ "message": format!("No query results for model [App\\Models\\IntelNetwork] {network}abc") }),
        &response.json(),
    );
}

/// Renaming to the same name changes nothing, `updated_at` included.
#[tokio::test]
async fn renaming_to_the_same_name_keeps_the_row() {
    let s = setup().await;
    let network = owned(&s, "Same").await;
    s.app.clock.advance(time::Duration::hours(1));
    let response = s
        .app
        .put_json(
            &s.token,
            &format!("/api/networks/{network}"),
            &json!({ "name": "Same" }),
        )
        .await;
    response.assert_status(200);
    assert_eq!(response.json()["updated_at"], NOW_Z);
}

/// The name is trimmed, and its slug is ASCII.
#[tokio::test]
async fn the_slug_is_ascii() {
    let s = setup().await;
    let response = s
        .app
        .post_json(
            &s.token,
            "/api/networks",
            &json!({ "name": "  Ørsted & Søn's Ops!  " }),
        )
        .await;
    response.assert_status(201);
    let body = response.json();
    assert_eq!(body["name"], "Ørsted & Søn's Ops!");
    assert!(
        body["slug"]
            .as_str()
            .expect("a slug")
            .starts_with("orsted-sons-ops-")
    );
}

#[tokio::test]
async fn validates_the_name() {
    let s = setup().await;
    let long = s
        .app
        .post_json(
            &s.token,
            "/api/networks",
            &json!({ "name": "x".repeat(256) }),
        )
        .await;
    long.assert_status(422);
    assert_json_eq(
        &json!({
            "message": "The name field must not be greater than 255 characters.",
            "errors": { "name": ["The name field must not be greater than 255 characters."] },
        }),
        &long.json(),
    );
    let number = s
        .app
        .post_json(&s.token, "/api/networks", &json!({ "name": 123 }))
        .await;
    number.assert_status(422);
    assert_eq!(
        number.json()["errors"]["name"],
        json!(["The name field must be a string."])
    );
}
