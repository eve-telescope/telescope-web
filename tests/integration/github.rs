//! The latest desktop release on every page (`appVersion`, GitHubService):
//! asked from the mock GitHub, kept for an hour, `null` when GitHub fails
//! or takes longer than 5 seconds.

use std::time::Duration as StdDuration;

use serde_json::Value;
use time::Duration;

use crate::common::TestApp;

async fn version(app: &TestApp) -> Value {
    let response = app.get("/api/pages/home").await;
    response.assert_status(200);
    response.json()["appVersion"].clone()
}

#[tokio::test]
async fn shows_the_latest_release_and_keeps_it_for_an_hour() {
    let app = TestApp::new().await;
    app.github.latest("v0.6.0");
    assert_eq!(version(&app).await, "v0.6.0");
    app.github.latest("v0.7.0");
    app.clock.advance(Duration::minutes(59));
    assert_eq!(version(&app).await, "v0.6.0");
    assert_eq!(app.github.requests().len(), 1);
    app.clock.advance(Duration::minutes(1));
    assert_eq!(version(&app).await, "v0.7.0");
    let requests = app.github.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[0].path,
        "/repos/eve-telescope/telescope-app/releases/latest"
    );
    assert!(requests[0].header("user-agent").is_some());
}

/// A failure is cached too, so a failing GitHub is not asked on every
/// page.
#[tokio::test]
async fn a_failure_is_remembered_for_five_minutes() {
    let app = TestApp::new().await;
    app.github.fail_with(403);
    assert_eq!(version(&app).await, Value::Null);
    app.github.recover();
    app.clock.advance(Duration::minutes(4));
    assert_eq!(version(&app).await, Value::Null);
    assert_eq!(app.github.requests().len(), 1);
    app.clock.advance(Duration::minutes(1));
    assert_eq!(version(&app).await, "v0.5.1");
    assert_eq!(app.github.requests().len(), 2);
}

/// GitHub gets 5 seconds.
#[tokio::test]
async fn a_slow_github_is_no_version() {
    let app = TestApp::new().await;
    app.github.delay(StdDuration::from_secs(7));
    let started = std::time::Instant::now();
    assert_eq!(version(&app).await, Value::Null);
    let waited = started.elapsed();
    assert!(waited >= StdDuration::from_secs(5), "{waited:?}");
    assert!(waited < StdDuration::from_secs(7), "{waited:?}");
}
