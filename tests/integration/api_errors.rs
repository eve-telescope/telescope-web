//! Error answers of the API as Laravel's exception handler gave them:
//! JSON for clients that ask for it, the error page otherwise, Laravel's
//! messages for unknown routes and methods.

use axum::body::Body;
use axum::http::Method;
use serde_json::json;

use crate::common::{HOST, TestApp, assert_json_eq, header_map};

#[tokio::test]
async fn an_unknown_api_route_is_not_found() {
    let app = TestApp::new().await;
    let json = app.json(Method::GET, "/api/nope", None, None).await;
    json.assert_status(404);
    assert_json_eq(
        &json!({ "message": "The route api/nope could not be found." }),
        &json.json(),
    );
    let page = app.get("/api/nope").await;
    page.assert_status(404);
    assert_eq!(
        page.header("content-type"),
        Some("text/html; charset=utf-8")
    );
    assert!(page.text().contains("<title>Not Found</title>"));
}

#[tokio::test]
async fn a_known_route_with_another_method_is_405() {
    let app = TestApp::new().await;
    let user = app.user().await;
    let token = app.token(&user).await;
    let response = app
        .patch_json(&token, "/api/networks/1", &json!({ "name": "x" }))
        .await;
    response.assert_status(405);
    assert_eq!(response.header("allow"), Some("GET, HEAD, PUT, DELETE"));
    assert_json_eq(
        &json!({ "message": "The PATCH method is not supported for route api/networks/1. Supported methods: GET, HEAD, PUT, DELETE." }),
        &response.json(),
    );
    let page = app
        .request(
            Method::PATCH,
            "/api/networks/1",
            header_map(&[("host", HOST)]),
            Body::empty(),
        )
        .await;
    page.assert_status(405);
    assert_eq!(page.header("allow"), Some("GET, HEAD, PUT, DELETE"));
    assert!(page.text().contains("<title>Method Not Allowed</title>"));
}

/// FIXES.md (deliberate): compact JSON without escaped slashes, where
/// Laravel pretty-printed exceptions.
#[tokio::test]
async fn error_answers_are_compact_json() {
    let app = TestApp::new().await;
    let user = app.user().await;
    let token = app.token(&user).await;
    let response = app.get_json(&token, "/api/networks/999").await;
    response.assert_status(404);
    assert_eq!(
        response.text(),
        r#"{"message":"No query results for model [App\\Models\\IntelNetwork] 999"}"#
    );
    assert_eq!(response.header("content-type"), Some("application/json"));
    let share = app
        .json(
            Method::POST,
            "/api/share",
            None,
            Some(&json!({ "pilots": ["a/b"] })),
        )
        .await;
    assert!(
        share.text().contains("https://eve-telescope.com/s/"),
        "{}",
        share.text()
    );
}

/// A model that is not found is the 404 page for a browser.
#[tokio::test]
async fn a_missing_model_is_the_404_page_for_a_browser() {
    let app = TestApp::new().await;
    let user = app.user().await;
    let token = app.token(&user).await;
    let response = app
        .get_with(
            "/api/networks/999",
            &[("authorization", &format!("Bearer {token}"))],
        )
        .await;
    response.assert_status(404);
    assert!(response.text().contains("<title>Not Found</title>"));
}

/// A trailing slash reaches the same route, without a redirect.
#[tokio::test]
async fn a_trailing_slash_reaches_the_route() {
    let app = TestApp::new().await;
    let user = app.user().await;
    let token = app.token(&user).await;
    app.get_json(&token, "/api/user/").await.assert_status(200);
    app.get_json(&token, "/api/networks/")
        .await
        .assert_status(200);
}

/// A JSON body that does not parse reads as no input.
#[tokio::test]
async fn a_broken_json_body_is_no_input() {
    let app = TestApp::new().await;
    let user = app.user().await;
    let token = app.token(&user).await;
    let response = app
        .request(
            Method::POST,
            "/api/networks",
            header_map(&[
                ("host", HOST),
                ("accept", "application/json"),
                ("content-type", "application/json"),
                ("authorization", &format!("Bearer {token}")),
            ]),
            Body::from("{not json"),
        )
        .await;
    response.assert_status(422);
    assert_eq!(
        response.json()["errors"]["name"],
        json!(["The name field is required."])
    );
}
