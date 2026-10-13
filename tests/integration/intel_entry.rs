//! Creating, updating, deleting and listing a network's entries, and the
//! bulk lookup across networks: entry ids scoped to the network in the
//! URL, clearing a label, the v0.5 client's update and PATCH staying 405.

use axum::body::Body;
use axum::http::Method;
use serde_json::{Value, json};

use crate::common::factories::{ALLIANCE, CORPORATION, NewEntry, NewUser, User};
use crate::common::{HOST, NOW_Z, TestApp, assert_json_eq, header_map};

struct Setup {
    app: TestApp,
    owner: User,
    member: User,
    viewer: User,
    network: i64,
    member_token: String,
    viewer_token: String,
}

/// An owner, a member and a viewer of one network.
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
    assert_json_eq(
        &json!({ "message": "This action is unauthorized." }),
        &response.json(),
    );
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

/// The entry id must belong to the network in the URL.
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

/// An update may clear the label and the color, as the v0.5 client
/// sends them (empty strings for no color).
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

/// The answer carries the stored integer.
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
    let page = s
        .app
        .request(
            Method::PATCH,
            &format!("/api/networks/{}/entries/{entry}", s.network),
            header_map(&[
                ("host", HOST),
                ("authorization", &format!("Bearer {}", s.member_token)),
                ("content-type", "application/json"),
            ]),
            Body::from(r#"{"label":"X"}"#),
        )
        .await;
    page.assert_status(405);
    assert_eq!(page.header("allow"), Some("PUT, DELETE"));
    assert!(page.text().contains("<title>Method Not Allowed</title>"));
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

/// An entry id that does not exist is not found, for an update and a
/// delete alike.
#[tokio::test]
async fn a_missing_entry_is_not_found() {
    let s = setup().await;
    let path = format!("/api/networks/{}/entries/999", s.network);
    let expected = json!({ "message": "No query results for model [App\\Models\\IntelEntry] 999" });
    let update = s
        .app
        .put_json(&s.member_token, &path, &json!({ "label": "X" }))
        .await;
    update.assert_status(404);
    assert_json_eq(&expected, &update.json());
    let delete = s.app.delete_json(&s.member_token, &path).await;
    delete.assert_status(404);
    assert_json_eq(&expected, &delete.json());
}

/// A network's entries are listed by entity type, then entity id.
#[tokio::test]
async fn entries_are_listed_by_type_then_entity_id() {
    let s = setup().await;
    let entry = |entity_type: &'static str, entity_id: i64, added_by: i64| NewEntry {
        entity_type,
        entity_id,
        entity_name: format!("Entity {entity_id}"),
        added_by,
        ..NewEntry::default()
    };
    let hank = s
        .app
        .create_entry(s.network, entry("character", 90_001_001, s.owner.id))
        .await;
    let corp = s
        .app
        .create_entry(s.network, entry("corporation", 98_000_777, s.member.id))
        .await;
    let alliance = s
        .app
        .create_entry(s.network, entry("alliance", 99_000_777, s.owner.id))
        .await;
    let early = s
        .app
        .create_entry(s.network, entry("character", 90_000_500, s.member.id))
        .await;
    let response = s
        .app
        .get_json(
            &s.viewer_token,
            &format!("/api/networks/{}/entries", s.network),
        )
        .await;
    response.assert_status(200);
    let rows: Vec<(i64, i64)> = response
        .json()
        .as_array()
        .expect("a list")
        .iter()
        .map(|row| {
            (
                row["id"].as_i64().expect("an id"),
                row["added_by"]["id"].as_i64().expect("a user"),
            )
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            (alliance, s.owner.id),
            (early, s.member.id),
            (hank, s.owner.id),
            (corp, s.member.id),
        ]
    );
}

#[tokio::test]
async fn creating_an_entry_validates_every_field() {
    let s = setup().await;
    let path = format!("/api/networks/{}/entries", s.network);
    let missing = s.app.post_json(&s.member_token, &path, &json!({})).await;
    missing.assert_status(422);
    assert_json_eq(
        &json!({
            "message": "The entity type field is required. (and 2 more errors)",
            "errors": {
                "entity_type": ["The entity type field is required."],
                "entity_id": ["The entity id field is required."],
                "entity_name": ["The entity name field is required."],
            },
        }),
        &missing.json(),
    );
    let invalid = s
        .app
        .post_json(
            &s.member_token,
            &path,
            &json!({
                "entity_type": "ship",
                "entity_id": 1.5,
                "color": "red",
                "label": "x".repeat(256),
                "notes": "y".repeat(10_001),
            }),
        )
        .await;
    invalid.assert_status(422);
    assert_json_eq(
        &json!({
            "message": "The selected entity type is invalid. (and 5 more errors)",
            "errors": {
                "entity_type": ["The selected entity type is invalid."],
                "entity_id": ["The entity id field must be an integer."],
                "entity_name": ["The entity name field is required."],
                "color": ["The color field format is invalid."],
                "label": ["The label field must not be greater than 255 characters."],
                "notes": ["The notes field must not be greater than 10000 characters."],
            },
        }),
        &invalid.json(),
    );
    assert_eq!(s.app.count("intel_entries").await, 0);
}

/// Empty strings and blank notes are stored as null.
#[tokio::test]
async fn empty_strings_are_stored_as_null() {
    let s = setup().await;
    let response = s
        .app
        .post_json(
            &s.member_token,
            &format!("/api/networks/{}/entries", s.network),
            &json!({
                "entity_type": "alliance",
                "entity_id": 99_000_555,
                "entity_name": "Some Alliance",
                "color": "",
                "label": "",
                "notes": "  ",
            }),
        )
        .await;
    response.assert_status(201);
    assert_json_eq(
        &json!({
            "entity_type": "alliance",
            "entity_id": 99_000_555,
            "entity_name": "Some Alliance",
            "color": null,
            "label": null,
            "notes": null,
            "added_by": s.member.json(),
            "intel_network_id": s.network,
            "updated_at": NOW_Z,
            "created_at": NOW_Z,
            "id": 1,
        }),
        &response.json(),
    );
    let row = s.app.row("intel_entries", 1).await.expect("the entry");
    assert_eq!(row["color"], Value::Null);
    assert_eq!(row["label"], Value::Null);
    assert_eq!(row["notes"], Value::Null);
}

/// The same entity id under another entity type is another entry.
#[tokio::test]
async fn the_same_id_under_another_type_is_a_new_entry() {
    let s = setup().await;
    let existing = s
        .app
        .entry(s.network, 90_001_001, "HOSTILE", s.owner.id)
        .await;
    let response = s
        .app
        .post_json(
            &s.member_token,
            &format!("/api/networks/{}/entries", s.network),
            &json!({ "entity_type": "corporation", "entity_id": 90_001_001, "entity_name": "Hank Corp" }),
        )
        .await;
    response.assert_status(201);
    assert_eq!(response.json()["id"], existing + 1);
    assert_eq!(response.json()["entity_type"], "corporation");
    assert_eq!(s.app.count("intel_entries").await, 2);
}

/// Names and notes keep their characters, slashes unescaped.
#[tokio::test]
async fn unicode_names_and_notes_are_kept() {
    let s = setup().await;
    let response = s
        .app
        .post_json(
            &s.member_token,
            &format!("/api/networks/{}/entries", s.network),
            &json!({
                "entity_type": "character",
                "entity_id": 90_001_013,
                "entity_name": "José Ørsted",
                "notes": "Ünïcode ✓ / slash",
            }),
        )
        .await;
    response.assert_status(201);
    let text = response.text();
    assert!(text.contains(r#""entity_name":"José Ørsted""#), "{text}");
    assert!(text.contains(r#""notes":"Ünïcode ✓ / slash""#), "{text}");
    let row = s.app.row("intel_entries", 1).await.expect("the entry");
    assert_eq!(row["entity_name"], "José Ørsted");
    assert_eq!(row["notes"], "Ünïcode ✓ / slash");
}

/// Upserting another member's entry makes the sender its author and
/// keeps when it was created.
#[tokio::test]
async fn an_upsert_takes_over_the_entry() {
    let s = setup().await;
    let entry = s
        .app
        .create_entry(
            s.network,
            NewEntry {
                entity_type: "character",
                entity_id: 90_001_001,
                entity_name: "Hostile Hank".to_owned(),
                color: Some("#FF3B3B".to_owned()),
                label: Some("HOSTILE".to_owned()),
                notes: Some("Old note".to_owned()),
                added_by: s.owner.id,
            },
        )
        .await;
    s.app.clock.advance(time::Duration::hours(1));
    let response = s
        .app
        .post_json(
            &s.member_token,
            &format!("/api/networks/{}/entries", s.network),
            &json!({
                "entity_type": "character",
                "entity_id": 90_001_001,
                "entity_name": "Hostile Hank",
                "color": "#00FF00",
                "label": "HOSTILE | SPY",
            }),
        )
        .await;
    response.assert_status(201);
    assert_json_eq(
        &stored_entry(
            entry,
            s.network,
            json!({
                "entity_type": "character",
                "entity_id": 90_001_001,
                "entity_name": "Hostile Hank",
                "color": "#00FF00",
                "label": "HOSTILE | SPY",
                "notes": null,
            }),
            &s.member,
            "2026-10-13T09:30:00.000000Z",
        ),
        &response.json(),
    );
}

/// A manager may rename and recolor; notes may be cleared with null; an
/// update that changes nothing leaves `updated_at` alone.
#[tokio::test]
async fn updates_change_only_what_they_name() {
    let s = setup().await;
    let owner_token = s.app.token(&s.owner).await;
    let entry = s
        .app
        .create_entry(
            s.network,
            NewEntry {
                entity_type: "corporation",
                entity_id: 98_000_777,
                entity_name: "Spy Corp".to_owned(),
                label: Some("SPY".to_owned()),
                notes: Some("Watch their alts".to_owned()),
                added_by: s.member.id,
                ..NewEntry::default()
            },
        )
        .await;
    let path = format!("/api/networks/{}/entries/{entry}", s.network);
    let fields = |name: &str, color: Value, notes: Value| {
        json!({
            "entity_type": "corporation",
            "entity_id": 98_000_777,
            "entity_name": name,
            "color": color,
            "label": "SPY",
            "notes": notes,
        })
    };

    s.app.clock.advance(time::Duration::minutes(1));
    let same = s
        .app
        .put_json(&s.member_token, &path, &json!({ "label": "SPY" }))
        .await;
    same.assert_status(200);
    assert_json_eq(
        &stored_entry(
            entry,
            s.network,
            fields("Spy Corp", Value::Null, json!("Watch their alts")),
            &s.member,
            NOW_Z,
        ),
        &same.json(),
    );

    let renamed = s
        .app
        .put_json(
            &owner_token,
            &path,
            &json!({ "entity_name": "Spy Corp [SPY]", "color": "#123ABC" }),
        )
        .await;
    renamed.assert_status(200);
    assert_json_eq(
        &stored_entry(
            entry,
            s.network,
            fields(
                "Spy Corp [SPY]",
                json!("#123ABC"),
                json!("Watch their alts"),
            ),
            &s.member,
            "2026-10-13T08:31:00.000000Z",
        ),
        &renamed.json(),
    );

    let cleared = s
        .app
        .put_json(&s.member_token, &path, &json!({ "notes": null }))
        .await;
    cleared.assert_status(200);
    assert_json_eq(
        &stored_entry(
            entry,
            s.network,
            fields("Spy Corp [SPY]", json!("#123ABC"), Value::Null),
            &s.member,
            "2026-10-13T08:31:00.000000Z",
        ),
        &cleared.json(),
    );
}

/// The lookup reaches networks shared with the user's alliance or
/// corporation, skips expired access and takes indexed ids
/// (`entity_ids[0]=...`).
#[tokio::test]
async fn the_lookup_follows_the_users_access() {
    let s = setup().await;
    let stranger = s.app.user().await;
    let shared = s.app.create_network("Bravo Watch", stranger.id).await;
    s.app.grant_owner(shared, &stranger).await;
    let corp_network = s.app.create_network("Charlie Ops", stranger.id).await;
    let lapsed_network = s.app.create_network("Delta", stranger.id).await;
    s.app
        .grant(shared, ALLIANCE, 99_000_001, "member", false, None)
        .await;
    s.app
        .grant(corp_network, CORPORATION, 98_000_001, "viewer", false, None)
        .await;
    s.app.grant_user(lapsed_network, &s.owner, "viewer").await;
    sqlx::query("update network_access set expires_at = $1 where intel_network_id = $2")
        .bind(s.app.now() - time::Duration::days(3))
        .bind(lapsed_network)
        .execute(&s.app.pool)
        .await
        .expect("access expires");
    let owner_token = s.app.token(&s.owner).await;
    let own = s
        .app
        .entry(s.network, 90_001_001, "HOSTILE", s.owner.id)
        .await;
    let via_alliance = s
        .app
        .entry(shared, 90_001_001, "FRIENDLY", stranger.id)
        .await;
    let via_corp = s
        .app
        .entry(corp_network, 90_001_003, "SPY", stranger.id)
        .await;
    s.app
        .entry(lapsed_network, 90_001_001, "LAPSED", stranger.id)
        .await;
    let response = s
        .app
        .get_json(
            &owner_token,
            "/api/intel/lookup?entity_ids%5B0%5D=90001001&entity_ids%5B1%5D=90001003",
        )
        .await;
    response.assert_status(200);
    let target = |id: i64, network: i64, name: &str, entity_id: i64, label: &str| {
        json!({
            "id": id,
            "intel_network_id": network,
            "network_name": name,
            "entity_type": "character",
            "entity_id": entity_id,
            "entity_name": "Target",
            "color": null,
            "label": label,
            "notes": null,
        })
    };
    assert_json_eq(
        &json!([
            target(own, s.network, "Test Network", 90_001_001, "HOSTILE"),
            target(via_alliance, shared, "Bravo Watch", 90_001_001, "FRIENDLY"),
            target(via_corp, corp_network, "Charlie Ops", 90_001_003, "SPY"),
        ]),
        &response.json(),
    );

    // Only lapsed access: nothing.
    let lapsed = s.app.user().await;
    s.app.grant_user(lapsed_network, &lapsed, "member").await;
    sqlx::query("update network_access set expires_at = $1 where intel_network_id = $2")
        .bind(s.app.now() - time::Duration::days(3))
        .bind(lapsed_network)
        .execute(&s.app.pool)
        .await
        .expect("access expires");
    let lapsed_token = s.app.token(&lapsed).await;
    let none = s
        .app
        .get_json(&lapsed_token, "/api/intel/lookup?entity_ids[]=90001001")
        .await;
    none.assert_status(200);
    assert_json_eq(&json!([]), &none.json());
}

#[tokio::test]
async fn the_lookup_names_every_bad_id() {
    let s = setup().await;
    let invalid = s
        .app
        .get_json(
            &s.viewer_token,
            "/api/intel/lookup?entity_ids[]=abc&entity_ids[]=&entity_ids[]=5",
        )
        .await;
    invalid.assert_status(422);
    assert_json_eq(
        &json!({
            "message": "The entity_ids.0 field must be an integer. (and 1 more error)",
            "errors": {
                "entity_ids.0": ["The entity_ids.0 field must be an integer."],
                "entity_ids.1": ["The entity_ids.1 field must be an integer."],
            },
        }),
        &invalid.json(),
    );
    let single = s
        .app
        .get_json(&s.viewer_token, "/api/intel/lookup?entity_ids=90001001")
        .await;
    single.assert_status(422);
    assert_json_eq(
        &json!({
            "message": "The entity ids field must be an array.",
            "errors": { "entity_ids": ["The entity ids field must be an array."] },
        }),
        &single.json(),
    );
}
