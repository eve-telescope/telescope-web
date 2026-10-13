//! tests/Feature/IntelEntryTest.php: creating, updating, deleting and
//! listing a network's entries, and the bulk lookup across networks. Plus
//! the fixed unscoped entry ids (FIXES.md), clearing a label, the v0.5
//! client's update and PATCH staying 405.

use serde_json::{Value, json};

use crate::common::factories::{NewEntry, NewUser, User};
use crate::common::{NOW_Z, TestApp, assert_json_eq};

struct Setup {
    app: TestApp,
    owner: User,
    member: User,
    viewer: User,
    network: i64,
    member_token: String,
    viewer_token: String,
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
    let member = app
        .create_user(NewUser {
            corporation_id: Some(98_000_001),
            ..NewUser::default()
        })
        .await;
    let viewer = app.user().await;
    let network = app.create_network("Test Network", owner.id).await;
    app.grant_owner(network, &owner).await;
    app.grant_user(network, &member, "member").await;
    app.grant_user(network, &viewer, "viewer").await;
    let member_token = app.token(&member).await;
    let viewer_token = app.token(&viewer).await;
    Setup {
        app,
        owner,
        member,
        viewer,
        network,
        member_token,
        viewer_token,
    }
}

/// An entry as the API answers it once it is stored: columns in order,
/// `added_by` replaced by the user who added it.
fn stored_entry(id: i64, network: i64, entity: Value, added_by: &User, updated_at: &str) -> Value {
    json!({
        "id": id,
        "intel_network_id": network,
        "entity_type": entity["entity_type"],
        "entity_id": entity["entity_id"],
        "entity_name": entity["entity_name"],
        "color": entity["color"],
        "label": entity["label"],
        "notes": entity["notes"],
        "added_by": added_by.json(),
        "created_at": NOW_Z,
        "updated_at": updated_at,
    })
}

#[tokio::test]
async fn allows_member_to_create_an_entry() {
    let s = setup().await;
    let response = s
        .app
        .post_json(
            &s.member_token,
            &format!("/api/networks/{}/entries", s.network),
            &json!({
                "entity_type": "character",
                "entity_id": 12345,
                "entity_name": "Bad Guy",
                "color": "#FF3B3B",
                "label": "HOSTILE",
            }),
        )
        .await;
    response.assert_status(201);
    // A new entry answers in the order its attributes were set.
    assert_json_eq(
        &json!({
            "entity_type": "character",
            "entity_id": 12345,
            "entity_name": "Bad Guy",
            "color": "#FF3B3B",
            "label": "HOSTILE",
            "notes": null,
            "added_by": s.member.json(),
            "intel_network_id": s.network,
            "updated_at": NOW_Z,
            "created_at": NOW_Z,
            "id": 1,
        }),
        &response.json(),
    );
}

#[tokio::test]
async fn allows_note_only_entries_without_tags() {
    let s = setup().await;
    let response = s
        .app
        .post_json(
            &s.member_token,
            &format!("/api/networks/{}/entries", s.network),
            &json!({
                "entity_type": "character",
                "entity_id": 12345,
                "entity_name": "Noted Guy",
                "notes": "Seen in Jita",
            }),
        )
        .await;
    response.assert_status(201);
    let body = response.json();
    assert_eq!(body["notes"], "Seen in Jita");
    assert_eq!(body["label"], Value::Null);
    assert_eq!(body["color"], Value::Null);
}

#[tokio::test]
async fn denies_viewer_from_creating_entries() {
    let s = setup().await;
    let response = s
        .app
        .post_json(
            &s.viewer_token,
            &format!("/api/networks/{}/entries", s.network),
            &json!({
                "entity_type": "character",
                "entity_id": 12345,
                "entity_name": "Test",
                "label": "HOSTILE",
            }),
        )
        .await;
    response.assert_status(403);
    assert_json_eq(
        &json!({ "message": "This action is unauthorized." }),
        &response.json(),
    );
    assert_eq!(s.app.count("intel_entries").await, 0);
}

#[tokio::test]
async fn upserts_entry_for_same_entity() {
    let s = setup().await;
    let path = format!("/api/networks/{}/entries", s.network);
    s.app
        .post_json(
            &s.member_token,
            &path,
            &json!({
                "entity_type": "character",
                "entity_id": 12345,
                "entity_name": "Target",
                "label": "HOSTILE",
            }),
        )
        .await
        .assert_status(201);
    s.app.clock.advance(time::Duration::minutes(5));
    let second = s
        .app
        .post_json(
            &s.member_token,
            &path,
            &json!({
                "entity_type": "character",
                "entity_id": 12345,
                "entity_name": "Target",
                "label": "HOSTILE | SPY",
            }),
        )
        .await;
    // Still 201 for the update; the stored row answers in column order.
    second.assert_status(201);
    assert_json_eq(
        &stored_entry(
            1,
            s.network,
            json!({
                "entity_type": "character",
                "entity_id": 12345,
                "entity_name": "Target",
                "color": null,
                "label": "HOSTILE | SPY",
                "notes": null,
            }),
            &s.member,
            "2026-10-13T08:35:00.000000Z",
        ),
        &second.json(),
    );
    assert_eq!(s.app.count("intel_entries").await, 1);
}

#[tokio::test]
async fn allows_member_to_update_an_entry() {
    let s = setup().await;
    let entry = s.app.entry(s.network, 12345, "HOSTILE", s.owner.id).await;
    s.app.clock.advance(time::Duration::minutes(1));
    let response = s
        .app
        .put_json(
            &s.member_token,
            &format!("/api/networks/{}/entries/{entry}", s.network),
            &json!({ "label": "FRIENDLY", "notes": "Actually a friend" }),
        )
        .await;
    response.assert_status(200);
    assert_json_eq(
        &stored_entry(
            entry,
            s.network,
            json!({
                "entity_type": "character",
                "entity_id": 12345,
                "entity_name": "Target",
                "color": null,
                "label": "FRIENDLY",
                "notes": "Actually a friend",
            }),
            &s.owner,
            "2026-10-13T08:31:00.000000Z",
        ),
        &response.json(),
    );
}

#[tokio::test]
async fn allows_member_to_delete_an_entry() {
    let s = setup().await;
    let entry = s.app.entry(s.network, 12345, "HOSTILE", s.owner.id).await;
    let response = s
        .app
        .delete_json(
            &s.member_token,
            &format!("/api/networks/{}/entries/{entry}", s.network),
        )
        .await;
    response.assert_status(200);
    assert_json_eq(&json!({ "message": "Entry deleted" }), &response.json());
    assert_eq!(s.app.row("intel_entries", entry).await, None);
}

#[tokio::test]
async fn denies_viewer_from_deleting_entries() {
    let s = setup().await;
    let entry = s.app.entry(s.network, 12345, "HOSTILE", s.owner.id).await;
    let response = s
        .app
        .delete_json(
            &s.viewer_token,
            &format!("/api/networks/{}/entries/{entry}", s.network),
        )
        .await;
    response.assert_status(403);
    assert!(s.app.row("intel_entries", entry).await.is_some());
}

#[tokio::test]
async fn allows_viewer_to_list_entries() {
    let s = setup().await;
    let entry = s.app.entry(s.network, 12345, "HOSTILE", s.owner.id).await;
    let response = s
        .app
        .get_json(
            &s.viewer_token,
            &format!("/api/networks/{}/entries", s.network),
        )
        .await;
    response.assert_status(200);
    assert_json_eq(
        &json!([stored_entry(
            entry,
            s.network,
            json!({
                "entity_type": "character",
                "entity_id": 12345,
                "entity_name": "Target",
                "color": null,
                "label": "HOSTILE",
                "notes": null,
            }),
            &s.owner,
            NOW_Z,
        )]),
        &response.json(),
    );
}

#[tokio::test]
async fn returns_entries_matching_entity_ids_across_accessible_networks() {
    let s = setup().await;
    let entry = s.app.entry(s.network, 12345, "HOSTILE", s.owner.id).await;
    let response = s
        .app
        .get_json(&s.viewer_token, "/api/intel/lookup?entity_ids[]=12345")
        .await;
    response.assert_status(200);
    assert_json_eq(
        &json!([{
            "id": entry,
            "intel_network_id": s.network,
            "network_name": "Test Network",
            "entity_type": "character",
            "entity_id": 12345,
            "entity_name": "Target",
            "color": null,
            "label": "HOSTILE",
            "notes": null,
        }]),
        &response.json(),
    );
}

#[tokio::test]
async fn excludes_entries_from_inaccessible_networks() {
    let s = setup().await;
    let other = s.app.user().await;
    let other_network = s.app.create_network("Other", other.id).await;
    s.app.grant_owner(other_network, &other).await;
    s.app.entry(other_network, 99999, "SPY", other.id).await;
    let response = s
        .app
        .get_json(&s.viewer_token, "/api/intel/lookup?entity_ids[]=99999")
        .await;
    response.assert_status(200);
    assert_json_eq(&json!([]), &response.json());
}

// ---------------------------------------------------------------------------
// Beyond the PHP suite.
// ---------------------------------------------------------------------------

/// FIXES.md (security): the entry id must belong to the network in the
/// URL. Laravel updated another network's entry.
#[tokio::test]
async fn updating_an_entry_of_another_network_is_not_found() {
    let s = setup().await;
    let stranger = s.app.user().await;
    let foreign_network = s.app.create_network("Foreign", stranger.id).await;
    s.app.grant_owner(foreign_network, &stranger).await;
    let foreign = s
        .app
        .entry(foreign_network, 777, "SECRET", stranger.id)
        .await;
    let response = s
        .app
        .put_json(
            &s.member_token,
            &format!("/api/networks/{}/entries/{foreign}", s.network),
            &json!({ "label": "PWNED" }),
        )
        .await;
    response.assert_status(404);
    assert_json_eq(
        &json!({ "message": format!("No query results for model [App\\Models\\IntelEntry] {foreign}") }),
        &response.json(),
    );
    assert_eq!(
        s.app.row("intel_entries", foreign).await.expect("kept")["label"],
        "SECRET"
    );
}

/// FIXES.md (security).
#[tokio::test]
async fn deleting_an_entry_of_another_network_is_not_found() {
    let s = setup().await;
    let stranger = s.app.user().await;
    let foreign_network = s.app.create_network("Foreign", stranger.id).await;
    s.app.grant_owner(foreign_network, &stranger).await;
    let foreign = s
        .app
        .entry(foreign_network, 777, "SECRET", stranger.id)
        .await;
    let response = s
        .app
        .delete_json(
            &s.member_token,
            &format!("/api/networks/{}/entries/{foreign}", s.network),
        )
        .await;
    response.assert_status(404);
    assert!(s.app.row("intel_entries", foreign).await.is_some());
}

/// FIXES.md: an update may clear the label and the color, as the v0.5
/// client sends them (empty strings for no color).
#[tokio::test]
async fn updating_can_clear_the_label_and_color() {
    let s = setup().await;
    let entry = s
        .app
        .create_entry(
            s.network,
            NewEntry {
                entity_type: "character",
                entity_id: 12345,
                entity_name: "Target".to_owned(),
                color: Some("#FF3B3B".to_owned()),
                label: Some("HOSTILE".to_owned()),
                notes: Some("Note".to_owned()),
                added_by: s.owner.id,
            },
        )
        .await;
    let response = s
        .app
        .put_json(
            &s.member_token,
            &format!("/api/networks/{}/entries/{entry}", s.network),
            &json!({
                "entity_type": "character",
                "entity_id": 12345,
                "entity_name": "Target",
                "color": "",
                "label": null,
                "notes": "Note",
            }),
        )
        .await;
    response.assert_status(200);
    let body = response.json();
    assert_eq!(body["color"], Value::Null);
    assert_eq!(body["label"], Value::Null);
    assert_eq!(body["notes"], "Note");
    let row = s.app.row("intel_entries", entry).await.expect("the entry");
    assert_eq!(row["color"], Value::Null);
    assert_eq!(row["label"], Value::Null);
}

/// The other rules of an update stay.
#[tokio::test]
async fn updating_still_checks_the_color_and_name() {
    let s = setup().await;
    let entry = s.app.entry(s.network, 12345, "HOSTILE", s.owner.id).await;
    let response = s
        .app
        .put_json(
            &s.member_token,
            &format!("/api/networks/{}/entries/{entry}", s.network),
            &json!({ "color": "#GGGGGG", "entity_name": null }),
        )
        .await;
    response.assert_status(422);
    assert_json_eq(
        &json!({
            "message": "The color field format is invalid. (and 1 more error)",
            "errors": {
                "color": ["The color field format is invalid."],
                "entity_name": ["The entity name field must be a string."],
            },
        }),
        &response.json(),
    );
}

/// FIXES.md (deliberate): the answer carries the stored integer.
#[tokio::test]
async fn an_entity_id_sent_as_text_is_answered_as_a_number() {
    let s = setup().await;
    let response = s
        .app
        .post_json(
            &s.member_token,
            &format!("/api/networks/{}/entries", s.network),
            &json!({ "entity_type": "character", "entity_id": "12345", "entity_name": "Text" }),
        )
        .await;
    response.assert_status(201);
    assert_eq!(response.json()["entity_id"], 12345);
}

/// The v0.5 client tries PATCH first and falls back to PUT on 405.
#[tokio::test]
async fn patch_is_not_allowed() {
    let s = setup().await;
    let entry = s.app.entry(s.network, 12345, "HOSTILE", s.owner.id).await;
    let response = s
        .app
        .patch_json(
            &s.member_token,
            &format!("/api/networks/{}/entries/{entry}", s.network),
            &json!({ "label": "X" }),
        )
        .await;
    response.assert_status(405);
    assert_eq!(response.header("allow"), Some("PUT, DELETE"));
    assert_json_eq(
        &json!({
            "message": format!(
                "The PATCH method is not supported for route api/networks/{}/entries/{entry}. Supported methods: PUT, DELETE.",
                s.network
            ),
        }),
        &response.json(),
    );
}

/// The lookup lists one row per network an entity is tagged in, in id
/// order, with the network's name.
#[tokio::test]
async fn the_lookup_lists_every_accessible_network() {
    let s = setup().await;
    let second = s.app.create_network("Second Network", s.owner.id).await;
    s.app.grant_user(second, &s.viewer, "viewer").await;
    let first_entry = s.app.entry(s.network, 12345, "HOSTILE", s.owner.id).await;
    let second_entry = s.app.entry(second, 12345, "SPY", s.owner.id).await;
    s.app.entry(second, 777, "OTHER", s.owner.id).await;
    let response = s
        .app
        .get_json(
            &s.viewer_token,
            "/api/intel/lookup?entity_ids[]=12345&entity_ids[]=555",
        )
        .await;
    response.assert_status(200);
    let body = response.json();
    let rows: Vec<(i64, String, String)> = body
        .as_array()
        .expect("a list")
        .iter()
        .map(|row| {
            (
                row["id"].as_i64().expect("an id"),
                row["network_name"].as_str().expect("a name").to_owned(),
                row["label"].as_str().expect("a label").to_owned(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            (first_entry, "Test Network".to_owned(), "HOSTILE".to_owned()),
            (second_entry, "Second Network".to_owned(), "SPY".to_owned()),
        ]
    );
}

#[tokio::test]
async fn the_lookup_requires_a_list_of_integers() {
    let s = setup().await;
    let missing = s.app.get_json(&s.viewer_token, "/api/intel/lookup").await;
    missing.assert_status(422);
    assert_json_eq(
        &json!({
            "message": "The entity ids field is required.",
            "errors": { "entity_ids": ["The entity ids field is required."] },
        }),
        &missing.json(),
    );
    let invalid = s
        .app
        .get_json(
            &s.viewer_token,
            "/api/intel/lookup?entity_ids[]=abc&entity_ids[]=5",
        )
        .await;
    invalid.assert_status(422);
    assert_json_eq(
        &json!({
            "message": "The entity_ids.0 field must be an integer.",
            "errors": { "entity_ids.0": ["The entity_ids.0 field must be an integer."] },
        }),
        &invalid.json(),
    );
}
