//! The harness itself and the route contract: every route the clients
//! call exists in the api (the pages are SvelteKit's,
//! frontend/e2e/routes.spec.ts).

use axum::body::Body;
use axum::http::Method;
use serde_json::Value;

use crate::common::{HOST, TestApp, header_map};

/// Every route of the api: method and path.
const ROUTES: [(&str, &str); 30] = [
    ("GET", "/api/intel/lookup"),
    ("GET", "/api/networks"),
    ("POST", "/api/networks"),
    ("GET", "/api/networks/{network}"),
    ("PUT", "/api/networks/{network}"),
    ("DELETE", "/api/networks/{network}"),
    ("GET", "/api/networks/{network}/access"),
    ("POST", "/api/networks/{network}/access"),
    ("PUT", "/api/networks/{network}/access/{access}"),
    ("DELETE", "/api/networks/{network}/access/{access}"),
    ("GET", "/api/networks/{network}/entries"),
    ("POST", "/api/networks/{network}/entries"),
    ("PUT", "/api/networks/{network}/entries/{entry}"),
    ("DELETE", "/api/networks/{network}/entries/{entry}"),
    ("GET", "/api/networks/{network}/scans"),
    ("POST", "/api/networks/{network}/scans"),
    ("GET", "/api/networks/{network}/scans/{scan}"),
    ("GET", "/api/search"),
    ("POST", "/api/share"),
    ("GET", "/api/share/{code}"),
    ("POST", "/api/tokens"),
    ("DELETE", "/api/tokens/{tokenId}"),
    ("GET", "/api/user"),
    ("GET", "/broadcasting/auth"),
    ("POST", "/broadcasting/auth"),
    ("GET", "/eve"),
    ("GET", "/sanctum/csrf-cookie"),
    ("GET", "/storage/{path}"),
    ("PUT", "/storage/{path}"),
    ("GET", "/up"),
];

#[tokio::test]
async fn every_api_route_exists() {
    let app = TestApp::new().await;
    for (method, uri) in ROUTES {
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
