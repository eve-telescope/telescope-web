//! The EVE login the PHP suite never covered: `/eve` sends the browser to
//! EVE SSO, the callback page (`/eve/callback`, its props from
//! `/api/pages/eve/callback`) checks the state, trades the code for a
//! token at the mock SSO, verifies it against the mock JWKS, refreshes the
//! affiliation from the mock ESI and either shows the desktop token or
//! signs the website in.

use serde_json::{Value, json};

use crate::common::factories::NewUser;
use crate::common::mocks::SSO_KEY_ID;
use crate::common::{APP_URL, EVE_CLIENT_ID, EVE_CLIENT_SECRET, TestApp, assert_json_eq};

/// The state cookie's name.
const STATE_COOKIE: &str = "telescope_sso";

/// Starts a login; answers the state and the cookie for the callback.
pub async fn start(app: &TestApp, path: &str) -> (String, String) {
    let response = app.get(path).await;
    response.assert_status(302);
    let location = response.location().to_owned();
    let state = location
        .split("&state=")
        .nth(1)
        .expect("a state")
        .to_owned();
    let cookie = response.cookie(STATE_COOKIE).expect("the state cookie");
    (state, cookie)
}

pub async fn callback(app: &TestApp, state: &str, cookie: &str) -> crate::common::TestResponse {
    app.get_with(
        &format!(
            "/api/pages/eve/callback?code={}&state={state}",
            app.sso.code()
        ),
        &[("cookie", cookie)],
    )
    .await
}

/// ESI knows the pilot's corporation and alliance.
pub fn esi_knows_the_pilot(app: &TestApp) {
    app.esi
        .affiliation(90_000_001, 98_000_001, Some(99_000_001));
    app.esi
        .alliance(99_000_001, "Goonswarm Federation", "CONDI");
    app.esi
        .corporation(98_000_001, "Karmafleet", "KARMA", Some(99_000_001), 5321);
}

#[tokio::test]
async fn redirects_to_eve_sso_with_a_state() {
    let app = TestApp::new().await;
    let response = app.get("/eve").await;
    response.assert_status(302);
    let location = response.location();
    let (prefix, state) = location.split_once("&state=").expect("a state");
    assert_eq!(
        prefix,
        format!(
            "{}/v2/oauth/authorize/?client_id={EVE_CLIENT_ID}&redirect_uri=https%3A%2F%2Feve-telescope.com%2Feve%2Fcallback&scope=&response_type=code",
            app.sso.url()
        )
    );
    assert_eq!(state.len(), 40);
    assert!(state.chars().all(|c| c.is_ascii_alphanumeric()));
    let cookie = response
        .set_cookies()
        .into_iter()
        .find(|cookie| cookie.starts_with(STATE_COOKIE))
        .expect("the state cookie");
    assert!(cookie.contains("HttpOnly"), "{cookie}");
    assert!(cookie.contains("SameSite=Lax"), "{cookie}");
    assert!(cookie.contains("Secure"), "{cookie}");
    assert!(cookie.contains("Path=/eve"), "{cookie}");
}

#[tokio::test]
async fn a_website_login_signs_in_and_goes_home() {
    let app = TestApp::new().await;
    esi_knows_the_pilot(&app);
    app.sso.sign_in_as(90_000_001, "Test Pilot", "owner-hash-1");
    let (state, cookie) = start(&app, "/eve").await;
    let response = callback(&app, &state, &cookie).await;
    response.assert_status(302);
    assert_eq!(response.location(), APP_URL);
    let session = response
        .set_cookies()
        .into_iter()
        .find(|cookie| cookie.starts_with("telescope_session="))
        .expect("a session");
    assert!(session.contains("HttpOnly"), "{session}");
    assert!(session.contains("SameSite=Lax"), "{session}");
    assert!(session.contains("Max-Age=2592000"), "{session}");
    // The state cookie is used up.
    let cleared = response
        .set_cookies()
        .into_iter()
        .find(|cookie| cookie.starts_with(STATE_COOKIE))
        .expect("the cleared state cookie");
    assert!(cleared.contains("Max-Age=0"), "{cleared}");

    let user = app.row("users", 1).await.expect("the user");
    assert_eq!(user["character_id"], 90_000_001);
    assert_eq!(user["character_name"], "Test Pilot");
    assert_eq!(user["character_owner_hash"], "owner-hash-1");
    assert_eq!(user["corporation_id"], 98_000_001);
    assert_eq!(user["alliance_id"], 99_000_001);
    assert_eq!(app.count("personal_access_tokens").await, 0);

    // The code was traded with basic auth, the token checked against the
    // JWKS.
    let requests = app.sso.requests();
    assert_eq!(requests[0].path, "/v2/oauth/token");
    let basic = format!(
        "Basic {}",
        base64_encode(&format!("{EVE_CLIENT_ID}:{EVE_CLIENT_SECRET}"))
    );
    assert_eq!(requests[0].header("authorization"), Some(basic.as_str()));
    assert_eq!(
        requests[0].text(),
        format!("grant_type=authorization_code&code={}", app.sso.code())
    );
    assert_eq!(requests[1].path, "/oauth/jwks");
}

#[tokio::test]
async fn a_desktop_login_shows_a_token() {
    let app = TestApp::new().await;
    esi_knows_the_pilot(&app);
    app.sso.sign_in_as(90_000_001, "Test Pilot", "owner-hash-1");
    let (state, cookie) = start(&app, "/eve?desktop").await;
    let response = callback(&app, &state, &cookie).await;
    response.assert_status(200);
    let body = response.json();
    let token = body["token"].as_str().expect("a token").to_owned();
    assert!(token.starts_with("1|"), "{token}");
    assert_eq!(token.len(), 2 + 40 + 8);
    assert_json_eq(
        &json!({
            "name": "Telescope",
            "auth": { "user": null },
            "appVersion": "v0.5.1",
            "token": token,
            "deepLink": format!("telescope://auth?token={token}"),
            "characterName": "Test Pilot",
        }),
        &body,
    );
    assert!(response.cookie("telescope_session").is_none());
    let row = app
        .row("personal_access_tokens", 1)
        .await
        .expect("the token");
    assert_eq!(row["name"], "Telescope Desktop");
    // The token is the desktop app's from now on.
    let me = app.get_json(&token, "/api/user").await;
    me.assert_status(200);
    assert_eq!(me.json()["character_id"], 90_000_001);
}

#[tokio::test]
async fn every_desktop_login_adds_a_token_and_updates_the_pilot() {
    let app = TestApp::new().await;
    esi_knows_the_pilot(&app);
    app.create_user(NewUser {
        character_id: Some(90_000_001),
        character_name: Some("Old Name".to_owned()),
        ..NewUser::default()
    })
    .await;
    for _ in 0..2 {
        let (state, cookie) = start(&app, "/eve?desktop=1").await;
        callback(&app, &state, &cookie).await.assert_status(200);
    }
    assert_eq!(app.count("users").await, 1);
    assert_eq!(app.count("personal_access_tokens").await, 2);
    let user = app.row("users", 1).await.expect("the user");
    assert_eq!(user["character_name"], "Test Pilot");
    assert_eq!(user["character_owner_hash"], "owner-hash-1");
    assert_eq!(user["corporation_id"], 98_000_001);
}

#[tokio::test]
async fn the_login_stores_the_corporation_and_alliance() {
    let app = TestApp::new().await;
    esi_knows_the_pilot(&app);
    let (state, cookie) = start(&app, "/eve").await;
    callback(&app, &state, &cookie).await.assert_status(302);
    let alliance = app
        .row("alliances", 99_000_001)
        .await
        .expect("the alliance");
    assert_eq!(alliance["name"], "Goonswarm Federation");
    assert_eq!(alliance["ticker"], "CONDI");
    assert!(!alliance["last_updated"].is_null());
    let corporation = app
        .row("corporations", 98_000_001)
        .await
        .expect("the corporation");
    assert_eq!(corporation["name"], "Karmafleet");
    assert_eq!(corporation["alliance_id"], 99_000_001);
    assert_eq!(corporation["member_count"], 5321);
    // ESI was asked in this order, with the app's user agent.
    assert_eq!(
        app.esi.paths(),
        [
            "POST /latest/characters/affiliation/",
            "GET /latest/alliances/99000001/",
            "GET /latest/corporations/98000001/",
        ]
    );
    let affiliation = &app.esi.requests()[0];
    assert_eq!(affiliation.json(), json!([90_000_001]));
    assert_eq!(
        affiliation.header("user-agent"),
        Some("Telescope | https://eve-telescope.com")
    );
}

/// Rows fetched less than 24 hours ago are not fetched again.
#[tokio::test]
async fn fresh_corporations_are_not_fetched_again() {
    let app = TestApp::new().await;
    esi_knows_the_pilot(&app);
    app.create_alliance(99_000_001, "Goonswarm Federation", "CONDI")
        .await;
    app.create_corporation(98_000_001, "Karmafleet", "KARMA", Some(99_000_001))
        .await;
    sqlx::query("update alliances set last_updated = $1")
        .bind(app.now() - time::Duration::hours(24))
        .execute(&app.pool)
        .await
        .expect("alliance stamped");
    sqlx::query("update corporations set last_updated = $1")
        .bind(app.now() - time::Duration::hours(24) - time::Duration::seconds(1))
        .execute(&app.pool)
        .await
        .expect("corporation stamped");
    let (state, cookie) = start(&app, "/eve").await;
    callback(&app, &state, &cookie).await.assert_status(302);
    assert_eq!(
        app.esi.paths(),
        [
            "POST /latest/characters/affiliation/",
            "GET /latest/corporations/98000001/",
        ]
    );
}

/// FIXES.md: Laravel threw InvalidStateException (500).
#[tokio::test]
async fn a_callback_with_a_wrong_state_has_expired() {
    let app = TestApp::new().await;
    let (_, cookie) = start(&app, "/eve").await;
    let response = callback(&app, "not-the-state", &cookie).await;
    response.assert_status(419);
    let without_cookie = app
        .get("/api/pages/eve/callback?code=valid-code&state=abc")
        .await;
    without_cookie.assert_status(419);
    assert!(app.sso.requests().is_empty());
    assert_eq!(app.count("users").await, 0);
}

/// A state works once.
#[tokio::test]
async fn a_replayed_callback_has_expired() {
    let app = TestApp::new().await;
    esi_knows_the_pilot(&app);
    let (state, cookie) = start(&app, "/eve").await;
    callback(&app, &state, &cookie).await.assert_status(302);
    // The browser dropped the cookie; a replay with the old one fails.
    let replay = callback(&app, &state, &cookie).await;
    replay.assert_status(419);
}

/// FIXES.md: Laravel tried to trade no code (500).
#[tokio::test]
async fn a_cancelled_login_goes_home() {
    let app = TestApp::new().await;
    let (state, cookie) = start(&app, "/eve?desktop").await;
    let response = app
        .get_with(
            &format!("/api/pages/eve/callback?error=access_denied&state={state}"),
            &[("cookie", &cookie)],
        )
        .await;
    response.assert_status(302);
    assert_eq!(response.location(), APP_URL);
    assert!(app.sso.requests().is_empty());
}

/// FIXES.md: Laravel's session kept the desktop flag.
#[tokio::test]
async fn a_website_login_after_an_abandoned_desktop_login_signs_in() {
    let app = TestApp::new().await;
    esi_knows_the_pilot(&app);
    start(&app, "/eve?desktop").await;
    let (state, cookie) = start(&app, "/eve").await;
    let response = callback(&app, &state, &cookie).await;
    response.assert_status(302);
    assert!(response.cookie("telescope_session").is_some());
    assert_eq!(app.count("personal_access_tokens").await, 0);
}

/// FIXES.md: Laravel saved the user without corporation and alliance.
#[tokio::test]
async fn an_esi_failure_keeps_the_known_affiliation() {
    let app = TestApp::new().await;
    app.create_alliance(99_000_001, "Goonswarm Federation", "CONDI")
        .await;
    app.create_corporation(98_000_001, "Karmafleet", "KARMA", Some(99_000_001))
        .await;
    app.create_user(NewUser {
        character_id: Some(90_000_001),
        corporation_id: Some(98_000_001),
        alliance_id: Some(99_000_001),
        ..NewUser::default()
    })
    .await;
    app.esi.fail_with(503);
    let (state, cookie) = start(&app, "/eve").await;
    callback(&app, &state, &cookie).await.assert_status(302);
    let user = app.row("users", 1).await.expect("the user");
    assert_eq!(user["corporation_id"], 98_000_001);
    assert_eq!(user["alliance_id"], 99_000_001);
}

/// A new pilot whose affiliation ESI could not answer has none.
#[tokio::test]
async fn a_new_pilot_without_an_esi_answer_has_no_affiliation() {
    let app = TestApp::new().await;
    app.esi.fail_with(500);
    let (state, cookie) = start(&app, "/eve").await;
    callback(&app, &state, &cookie).await.assert_status(302);
    let user = app.row("users", 1).await.expect("the user");
    assert_eq!(user["corporation_id"], Value::Null);
    assert_eq!(user["alliance_id"], Value::Null);
}

/// FIXES.md: a corporation ESI could not load failed the login (500).
#[tokio::test]
async fn a_corporation_esi_cannot_load_does_not_fail_the_login() {
    let app = TestApp::new().await;
    app.esi
        .affiliation(90_000_001, 98_000_001, Some(99_000_001));
    app.esi
        .alliance(99_000_001, "Goonswarm Federation", "CONDI");
    app.esi.fail_path("/latest/corporations/98000001/", 502);
    let (state, cookie) = start(&app, "/eve").await;
    callback(&app, &state, &cookie).await.assert_status(302);
    let user = app.row("users", 1).await.expect("the user");
    assert_eq!(user["corporation_id"], Value::Null);
    assert_eq!(user["alliance_id"], 99_000_001);
}

/// A token the SSO would not issue or that does not verify fails the
/// login.
#[tokio::test]
async fn an_invalid_sso_answer_fails_the_login() {
    let app = TestApp::new().await;
    app.sso.fail_token_with(400);
    let (state, cookie) = start(&app, "/eve").await;
    callback(&app, &state, &cookie).await.assert_status(500);

    let app = TestApp::new().await;
    app.sso.issuer("https://evil.example");
    let (state, cookie) = start(&app, "/eve").await;
    callback(&app, &state, &cookie).await.assert_status(500);

    let app = TestApp::new().await;
    app.sso.expires_in(-120);
    let (state, cookie) = start(&app, "/eve").await;
    callback(&app, &state, &cookie).await.assert_status(500);

    let app = TestApp::new().await;
    app.sso.fail_jwks_with(503);
    let (state, cookie) = start(&app, "/eve").await;
    callback(&app, &state, &cookie).await.assert_status(500);
    assert_eq!(app.count("users").await, 0);
}

/// Both issuer spellings EVE uses are accepted.
#[tokio::test]
async fn the_short_issuer_is_accepted() {
    let app = TestApp::new().await;
    app.sso.issuer("login.eveonline.com");
    let (state, cookie) = start(&app, "/eve?desktop").await;
    callback(&app, &state, &cookie).await.assert_status(200);
    assert_eq!(SSO_KEY_ID, "JWT-Signature-Key");
}

fn base64_encode(text: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = text.as_bytes();
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}
