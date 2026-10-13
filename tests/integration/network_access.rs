//! tests/Feature/NetworkAccessTest.php: granting, listing, changing and
//! revoking access, always answered with plain types (`character`, not
//! the PHP class). Plus the fixed unscoped access ids and owner grants
//! (FIXES.md) and the expiry dates.

use serde_json::{Value, json};
use time::macros::datetime;

use crate::common::factories::{NewUser, USER, User};
use crate::common::{TestApp, assert_json_eq};

struct Setup {
    app: TestApp,
    owner: User,
    network: i64,
    owner_access: i64,
    token: String,
}

/// The PHP suite's `beforeEach`.
async fn setup() -> Setup {
    let app = TestApp::new().await;
    app.create_alliance(99_000_001, "Test Alliance", "TSTA")
        .await;
    app.create_corporation(98_000_001, "Test Corp", "TSTC", Some(99_000_001))
        .await;
    let owner = app
        .create_user(NewUser {
            corporation_id: Some(98_000_001),
            alliance_id: Some(99_000_001),
            ..NewUser::default()
        })
        .await;
    let network = app.create_network("Test Network", owner.id).await;
    let owner_access = app.grant_owner(network, &owner).await;
    let token = app.token(&owner).await;
    Setup {
        app,
        owner,
        network,
        owner_access,
        token,
    }
}

fn character(user: &User, corporation: Value, alliance: Value) -> Value {
    json!({
        "id": user.character_id,
        "name": user.character_name,
        "type": "character",
        "ticker": null,
        "corporation": corporation,
        "alliance": alliance,
    })
}

fn test_corp() -> Value {
    json!({ "id": 98_000_001, "name": "Test Corp", "ticker": "TSTC" })
}

fn test_alliance() -> Value {
    json!({ "id": 99_000_001, "name": "Test Alliance", "ticker": "TSTA" })
}

#[tokio::test]
async fn can_grant_character_access_and_returns_correct_json_shape() {
    let s = setup().await;
    let target = s
        .app
        .create_user(NewUser {
            corporation_id: Some(98_000_001),
            alliance_id: Some(99_000_001),
            ..NewUser::default()
        })
        .await;
    let response = s
        .app
        .post_json(
            &s.token,
            &format!("/api/networks/{}/access", s.network),
            &json!({
                "accessible_type": "character",
                "accessible_id": target.character_id,
                "accessible_name": target.character_name,
                "permission": "viewer",
            }),
        )
        .await;
    response.assert_status(201);
    assert_json_eq(
        &json!({
            "id": 2,
            "accessible_type": "character",
            "accessible_id": target.character_id,
            "permission": "viewer",
            "is_owner": false,
            "expires_at": null,
            "entity": character(&target, test_corp(), test_alliance()),
        }),
        &response.json(),
    );
    let row = s.app.row("network_access", 2).await.expect("the grant");
    assert_eq!(row["accessible_type"], USER);
}

#[tokio::test]
async fn can_grant_corporation_access_and_returns_correct_json_shape() {
    let s = setup().await;
    let response = s
        .app
        .post_json(
            &s.token,
            &format!("/api/networks/{}/access", s.network),
            &json!({
                "accessible_type": "corporation",
                "accessible_id": 98_000_001,
                "accessible_name": "Test Corp",
                "permission": "member",
            }),
        )
        .await;
    response.assert_status(201);
    assert_json_eq(
        &json!({
            "id": 2,
            "accessible_type": "corporation",
            "accessible_id": 98_000_001,
            "permission": "member",
            "is_owner": false,
            "expires_at": null,
            "entity": {
                "id": 98_000_001,
                "name": "Test Corp",
                "type": "corporation",
                "ticker": "TSTC",
                "corporation": null,
                "alliance": test_alliance(),
            },
        }),
        &response.json(),
    );
}

#[tokio::test]
async fn can_grant_alliance_access_and_returns_correct_json_shape() {
    let s = setup().await;
    let response = s
        .app
        .post_json(
            &s.token,
            &format!("/api/networks/{}/access", s.network),
            &json!({
                "accessible_type": "alliance",
                "accessible_id": 99_000_001,
                "accessible_name": "Test Alliance",
                "permission": "manager",
            }),
        )
        .await;
    response.assert_status(201);
    assert_json_eq(
        &json!({
            "id": 2,
            "accessible_type": "alliance",
            "accessible_id": 99_000_001,
            "permission": "manager",
            "is_owner": false,
            "expires_at": null,
            "entity": {
                "id": 99_000_001,
                "name": "Test Alliance",
                "type": "alliance",
                "ticker": "TSTA",
                "corporation": null,
                "alliance": null,
            },
        }),
        &response.json(),
    );
}

#[tokio::test]
async fn never_returns_php_class_names_in_accessible_type() {
    let s = setup().await;
    s.app
        .post_json(
            &s.token,
            &format!("/api/networks/{}/access", s.network),
            &json!({
                "accessible_type": "corporation",
                "accessible_id": 98_000_001,
                "accessible_name": "Test Corp",
                "permission": "viewer",
            }),
        )
        .await
        .assert_status(201);
    let response = s
        .app
        .get_json(&s.token, &format!("/api/networks/{}", s.network))
        .await;
    response.assert_status(200);
    // Corporations sort before characters (the PHP class names in order).
    let types: Vec<Value> = response.json()["accesses"]
        .as_array()
        .expect("accesses")
        .iter()
        .map(|access| access["accessible_type"].clone())
        .collect();
    assert_eq!(types, vec![json!("corporation"), json!("character")]);
}

#[tokio::test]
async fn returns_accesses_with_entity_data_in_network_detail() {
    let s = setup().await;
    let response = s
        .app
        .get_json(&s.token, &format!("/api/networks/{}", s.network))
        .await;
    response.assert_status(200);
    assert_json_eq(
        &json!([{
            "id": s.owner_access,
            "accessible_type": "character",
            "accessible_id": s.owner.character_id,
            "permission": "manager",
            "is_owner": true,
            "expires_at": null,
            "entity": character(&s.owner, test_corp(), test_alliance()),
        }]),
        &response.json()["accesses"],
    );
}

#[tokio::test]
async fn returns_formatted_accesses_in_index() {
    let s = setup().await;
    let unknown = s
        .app
        .grant(s.network, USER, 90_555_555, "viewer", false, None)
        .await;
    let response = s
        .app
        .get_json(&s.token, &format!("/api/networks/{}/access", s.network))
        .await;
    response.assert_status(200);
    assert_json_eq(
        &json!([
            {
                "id": s.owner_access,
                "accessible_type": "character",
                "accessible_id": s.owner.character_id,
                "permission": "manager",
                "is_owner": true,
                "expires_at": null,
                "entity": character(&s.owner, test_corp(), test_alliance()),
            },
            {
                "id": unknown,
                "accessible_type": "character",
                "accessible_id": 90_555_555,
                "permission": "viewer",
                "is_owner": false,
                "expires_at": null,
                "entity": null,
            },
        ]),
        &response.json(),
    );
}

#[tokio::test]
async fn returns_formatted_access_after_update() {
    let s = setup().await;
    let target = s.app.user().await;
    let access = s.app.grant_user(s.network, &target, "viewer").await;
    let response = s
        .app
        .put_json(
            &s.token,
            &format!("/api/networks/{}/access/{access}", s.network),
            &json!({ "permission": "member" }),
        )
        .await;
    response.assert_status(200);
    assert_json_eq(
        &json!({
            "id": access,
            "accessible_type": "character",
            "accessible_id": target.character_id,
            "permission": "member",
            "is_owner": false,
            "expires_at": null,
            "entity": character(&target, Value::Null, Value::Null),
        }),
        &response.json(),
    );
}

#[tokio::test]
async fn prevents_modifying_owner_access() {
    let s = setup().await;
    let response = s
        .app
        .put_json(
            &s.token,
            &format!("/api/networks/{}/access/{}", s.network, s.owner_access),
            &json!({ "permission": "viewer" }),
        )
        .await;
    response.assert_status(403);
    assert_json_eq(
        &json!({ "message": "Cannot modify owner access" }),
        &response.json(),
    );
}

#[tokio::test]
async fn prevents_revoking_owner_access() {
    let s = setup().await;
    let response = s
        .app
        .delete_json(
            &s.token,
            &format!("/api/networks/{}/access/{}", s.network, s.owner_access),
        )
        .await;
    response.assert_status(403);
    assert_json_eq(
        &json!({ "message": "Cannot revoke owner access" }),
        &response.json(),
    );
}

#[tokio::test]
async fn can_revoke_non_owner_access() {
    let s = setup().await;
    let target = s.app.user().await;
    let access = s.app.grant_user(s.network, &target, "viewer").await;
    let response = s
        .app
        .delete_json(
            &s.token,
            &format!("/api/networks/{}/access/{access}", s.network),
        )
        .await;
    response.assert_status(200);
    assert_json_eq(&json!({ "message": "Access revoked" }), &response.json());
    assert_eq!(s.app.row("network_access", access).await, None);
}

#[tokio::test]
async fn denies_access_management_to_non_managers() {
    let s = setup().await;
    let viewer = s.app.user().await;
    s.app.grant_user(s.network, &viewer, "viewer").await;
    let token = s.app.token(&viewer).await;
    let response = s
        .app
        .post_json(
            &token,
            &format!("/api/networks/{}/access", s.network),
            &json!({
                "accessible_type": "character",
                "accessible_id": 99_999_999,
                "accessible_name": "Someone",
                "permission": "viewer",
            }),
        )
        .await;
    response.assert_status(403);
    assert_eq!(s.app.count("network_access").await, 2);
}

// ---------------------------------------------------------------------------
// Beyond the PHP suite.
// ---------------------------------------------------------------------------

/// FIXES.md (security): the access id must belong to the network in the
/// URL. Laravel changed another network's row.
#[tokio::test]
async fn updating_an_access_of_another_network_is_not_found() {
    let s = setup().await;
    let stranger = s.app.user().await;
    let other = s.app.create_network("Other", stranger.id).await;
    s.app.grant_owner(other, &stranger).await;
    let victim = s.app.user().await;
    let foreign = s.app.grant_user(other, &victim, "member").await;
    let response = s
        .app
        .put_json(
            &s.token,
            &format!("/api/networks/{}/access/{foreign}", s.network),
            &json!({ "permission": "viewer" }),
        )
        .await;
    response.assert_status(404);
    assert_json_eq(
        &json!({ "message": format!("No query results for model [App\\Models\\NetworkAccess] {foreign}") }),
        &response.json(),
    );
    assert_eq!(
        s.app.row("network_access", foreign).await.expect("kept")["permission"],
        "member"
    );
}

/// FIXES.md (security).
#[tokio::test]
async fn revoking_an_access_of_another_network_is_not_found() {
    let s = setup().await;
    let stranger = s.app.user().await;
    let other = s.app.create_network("Other", stranger.id).await;
    s.app.grant_owner(other, &stranger).await;
    let victim = s.app.user().await;
    let foreign = s.app.grant_user(other, &victim, "member").await;
    let response = s
        .app
        .delete_json(
            &s.token,
            &format!("/api/networks/{}/access/{foreign}", s.network),
        )
        .await;
    response.assert_status(404);
    assert!(s.app.row("network_access", foreign).await.is_some());
}

/// FIXES.md: a grant may not change the owner's row.
#[tokio::test]
async fn granting_access_to_the_owner_is_forbidden() {
    let s = setup().await;
    let manager = s.app.user().await;
    s.app.grant_user(s.network, &manager, "manager").await;
    let token = s.app.token(&manager).await;
    let response = s
        .app
        .post_json(
            &token,
            &format!("/api/networks/{}/access", s.network),
            &json!({
                "accessible_type": "character",
                "accessible_id": s.owner.character_id,
                "accessible_name": s.owner.character_name,
                "permission": "viewer",
                "expires_at": "2027-01-01",
            }),
        )
        .await;
    response.assert_status(403);
    assert_json_eq(
        &json!({ "message": "Cannot modify owner access" }),
        &response.json(),
    );
    let row = s
        .app
        .row("network_access", s.owner_access)
        .await
        .expect("the owner row");
    assert_eq!(row["permission"], "manager");
    assert_eq!(row["expires_at"], Value::Null);
}

/// A grant for a row that exists changes it (and answers 201); leaving
/// out `expires_at` clears the expiry.
#[tokio::test]
async fn granting_again_updates_the_row() {
    let s = setup().await;
    let target = s.app.user().await;
    let access = s
        .app
        .grant(
            s.network,
            USER,
            target.character_id,
            "viewer",
            false,
            Some(datetime!(2027-01-01 00:00:00 UTC)),
        )
        .await;
    let response = s
        .app
        .post_json(
            &s.token,
            &format!("/api/networks/{}/access", s.network),
            &json!({
                "accessible_type": "character",
                "accessible_id": target.character_id,
                "accessible_name": "Whatever",
                "permission": "member",
            }),
        )
        .await;
    response.assert_status(201);
    let body = response.json();
    assert_eq!(body["id"], access);
    assert_eq!(body["permission"], "member");
    assert_eq!(body["expires_at"], Value::Null);
    assert_eq!(s.app.count("network_access").await, 2);
}

#[tokio::test]
async fn an_expiry_is_answered_with_its_offset() {
    let s = setup().await;
    let target = s.app.user().await;
    let response = s
        .app
        .post_json(
            &s.token,
            &format!("/api/networks/{}/access", s.network),
            &json!({
                "accessible_type": "character",
                "accessible_id": target.character_id,
                "accessible_name": target.character_name,
                "permission": "viewer",
                "expires_at": "2027-06-01 12:00:00",
            }),
        )
        .await;
    response.assert_status(201);
    assert_eq!(response.json()["expires_at"], "2027-06-01T12:00:00+00:00");
}

/// FIXES.md: Laravel stored the local time as UTC.
#[tokio::test]
async fn an_expiry_with_an_offset_keeps_its_moment() {
    let s = setup().await;
    let target = s.app.user().await;
    let response = s
        .app
        .post_json(
            &s.token,
            &format!("/api/networks/{}/access", s.network),
            &json!({
                "accessible_type": "character",
                "accessible_id": target.character_id,
                "accessible_name": target.character_name,
                "permission": "viewer",
                "expires_at": "2027-06-01T10:00:00+02:00",
            }),
        )
        .await;
    response.assert_status(201);
    assert_eq!(response.json()["expires_at"], "2027-06-01T08:00:00+00:00");
    let stored: time::OffsetDateTime =
        sqlx::query_scalar("select expires_at from network_access where id = 2")
            .fetch_one(&s.app.pool)
            .await
            .expect("the grant");
    assert_eq!(stored, datetime!(2027-06-01 08:00:00 UTC));
}

/// FIXES.md (deliberate): ISO 8601 dates only, and in the future. As in
/// Laravel, `after:now` reads what PHP's `strtotime` reads (relative
/// words, overflowing days), so a date it can still compare gets one
/// message, and text it cannot read two.
#[tokio::test]
async fn an_expiry_must_be_an_iso_date() {
    let s = setup().await;
    let target = s.app.user().await;
    let invalid = "The expires at field must be a valid date.";
    let past = "The expires at field must be a date after now.";
    for (expires_at, messages) in [
        ("tomorrow", vec![invalid]),
        ("2027-02-30", vec![invalid]),
        ("June 1 2027", vec![invalid, past]),
        ("not a date", vec![invalid, past]),
        ("2026-01-01", vec![past]),
        ("2026-10-13 08:30:00", vec![past]),
    ] {
        let response = s
            .app
            .post_json(
                &s.token,
                &format!("/api/networks/{}/access", s.network),
                &json!({
                    "accessible_type": "character",
                    "accessible_id": target.character_id,
                    "accessible_name": target.character_name,
                    "permission": "viewer",
                    "expires_at": expires_at,
                }),
            )
            .await;
        response.assert_status(422);
        assert_eq!(
            response.json()["errors"],
            json!({ "expires_at": messages }),
            "{expires_at}"
        );
    }
    for expires_at in [
        "2027-06-01",
        "2027-06-01 12:00",
        "2027-06-01T12:00:00Z",
        "2027-06-01T12:00:00.123+01:00",
        "2026-10-13 08:30:01",
    ] {
        let response = s
            .app
            .post_json(
                &s.token,
                &format!("/api/networks/{}/access", s.network),
                &json!({
                    "accessible_type": "character",
                    "accessible_id": target.character_id,
                    "accessible_name": target.character_name,
                    "permission": "viewer",
                    "expires_at": expires_at,
                }),
            )
            .await;
        response.assert_status(201);
    }
}

#[tokio::test]
async fn validates_a_grant() {
    let s = setup().await;
    let response = s
        .app
        .post_json(
            &s.token,
            &format!("/api/networks/{}/access", s.network),
            &json!({}),
        )
        .await;
    response.assert_status(422);
    assert_json_eq(
        &json!({
            "message": "The accessible type field is required. (and 3 more errors)",
            "errors": {
                "accessible_type": ["The accessible type field is required."],
                "accessible_id": ["The accessible id field is required."],
                "accessible_name": ["The accessible name field is required."],
                "permission": ["The permission field is required."],
            },
        }),
        &response.json(),
    );
}

/// An update changes only what it sends: `expires_at: null` clears the
/// expiry, an empty body changes nothing.
#[tokio::test]
async fn an_update_changes_only_what_it_sends() {
    let s = setup().await;
    let target = s.app.user().await;
    let access = s
        .app
        .grant(
            s.network,
            USER,
            target.character_id,
            "member",
            false,
            Some(datetime!(2027-01-01 00:00:00 UTC)),
        )
        .await;
    let path = format!("/api/networks/{}/access/{access}", s.network);
    let nothing = s.app.put_json(&s.token, &path, &json!({})).await;
    nothing.assert_status(200);
    assert_eq!(nothing.json()["permission"], "member");
    assert_eq!(nothing.json()["expires_at"], "2027-01-01T00:00:00+00:00");
    let cleared = s
        .app
        .put_json(&s.token, &path, &json!({ "expires_at": null }))
        .await;
    cleared.assert_status(200);
    assert_eq!(cleared.json()["permission"], "member");
    assert_eq!(cleared.json()["expires_at"], Value::Null);
}
