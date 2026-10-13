//! The local demo login (FIXES.md, deliberate changes): with
//! DEMO_ENABLED, `/demo-login` signs in as the demo pilot without EVE
//! SSO; `seed_demo` fills its sample networks.

use serde_json::json;

use telescope::demo::{DEMO_CHARACTER_NAME, seed};

use crate::common::{APP_URL, TestApp, assert_json_eq};

async fn enabled() -> TestApp {
    let mut app = TestApp::new().await;
    app.reconfigure(|config| config.demo.enabled = true);
    app
}

#[tokio::test]
async fn the_demo_login_is_not_found_when_off() {
    let app = TestApp::new().await;
    app.get("/api/pages/demo-login").await.assert_status(404);
    app.get("/api/pages/demo-login?desktop")
        .await
        .assert_status(404);
    assert_eq!(app.count("users").await, 0);
}

#[tokio::test]
async fn the_demo_login_issues_a_desktop_token() {
    let app = enabled().await;
    let response = app.get("/api/pages/demo-login?desktop").await;
    response.assert_status(200);
    let body = response.json();
    let token = body["token"].as_str().expect("a token").to_owned();
    assert_json_eq(
        &json!({
            "name": "Telescope",
            "auth": { "user": null },
            "appVersion": "v0.5.1",
            "token": token,
            "deepLink": format!("telescope://auth?token={token}"),
            "characterName": DEMO_CHARACTER_NAME,
        }),
        &body,
    );
    let me = app.get_json(&token, "/api/user").await;
    me.assert_status(200);
    assert_eq!(me.json()["character_id"], 2_100_000_001_i64);
}

#[tokio::test]
async fn the_demo_login_signs_the_website_in() {
    let app = enabled().await;
    let response = app.get("/api/pages/demo-login").await;
    response.assert_status(302);
    assert_eq!(response.location(), APP_URL);
    let session = response.cookie("telescope_session").expect("a session");
    let home = app
        .get_with("/api/pages/home", &[("cookie", &session)])
        .await;
    assert_eq!(
        home.json()["auth"]["user"]["character_name"],
        DEMO_CHARACTER_NAME
    );
}

/// The seed makes the demo pilot's networks, entries, scans and a share,
/// and can run again.
#[tokio::test]
async fn the_seed_fills_the_demo_and_repeats() {
    let app = enabled().await;
    for _ in 0..2 {
        seed(&app.pool, app.now(), 2_100_000_001)
            .await
            .expect("the seed");
    }
    assert_eq!(app.count("users").await, 3);
    assert_eq!(app.count("intel_networks").await, 2);
    assert!(app.count("intel_entries").await >= 4);
    assert!(app.count("network_scans").await >= 3);
    assert_eq!(app.count("shares").await, 1);
    let response = app.get("/api/pages/demo-login?desktop").await;
    let token = response.json()["token"]
        .as_str()
        .expect("a token")
        .to_owned();
    let networks = app.get_json(&token, "/api/networks").await;
    networks.assert_status(200);
    assert_eq!(networks.json().as_array().map(Vec::len), Some(2));
    app.get("/api/pages/s/DEMO2026").await.assert_status(200);
}
