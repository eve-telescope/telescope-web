//! The routes Laravel had beside the app's own: the health checks, the
//! CSRF cookie route, the local disk routes, and the 409 that reloads
//! tabs left open on the Laravel site.

use axum::body::Body;
use axum::http::Method;
use serde_json::json;

use crate::common::{HOST, TestApp, assert_json_eq, header_map};

#[tokio::test]
async fn the_health_checks_answer() {
    let app = TestApp::new().await;
    let page = app.get("/up").await;
    page.assert_status(200);
    assert!(page.text().contains("Application up"));
    assert!(page.text().contains("<title>Telescope</title>"));
    let json = app.get_with("/up", &[("accept", "application/json")]).await;
    assert_json_eq(&json!({ "status": "up" }), &json.json());
    // Reverb's own health check on the websocket host.
    let reverb = app
        .request(
            Method::GET,
            "/up",
            header_map(&[("host", "ws.eve-telescope.com")]),
            Body::empty(),
        )
        .await;
    assert_json_eq(&json!({ "health": "OK" }), &reverb.json());
    let health = app.get("/api/health").await;
    assert_json_eq(&json!({ "status": "ok", "database": "ok" }), &health.json());
}

/// FIXES.md (deliberate): no cookies, nothing reads them.
#[tokio::test]
async fn the_csrf_cookie_route_answers_without_cookies() {
    let app = TestApp::new().await;
    let response = app.get("/sanctum/csrf-cookie").await;
    response.assert_status(204);
    assert!(response.set_cookies().is_empty());
    assert!(response.body.is_empty());
}

/// Nothing is stored on the local disk.
#[tokio::test]
async fn the_storage_routes_find_nothing() {
    let app = TestApp::new().await;
    let get = app.get("/storage/file.txt").await;
    get.assert_status(404);
    assert!(get.text().contains("<title>Not Found</title>"));
    let put = app
        .request(
            Method::PUT,
            "/storage/file.txt",
            header_map(&[("host", HOST)]),
            Body::from("x"),
        )
        .await;
    put.assert_status(404);
}

/// FIXES.md (deliberate): a request from a tab left open on the Laravel
/// site carries `X-Inertia`; the 409 makes Inertia reload the page in
/// full.
#[tokio::test]
async fn an_inertia_request_reloads_the_page() {
    let app = TestApp::new().await;
    let response = app
        .get_with(
            "/api/pages/s/ABCDEFGH",
            &[("x-inertia", "true"), ("x-inertia-version", "old")],
        )
        .await;
    response.assert_status(409);
    assert_eq!(
        response.header("x-inertia-location"),
        Some("https://eve-telescope.com/api/pages/s/ABCDEFGH")
    );
}
