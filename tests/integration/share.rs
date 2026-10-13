//! Shared pilot lists, which the PHP suite never covered: `POST
//! /api/share` and `GET /api/share/{code}` (every desktop client since
//! v0.1.0) and the share page's props, which count a view.

use axum::body::Body;
use axum::http::Method;
use serde_json::{Value, json};

use crate::common::{APP_URL, HOST, NOW_Z, TestApp, assert_json_eq, header_map};

async fn create(app: &TestApp, body: &Value) -> crate::common::TestResponse {
    app.json(Method::POST, "/api/share", None, Some(body)).await
}

async fn share_row(app: &TestApp) -> Value {
    app.row("shares", 1).await.expect("the share")
}

#[tokio::test]
async fn creates_a_share_and_answers_its_link() {
    let app = TestApp::new().await;
    let response = create(
        &app,
        &json!({ "pilots": ["Pilot One", " Pilot Two ", "Pilot One", "0", "pilot one", "José Ørsted"] }),
    )
    .await;
    // 200, not 201.
    response.assert_status(200);
    let body = response.json();
    let code = body["code"].as_str().expect("a code").to_owned();
    assert_eq!(code.len(), 8);
    assert!(code.chars().all(|c| c.is_ascii_alphanumeric()), "{code}");
    assert_json_eq(
        &json!({ "code": code, "url": format!("{APP_URL}/s/{code}") }),
        &body,
    );
    // Trimmed, without empties and "0", first occurrences in order, case
    // sensitive.
    let row = share_row(&app).await;
    assert_eq!(
        row["pilots"],
        json!(["Pilot One", "Pilot Two", "pilot one", "José Ørsted"])
    );
    assert_eq!(row["pilot_count"], 4);
    assert_eq!(row["views"], 0);
}

#[tokio::test]
async fn fetches_a_share() {
    let app = TestApp::new().await;
    app.create_share("ABCDEFGH", &["Pilot One", "Pilot Two"], 3)
        .await;
    for path in ["/api/share/ABCDEFGH", "/api/share/abcdefgh"] {
        let response = app.json(Method::GET, path, None, None).await;
        response.assert_status(200);
        assert_json_eq(
            &json!({ "code": "ABCDEFGH", "pilots": ["Pilot One", "Pilot Two"], "pilotCount": 2 }),
            &response.json(),
        );
    }
    // Fetching counts no view.
    assert_eq!(share_row(&app).await["views"], 3);
}

#[tokio::test]
async fn an_unknown_share_is_not_found() {
    let app = TestApp::new().await;
    let response = app
        .json(Method::GET, "/api/share/NOPE0000", None, None)
        .await;
    response.assert_status(404);
    assert_json_eq(
        &json!({ "message": "No query results for model [App\\Models\\Share]." }),
        &response.json(),
    );
    let page = app.get("/api/share/NOPE0000").await;
    page.assert_status(404);
    assert!(page.text().contains("<title>Not Found</title>"));
}

#[tokio::test]
async fn the_share_page_counts_a_view() {
    let app = TestApp::new().await;
    app.create_share("ABCDEFGH", &["Pilot One", "Pilot Two", "Pilot Three"], 3)
        .await;
    app.clock.advance(time::Duration::hours(2));
    let response = app.get("/api/pages/s/abcdefgh").await;
    response.assert_status(200);
    assert_json_eq(
        &json!({
            "name": "Telescope",
            "auth": { "user": null },
            "appVersion": "v0.5.1",
            "share": {
                "code": "ABCDEFGH",
                "pilots": ["Pilot One", "Pilot Two", "Pilot Three"],
                "pilotCount": 3,
                "views": 4,
                "createdAt": NOW_Z,
            },
        }),
        &response.json(),
    );
    let row = share_row(&app).await;
    assert_eq!(row["views"], 4);
    assert_eq!(row["updated_at"], "2026-10-13T10:30:00+00:00");
    app.get("/api/pages/s/ABCDEFGH").await.assert_status(200);
    assert_eq!(share_row(&app).await["views"], 5);
}

#[tokio::test]
async fn the_share_page_of_an_unknown_code_is_not_found() {
    let app = TestApp::new().await;
    let response = app.get("/api/pages/s/NOPE0000").await;
    response.assert_status(404);
}

#[tokio::test]
async fn validates_the_pilots() {
    let app = TestApp::new().await;
    let cases = [
        (
            json!({}),
            json!({ "pilots": ["The pilots field is required."] }),
        ),
        (
            json!({ "pilots": [] }),
            json!({ "pilots": ["The pilots field is required."] }),
        ),
        (
            json!({ "pilots": "Pilot One" }),
            json!({ "pilots": ["The pilots field must be an array."] }),
        ),
        (
            json!({ "pilots": (0..501).map(|i| format!("P{i}")).collect::<Vec<_>>() }),
            json!({ "pilots": ["The pilots field must not have more than 500 items."] }),
        ),
        (
            json!({ "pilots": ["Fine", null, "x".repeat(101), 5, ""] }),
            json!({
                "pilots.1": ["The pilots.1 field is required."],
                "pilots.2": ["The pilots.2 field must not be greater than 100 characters."],
                "pilots.3": ["The pilots.3 field must be a string."],
                "pilots.4": ["The pilots.4 field is required."],
            }),
        ),
    ];
    for (body, errors) in cases {
        let response = create(&app, &body).await;
        response.assert_status(422);
        assert_json_eq(&errors, &response.json()["errors"]);
    }
    assert_eq!(app.count("shares").await, 0);
}

/// FIXES.md: Laravel redirected a client that did not ask for JSON.
#[tokio::test]
async fn a_failed_validation_answers_422_without_a_json_accept_header() {
    let app = TestApp::new().await;
    let response = app
        .request(
            Method::POST,
            "/api/share",
            header_map(&[("host", HOST), ("content-type", "application/json")]),
            Body::from("{}"),
        )
        .await;
    response.assert_status(422);
    assert_json_eq(
        &json!({
            "message": "The pilots field is required.",
            "errors": { "pilots": ["The pilots field is required."] },
        }),
        &response.json(),
    );
}

/// Every pilot list gets its own code.
#[tokio::test]
async fn codes_are_unique() {
    let app = TestApp::new().await;
    let mut codes = std::collections::HashSet::new();
    for _ in 0..20 {
        let response = create(&app, &json!({ "pilots": ["A"] })).await;
        codes.insert(
            response.json()["code"]
                .as_str()
                .expect("a code")
                .to_lowercase(),
        );
    }
    assert_eq!(codes.len(), 20);
}
