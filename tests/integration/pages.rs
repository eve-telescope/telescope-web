//! The props of the SvelteKit pages (`/api/pages/*`): the shared `name`,
//! `auth.user` and `appVersion`, and each page's own.

use serde_json::json;

use crate::common::{TestApp, assert_json_eq};

/// The home page's props; the page itself is in
/// frontend/e2e/routes.spec.ts.
#[tokio::test]
async fn the_home_page_props() {
    let app = TestApp::new().await;
    let response = app.get("/api/pages/home").await;
    response.assert_status(200);
    assert_json_eq(
        &json!({
            "name": "Telescope",
            "auth": { "user": null },
            "appVersion": "v0.5.1",
            "demoLoginEnabled": false,
        }),
        &response.json(),
    );
}

#[tokio::test]
async fn the_home_page_says_when_the_demo_login_is_on() {
    let mut app = TestApp::new().await;
    app.reconfigure(|config| config.demo.enabled = true);
    let response = app.get("/api/pages/home").await;
    assert_eq!(response.json()["demoLoginEnabled"], true);
}

#[tokio::test]
async fn the_name_comes_from_the_config() {
    let mut app = TestApp::new().await;
    app.reconfigure(|config| config.app_name = "Telescope Staging".to_owned());
    let response = app.get("/api/pages/home").await;
    assert_eq!(response.json()["name"], "Telescope Staging");
}
