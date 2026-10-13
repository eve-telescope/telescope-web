//! Listing, creating, showing, renaming and deleting networks.

use serde_json::json;

use crate::common::factories::{ALLIANCE, CORPORATION, NewEntry, NewUser, USER, User};
use crate::common::{NOW_Z, TestApp, assert_json_eq, keys};

struct Setup {
    app: TestApp,
    user: User,
    token: String,
}

/// A user in Test Corp and Test Alliance.
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
    // the raw access row (the stored class name as its type).
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

/// The id is a whole number: `1abc` and `abc` name no network.
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
    let letters = s.app.get_json(&s.token, "/api/networks/abc").await;
    letters.assert_status(404);
    assert_json_eq(
        &json!({ "message": "No query results for model [App\\Models\\IntelNetwork] abc" }),
        &letters.json(),
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
    let missing = s.app.post_json(&s.token, "/api/networks", &json!({})).await;
    missing.assert_status(422);
    assert_json_eq(
        &json!({
            "message": "The name field is required.",
            "errors": { "name": ["The name field is required."] },
        }),
        &missing.json(),
    );
    let number = s
        .app
        .post_json(&s.token, "/api/networks", &json!({ "name": 123 }))
        .await;
    number.assert_status(422);
    assert_json_eq(
        &json!({
            "message": "The name field must be a string.",
            "errors": { "name": ["The name field must be a string."] },
        }),
        &number.json(),
    );
    assert_eq!(s.app.count("intel_networks").await, 0);
}

/// The detail lists entries by entity type then entity id, each with the
/// user who added it, and accesses by type then entity id. A trailing
/// slash answers the same, and a pilot with access through their
/// alliance sees the same detail.
#[tokio::test]
async fn shows_the_whole_network_detail() {
    let s = setup().await;
    let network = owned(&s, "Alpha Intel").await;
    let member = s.app.user().await;
    s.app.grant_user(network, &member, "member").await;
    s.app
        .grant(network, CORPORATION, 98_000_001, "viewer", false, None)
        .await;
    let entry = |entity_type, entity_id, entity_name: &str, added_by| NewEntry {
        entity_type,
        entity_id,
        entity_name: entity_name.to_owned(),
        added_by,
        ..NewEntry::default()
    };
    let hank = s
        .app
        .create_entry(
            network,
            NewEntry {
                color: Some("#FF3B3B".to_owned()),
                label: Some("HOSTILE".to_owned()),
                ..entry("character", 90_001_001, "Hostile Hank", s.user.id)
            },
        )
        .await;
    let spy = s
        .app
        .create_entry(
            network,
            NewEntry {
                label: Some("SPY".to_owned()),
                notes: Some("Watch their alts".to_owned()),
                ..entry("corporation", 98_000_777, "Spy Corp", member.id)
            },
        )
        .await;
    let enemy = s
        .app
        .create_entry(
            network,
            NewEntry {
                color: Some("#ffaa22".to_owned()),
                label: Some("KOS".to_owned()),
                ..entry("alliance", 99_000_777, "Enemy Alliance", s.user.id)
            },
        )
        .await;
    let ned = s
        .app
        .create_entry(
            network,
            entry("character", 90_001_000, "Neutral Ned", member.id),
        )
        .await;
    let slug = s.app.row("intel_networks", network).await.expect("the row")["slug"].clone();
    let stored = |id, entity_type, entity_id, entity_name, color, label, notes, added_by: &User| {
        json!({
            "id": id,
            "intel_network_id": network,
            "entity_type": entity_type,
            "entity_id": entity_id,
            "entity_name": entity_name,
            "color": color,
            "label": label,
            "notes": notes,
            "added_by": added_by.json(),
            "created_at": NOW_Z,
            "updated_at": NOW_Z,
        })
    };
    let expected = json!({
        "id": network,
        "name": "Alpha Intel",
        "slug": slug,
        "owner_id": s.user.id,
        "created_at": NOW_Z,
        "updated_at": NOW_Z,
        "entries_count": 4,
        "entries": [
            stored(enemy, "alliance", 99_000_777, "Enemy Alliance", json!("#ffaa22"), json!("KOS"), json!(null), &s.user),
            stored(ned, "character", 90_001_000, "Neutral Ned", json!(null), json!(null), json!(null), &member),
            stored(hank, "character", 90_001_001, "Hostile Hank", json!("#FF3B3B"), json!("HOSTILE"), json!(null), &s.user),
            stored(spy, "corporation", 98_000_777, "Spy Corp", json!(null), json!("SPY"), json!("Watch their alts"), &member),
        ],
        "accesses": [
            {
                "id": 3,
                "accessible_type": "corporation",
                "accessible_id": 98_000_001,
                "permission": "viewer",
                "is_owner": false,
                "expires_at": null,
                "entity": {
                    "id": 98_000_001,
                    "name": "Test Corp",
                    "type": "corporation",
                    "ticker": "TSTC",
                    "corporation": null,
                    "alliance": { "id": 99_000_001, "name": "Test Alliance", "ticker": "TSTA" },
                },
            },
            {
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
            },
            {
                "id": 2,
                "accessible_type": "character",
                "accessible_id": member.character_id,
                "permission": "member",
                "is_owner": false,
                "expires_at": null,
                "entity": {
                    "id": member.character_id,
                    "name": member.character_name,
                    "type": "character",
                    "ticker": null,
                    "corporation": null,
                    "alliance": null,
                },
            },
        ],
    });
    for path in [
        format!("/api/networks/{network}"),
        format!("/api/networks/{network}/"),
    ] {
        let response = s.app.get_json(&s.token, &path).await;
        response.assert_status(200);
        assert_json_eq(&expected, &response.json());
    }
    s.app
        .create_alliance(99_000_002, "Test Alliance Please Ignore", "TEST")
        .await;
    let ally = s
        .app
        .create_user(NewUser {
            alliance_id: Some(99_000_002),
            ..NewUser::default()
        })
        .await;
    s.app
        .grant(network, ALLIANCE, 99_000_002, "viewer", false, None)
        .await;
    let ally_token = s.app.token(&ally).await;
    let response = s
        .app
        .get_json(&ally_token, &format!("/api/networks/{network}"))
        .await;
    response.assert_status(200);
    assert_eq!(response.json()["entries"], expected["entries"]);
    assert_eq!(
        response.json()["accesses"][0]["accessible_type"],
        "alliance"
    );
}

/// The index has every network the user reaches through their character,
/// corporation or alliance, by id, with its entry count; lapsed access
/// does not count, and a user without any gets an empty list.
#[tokio::test]
async fn lists_every_network_the_user_reaches() {
    let s = setup().await;
    let (owner, through_character) = foreign(&s, "Alpha Intel").await;
    s.app.grant_user(through_character, &s.user, "viewer").await;
    s.app.entry(through_character, 1, "A", owner.id).await;
    s.app.entry(through_character, 2, "B", owner.id).await;
    let (_, lapsed) = foreign(&s, "Lapsed").await;
    s.app
        .grant(
            lapsed,
            USER,
            s.user.character_id,
            "member",
            false,
            Some(s.app.now() - time::Duration::days(3)),
        )
        .await;
    let (_, through_corporation) = foreign(&s, "Bravo Watch").await;
    s.app
        .grant(
            through_corporation,
            CORPORATION,
            98_000_001,
            "member",
            false,
            None,
        )
        .await;
    s.app.clock.advance(time::Duration::hours(1));
    let (_, through_alliance) = foreign(&s, "Charlie Ops").await;
    s.app
        .grant(
            through_alliance,
            ALLIANCE,
            99_000_001,
            "viewer",
            false,
            None,
        )
        .await;
    foreign(&s, "Elsewhere").await;
    let slug = |row: Option<serde_json::Value>| row.expect("the network")["slug"].clone();
    let response = s.app.get_json(&s.token, "/api/networks").await;
    response.assert_status(200);
    assert_json_eq(
        &json!([
            {
                "id": through_character,
                "name": "Alpha Intel",
                "slug": slug(s.app.row("intel_networks", through_character).await),
                "owner_id": owner.id,
                "created_at": NOW_Z,
                "updated_at": NOW_Z,
                "entries_count": 2,
            },
            {
                "id": through_corporation,
                "name": "Bravo Watch",
                "slug": slug(s.app.row("intel_networks", through_corporation).await),
                "owner_id": s.app.row("intel_networks", through_corporation).await.expect("the network")["owner_id"],
                "created_at": NOW_Z,
                "updated_at": NOW_Z,
                "entries_count": 0,
            },
            {
                "id": through_alliance,
                "name": "Charlie Ops",
                "slug": slug(s.app.row("intel_networks", through_alliance).await),
                "owner_id": s.app.row("intel_networks", through_alliance).await.expect("the network")["owner_id"],
                "created_at": "2026-10-13T09:30:00.000000Z",
                "updated_at": "2026-10-13T09:30:00.000000Z",
                "entries_count": 0,
            },
        ]),
        &response.json(),
    );
    let stranger = s.app.user().await;
    let token = s.app.token(&stranger).await;
    let empty = s.app.get_json(&token, "/api/networks").await;
    empty.assert_status(200);
    assert_eq!(empty.text(), "[]");
}

/// A name without letters or digits gets a slug of a dash and its six
/// random characters.
#[tokio::test]
async fn a_name_of_symbols_gets_only_the_random_slug() {
    let s = setup().await;
    let response = s
        .app
        .post_json(&s.token, "/api/networks", &json!({ "name": "!!!" }))
        .await;
    response.assert_status(201);
    let body = response.json();
    assert_eq!(body["name"], "!!!");
    let slug = body["slug"].as_str().expect("a slug");
    assert_eq!(slug.len(), 7, "{slug}");
    assert!(slug.starts_with('-'), "{slug}");
    assert!(
        slug[1..].chars().all(|c| c.is_ascii_alphanumeric()),
        "{slug}"
    );
}

/// Renaming keeps the slug. A member is refused before the body is
/// validated, and a manager's empty name is refused.
#[tokio::test]
async fn renaming_keeps_the_slug_and_validates_the_name() {
    let s = setup().await;
    let network = owned(&s, "Alpha Intel").await;
    let slug = s.app.row("intel_networks", network).await.expect("the row")["slug"].clone();
    let path = format!("/api/networks/{network}");
    s.app.clock.advance(time::Duration::hours(1));
    let renamed = s
        .app
        .put_json(&s.token, &path, &json!({ "name": "Alpha Intel Renamed" }))
        .await;
    renamed.assert_status(200);
    assert_json_eq(
        &json!({
            "id": network,
            "name": "Alpha Intel Renamed",
            "slug": slug,
            "owner_id": s.user.id,
            "created_at": NOW_Z,
            "updated_at": "2026-10-13T09:30:00.000000Z",
        }),
        &renamed.json(),
    );
    let empty = s
        .app
        .put_json(&s.token, &path, &json!({ "name": "" }))
        .await;
    empty.assert_status(422);
    assert_json_eq(
        &json!({
            "message": "The name field is required.",
            "errors": { "name": ["The name field is required."] },
        }),
        &empty.json(),
    );
    let member = s.app.user().await;
    s.app.grant_user(network, &member, "member").await;
    let token = s.app.token(&member).await;
    let refused = s.app.put_json(&token, &path, &json!({})).await;
    refused.assert_status(403);
    assert_json_eq(
        &json!({ "message": "This action is unauthorized." }),
        &refused.json(),
    );
    assert_eq!(
        s.app.row("intel_networks", network).await.expect("the row")["name"],
        "Alpha Intel Renamed"
    );
}
