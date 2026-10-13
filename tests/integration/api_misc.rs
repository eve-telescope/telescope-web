//! The routes beside the app's own: the health checks, the CSRF cookie
//! route and the local disk routes.

use axum::body::Body;
use axum::http::Method;
use serde_json::json;

use crate::common::{HOST, TestApp, assert_json_eq, header_map};

#[tokio::test]
async fn the_health_checks_answer() {
    let app = TestApp::new().await;
    let page = app.get("/up").await;
    page.assert_status(200);
    assert_eq!(
        page.header("content-type"),
        Some("text/html; charset=utf-8")
    );
    assert!(page.text().contains("Application up"));
    assert!(page.text().contains("<title>Telescope</title>"));
    let json = app.get_with("/up", &[("accept", "application/json")]).await;
    json.assert_status(200);
    assert_eq!(json.header("content-type"), Some("application/json"));
    assert_eq!(json.header("access-control-allow-origin"), None);
    assert_json_eq(&json!({ "status": "up" }), &json.json());
    // The websocket host's health check.
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

/// No cookies: nothing reads them.
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
