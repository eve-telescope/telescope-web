//! The website's sign-in (FIXES.md, deliberate changes): the EVE login
//! starts a `telescope_session` cookie for 30 days; the pages share the
//! signed-in user as `auth.user`.

use serde_json::{Value, json};
use time::Duration;

use crate::common::{NOW_Z, TestApp, assert_json_eq};
use crate::eve_login::{callback, esi_knows_the_pilot, start};

async fn signed_in(app: &TestApp) -> String {
    esi_knows_the_pilot(app);
    let (state, cookie) = start(app, "/eve").await;
    callback(app, &state, &cookie)
        .await
        .cookie("telescope_session")
        .expect("a session")
}

#[tokio::test]
async fn a_website_login_signs_the_pages_in() {
    let app = TestApp::new().await;
    let session = signed_in(&app).await;
    for path in ["/api/pages/home", "/api/pages/demo-login"] {
        let response = app.get_with(path, &[("cookie", &session)]).await;
        if path.ends_with("demo-login") {
            // Off in the tests.
            response.assert_status(404);
            continue;
        }
        response.assert_status(200);
        assert_json_eq(
            &json!({
                "id": 1,
                "character_id": 90_000_001,
                "character_name": "Test Pilot",
                "corporation_id": 98_000_001,
                "alliance_id": 99_000_001,
                "last_active_at": null,
                "created_at": NOW_Z,
                "updated_at": NOW_Z,
            }),
            &response.json()["auth"]["user"],
        );
    }
}

#[tokio::test]
async fn an_unknown_session_is_a_guest() {
    let app = TestApp::new().await;
    for cookie in [
        "telescope_session=unknown",
        "telescope-session=eyJpdiI6IkxhcmF2ZWwifQ%3D%3D",
        "remember_web_59ba36addc2b2f9401580f014c7f58ea4e30989d=x",
    ] {
        let response = app.get_with("/api/pages/home", &[("cookie", cookie)]).await;
        response.assert_status(200);
        assert_eq!(response.json()["auth"]["user"], Value::Null, "{cookie}");
    }
}

#[tokio::test]
async fn a_session_ends_after_30_days() {
    let app = TestApp::new().await;
    let session = signed_in(&app).await;
    app.clock.advance(Duration::days(30) - Duration::seconds(1));
    let response = app
        .get_with("/api/pages/home", &[("cookie", &session)])
        .await;
    assert_eq!(response.json()["auth"]["user"]["id"], 1);
    app.clock.advance(Duration::seconds(1));
    let response = app
        .get_with("/api/pages/home", &[("cookie", &session)])
        .await;
    assert_eq!(response.json()["auth"]["user"], Value::Null);
}

/// The session cookie is no API credential.
#[tokio::test]
async fn the_session_does_not_reach_the_api() {
    let app = TestApp::new().await;
    let session = signed_in(&app).await;
    let response = app
        .get_with(
            "/api/user",
            &[("cookie", &session), ("accept", "application/json")],
        )
        .await;
    response.assert_status(401);
}
