//! CORS on `/api/*` and `/broadcasting/auth` (config/cors.php): any
//! origin, the requested method and headers echoed, `max-age` 0, no
//! credentials. The Tauri desktop clients call `/broadcasting/auth` from
//! their webview.

use axum::body::Body;
use axum::http::Method;

use crate::common::{HOST, TestApp, header_map};

async fn preflight(
    app: &TestApp,
    path: &str,
    headers: &[(&str, &str)],
) -> crate::common::TestResponse {
    let mut all = vec![("host", HOST)];
    all.extend_from_slice(headers);
    app.request(Method::OPTIONS, path, header_map(&all), Body::empty())
        .await
}

#[tokio::test]
async fn answers_the_preflight_of_the_channel_auth() {
    let app = TestApp::new().await;
    let response = preflight(
        &app,
        "/broadcasting/auth",
        &[
            ("origin", "tauri://localhost"),
            ("access-control-request-method", "POST"),
            (
                "access-control-request-headers",
                "authorization, content-type",
            ),
        ],
    )
    .await;
    response.assert_status(204);
    assert_eq!(response.header("access-control-allow-origin"), Some("*"));
    assert_eq!(
        response.header("access-control-allow-methods"),
        Some("POST")
    );
    assert_eq!(
        response.header("access-control-allow-headers"),
        Some("authorization, content-type")
    );
    assert_eq!(response.header("access-control-max-age"), Some("0"));
    assert_eq!(
        response.header("vary"),
        Some("Access-Control-Request-Method, Access-Control-Request-Headers")
    );
    assert_eq!(response.header("access-control-allow-credentials"), None);
}

#[tokio::test]
async fn answers_any_api_preflight() {
    let app = TestApp::new().await;
    let response = preflight(
        &app,
        "/api/networks",
        &[
            ("origin", "http://tauri.localhost"),
            ("access-control-request-method", "get"),
        ],
    )
    .await;
    response.assert_status(204);
    assert_eq!(response.header("access-control-allow-methods"), Some("GET"));
    assert_eq!(response.header("access-control-allow-headers"), Some(""));
    let unknown = preflight(
        &app,
        "/api/nope/deeper",
        &[
            ("origin", "http://x.test"),
            ("access-control-request-method", "DELETE"),
            ("access-control-request-headers", "x-custom"),
        ],
    )
    .await;
    unknown.assert_status(204);
    assert_eq!(
        unknown.header("access-control-allow-headers"),
        Some("x-custom")
    );
}

#[tokio::test]
async fn every_api_answer_allows_any_origin() {
    let app = TestApp::new().await;
    let guest = app
        .get_with(
            "/api/networks",
            &[
                ("origin", "tauri://localhost"),
                ("accept", "application/json"),
            ],
        )
        .await;
    guest.assert_status(401);
    assert_eq!(guest.header("access-control-allow-origin"), Some("*"));
    let missing = app.get("/api/share/NOPE0000").await;
    assert_eq!(missing.header("access-control-allow-origin"), Some("*"));
    let auth = app
        .request(
            Method::POST,
            "/broadcasting/auth",
            header_map(&[("host", HOST), ("origin", "tauri://localhost")]),
            Body::empty(),
        )
        .await;
    assert_eq!(auth.header("access-control-allow-origin"), Some("*"));
}

#[tokio::test]
async fn other_paths_have_no_cors() {
    let app = TestApp::new().await;
    for path in ["/up", "/eve", "/api"] {
        let response = app.get_with(path, &[("origin", "http://x.test")]).await;
        assert_eq!(
            response.header("access-control-allow-origin"),
            None,
            "{path}"
        );
    }
}
