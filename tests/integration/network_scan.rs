//! tests/Feature/NetworkScanTest.php: sharing local and d-scans and the
//! paged scan history, newest first.

use serde_json::{Value, json};
use time::Duration;

use crate::common::factories::{NewUser, User};
use crate::common::{APP_URL, NOW_ISO, TestApp, assert_json_eq};

struct Setup {
    app: TestApp,
    owner: User,
    network: i64,
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
    app.grant_owner(network, &owner).await;
    let token = app.token(&owner).await;
    Setup {
        app,
        owner,
        network,
        token,
    }
}

async fn viewer(s: &Setup) -> String {
    let viewer = s.app.user().await;
    s.app.grant_user(s.network, &viewer, "viewer").await;
    s.app.token(&viewer).await
}

#[tokio::test]
async fn allows_member_to_share_a_local_scan() {
    let s = setup().await;
    let response = s
        .app
        .post_json(
            &s.token,
            &format!("/api/networks/{}/scans", s.network),
            &json!({ "scan_type": "local", "raw_text": "Pilot One\nPilot Two\nPilot Three" }),
        )
        .await;
    response.assert_status(201);
    assert_json_eq(
        &json!({
            "id": 1,
            "scan_type": "local",
            "raw_text": "Pilot One\nPilot Two\nPilot Three",
            "solar_system": null,
            "created_at": NOW_ISO,
            "submitted_by": {
                "id": s.owner.character_id,
                "character_name": s.owner.character_name,
            },
        }),
        &response.json(),
    );
}

#[tokio::test]
async fn allows_member_to_share_a_dscan() {
    let s = setup().await;
    let response = s
        .app
        .post_json(
            &s.token,
            &format!("/api/networks/{}/scans", s.network),
            &json!({
                "scan_type": "dscan",
                "raw_text": "12345\tShip Name\tRifter\t100 km",
                "solar_system": "Jita",
            }),
        )
        .await;
    response.assert_status(201);
    let body = response.json();
    assert_eq!(body["scan_type"], "dscan");
    assert_eq!(body["raw_text"], "12345\tShip Name\tRifter\t100 km");
    assert_eq!(body["solar_system"], "Jita");
}

#[tokio::test]
async fn denies_viewer_from_sharing_scans() {
    let s = setup().await;
    let token = viewer(&s).await;
    let response = s
        .app
        .post_json(
            &token,
            &format!("/api/networks/{}/scans", s.network),
            &json!({ "scan_type": "local", "raw_text": "Pilot One" }),
        )
        .await;
    response.assert_status(403);
    assert_eq!(s.app.count("network_scans").await, 0);
}

#[tokio::test]
async fn rejects_invalid_scan_type() {
    let s = setup().await;
    let response = s
        .app
        .post_json(
            &s.token,
            &format!("/api/networks/{}/scans", s.network),
            &json!({ "scan_type": "invalid", "raw_text": "test" }),
        )
        .await;
    response.assert_status(422);
    assert_json_eq(
        &json!({
            "message": "The selected scan type is invalid.",
            "errors": { "scan_type": ["The selected scan type is invalid."] },
        }),
        &response.json(),
    );
}

#[tokio::test]
async fn rejects_missing_raw_text() {
    let s = setup().await;
    let response = s
        .app
        .post_json(
            &s.token,
            &format!("/api/networks/{}/scans", s.network),
            &json!({ "scan_type": "local" }),
        )
        .await;
    response.assert_status(422);
    assert_json_eq(
        &json!({
            "message": "The raw text field is required.",
            "errors": { "raw_text": ["The raw text field is required."] },
        }),
        &response.json(),
    );
}

#[tokio::test]
async fn allows_viewer_to_list_scan_history() {
    let s = setup().await;
    let token = viewer(&s).await;
    s.app
        .create_scan(s.network, s.owner.id, "local", "Pilot A")
        .await;
    s.app
        .create_scan(s.network, s.owner.id, "dscan", "12345\tName\tType\t100km")
        .await;
    let response = s
        .app
        .get_json(&token, &format!("/api/networks/{}/scans", s.network))
        .await;
    response.assert_status(200);
    let path = format!("{APP_URL}/api/networks/{}/scans", s.network);
    let submitter = json!({ "id": s.owner.character_id, "character_name": s.owner.character_name });
    assert_json_eq(
        &json!({
            "current_page": 1,
            "data": [
                {
                    "id": 2,
                    "scan_type": "dscan",
                    "raw_text": "12345\tName\tType\t100km",
                    "solar_system": null,
                    "created_at": NOW_ISO,
                    "submitted_by": submitter,
                },
                {
                    "id": 1,
                    "scan_type": "local",
                    "raw_text": "Pilot A",
                    "solar_system": null,
                    "created_at": NOW_ISO,
                    "submitted_by": submitter,
                },
            ],
            "first_page_url": format!("{path}?page=1"),
            "from": 1,
            "last_page": 1,
            "last_page_url": format!("{path}?page=1"),
            "links": [
                { "url": null, "label": "&laquo; Previous", "page": null, "active": false },
                { "url": format!("{path}?page=1"), "label": "1", "page": 1, "active": true },
                { "url": null, "label": "Next &raquo;", "page": null, "active": false },
            ],
            "next_page_url": null,
            "path": path,
            "per_page": 20,
            "prev_page_url": null,
            "to": 2,
            "total": 2,
        }),
        &response.json(),
    );
}

#[tokio::test]
async fn returns_scans_in_newest_first_order() {
    let s = setup().await;
    s.app
        .create_scan(s.network, s.owner.id, "local", "First scan")
        .await;
    s.app
        .create_scan(s.network, s.owner.id, "local", "Second scan")
        .await;
    let response = s
        .app
        .get_json(&s.token, &format!("/api/networks/{}/scans", s.network))
        .await;
    response.assert_status(200);
    let texts: Vec<Value> = response.json()["data"]
        .as_array()
        .expect("data")
        .iter()
        .map(|scan| scan["raw_text"].clone())
        .collect();
    assert_eq!(texts, vec![json!("Second scan"), json!("First scan")]);
}

#[tokio::test]
async fn paginates_scan_history() {
    let s = setup().await;
    for i in 0..25 {
        s.app
            .create_scan_at(
                s.network,
                s.owner.id,
                "local",
                &format!("Scan {i}"),
                s.app.now() + Duration::seconds(i),
            )
            .await;
    }
    let first = s
        .app
        .get_json(&s.token, &format!("/api/networks/{}/scans", s.network))
        .await;
    first.assert_status(200);
    let body = first.json();
    assert_eq!(body["data"].as_array().map(Vec::len), Some(20));
    assert_eq!(body["data"][0]["raw_text"], "Scan 24");
    assert_eq!(body["last_page"], 2);
    let path = format!("{APP_URL}/api/networks/{}/scans", s.network);
    assert_eq!(body["next_page_url"], format!("{path}?page=2"));
    assert_json_eq(
        &json!([
            { "url": null, "label": "&laquo; Previous", "page": null, "active": false },
            { "url": format!("{path}?page=1"), "label": "1", "page": 1, "active": true },
            { "url": format!("{path}?page=2"), "label": "2", "page": 2, "active": false },
            { "url": format!("{path}?page=2"), "label": "Next &raquo;", "page": 2, "active": false },
        ]),
        &body["links"],
    );
    let second = s
        .app
        .get_json(
            &s.token,
            &format!("/api/networks/{}/scans?page=2", s.network),
        )
        .await;
    second.assert_status(200);
    let body = second.json();
    assert_eq!(body["data"].as_array().map(Vec::len), Some(5));
    assert_eq!(body["data"][4]["raw_text"], "Scan 0");
    assert_eq!(body["from"], 21);
    assert_eq!(body["to"], 25);
    assert_eq!(body["prev_page_url"], format!("{path}?page=1"));
}

// ---------------------------------------------------------------------------
// Beyond the PHP suite.
// ---------------------------------------------------------------------------

/// A page that is not a positive number is the first page; one past the
/// end is empty.
#[tokio::test]
async fn reads_the_page_like_laravel() {
    let s = setup().await;
    s.app
        .create_scan(s.network, s.owner.id, "local", "Only scan")
        .await;
    for page in ["abc", "0", "-1", "1.5"] {
        let response = s
            .app
            .get_json(
                &s.token,
                &format!("/api/networks/{}/scans?page={page}", s.network),
            )
            .await;
        response.assert_status(200);
        assert_eq!(response.json()["current_page"], 1, "{page}");
    }
    let past = s
        .app
        .get_json(
            &s.token,
            &format!("/api/networks/{}/scans?page=7", s.network),
        )
        .await;
    let body = past.json();
    assert_eq!(body["current_page"], 7);
    assert_json_eq(&json!([]), &body["data"]);
    assert_eq!(body["from"], Value::Null);
    assert_eq!(body["to"], Value::Null);
    assert_eq!(body["last_page"], 1);
}

#[tokio::test]
async fn shows_one_scan() {
    let s = setup().await;
    let scan = s
        .app
        .create_scan(s.network, s.owner.id, "local", "Pilot A")
        .await;
    let response = s
        .app
        .get_json(
            &s.token,
            &format!("/api/networks/{}/scans/{scan}", s.network),
        )
        .await;
    response.assert_status(200);
    assert_json_eq(
        &json!({
            "id": scan,
            "scan_type": "local",
            "raw_text": "Pilot A",
            "solar_system": null,
            "created_at": NOW_ISO,
            "submitted_by": { "id": s.owner.character_id, "character_name": s.owner.character_name },
        }),
        &response.json(),
    );
}

/// A scan of another network is not found, after the network is
/// authorized, with Laravel's empty message.
#[tokio::test]
async fn a_scan_of_another_network_is_not_found() {
    let s = setup().await;
    let other = s.app.create_network("Other", s.owner.id).await;
    let scan = s
        .app
        .create_scan(other, s.owner.id, "local", "Elsewhere")
        .await;
    let response = s
        .app
        .get_json(
            &s.token,
            &format!("/api/networks/{}/scans/{scan}", s.network),
        )
        .await;
    response.assert_status(404);
    assert_json_eq(&json!({ "message": "" }), &response.json());
}

/// Every whitespace-only text is empty and so missing.
#[tokio::test]
async fn a_blank_text_is_missing() {
    let s = setup().await;
    let response = s
        .app
        .post_json(
            &s.token,
            &format!("/api/networks/{}/scans", s.network),
            &json!({ "scan_type": "local", "raw_text": " \n " }),
        )
        .await;
    response.assert_status(422);
    assert_eq!(
        response.json()["errors"]["raw_text"],
        json!(["The raw text field is required."])
    );
}
