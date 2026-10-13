//! The harness itself and the route contract: every route of the Laravel
//! app (tests/fixtures/routes.json) exists in the api, unless SvelteKit
//! renders it.

use axum::body::Body;
use axum::http::Method;
use serde_json::Value;

use crate::common::{HOST, TestApp, header_map, read_routes};

/// The page routes SvelteKit renders (frontend/e2e/routes.spec.ts).
const PAGES: [&str; 3] = ["/", "/s/{code}", "/eve/callback"];

#[tokio::test]
async fn every_api_route_of_the_laravel_app_exists() {
    let app = TestApp::new().await;
    for (method, uri) in read_routes() {
        if PAGES.contains(&uri.as_str()) || method == "HEAD" {
            continue;
        }
        let path = uri.replace("{path}", "file.txt").replace(['{', '}'], "");
        let response = app
            .request(
                method.parse::<Method>().expect("a method"),
                &path,
                header_map(&[("host", HOST), ("accept", "application/json")]),
                Body::empty(),
            )
            .await;
        assert_ne!(response.status.as_u16(), 405, "{method} {uri}");
        if response.status.as_u16() == 404
            && let Ok(body) = serde_json::from_slice::<Value>(&response.body)
        {
            let message = body["message"].as_str().unwrap_or_default();
            assert!(
                !message.contains("could not be found"),
                "{method} {uri}: {message}"
            );
        }
    }
}

#[tokio::test]
async fn the_mocks_answer() {
    let app = TestApp::new().await;
    app.esi.affiliation(90_000_001, 98_000_001, None);
    let client = reqwest::Client::new();
    let answer: Value = client
        .post(format!("{}/latest/characters/affiliation/", app.esi.url()))
        .json(&[90_000_001])
        .send()
        .await
        .expect("ESI answers")
        .json()
        .await
        .expect("JSON");
    assert_eq!(
        answer,
        serde_json::json!([{ "character_id": 90_000_001, "corporation_id": 98_000_001 }])
    );
    let release: Value = client
        .get(format!(
            "{}/repos/eve-telescope/telescope-app/releases/latest",
            app.github.url()
        ))
        .send()
        .await
        .expect("GitHub answers")
        .json()
        .await
        .expect("JSON");
    assert_eq!(release["tag_name"], "v0.5.1");
    assert_eq!(app.esi.requests().len(), 1);
    assert_eq!(app.github.requests().len(), 1);
}

#[tokio::test]
async fn the_health_page_answers() {
    let app = TestApp::new().await;
    let response = app.get("/up").await;
    response.assert_status(200);
    assert!(response.text().contains("Application up"));
}
