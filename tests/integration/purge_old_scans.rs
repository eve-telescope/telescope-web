//! The scan purge deletes shared scans older than the retention, `--days`
//! overrides it. The command is
//! the `purge_scans` binary and the hourly task; both call
//! `telescope::scheduler::purge_scans`.

use time::Duration;

use telescope::scheduler::{parse_days_argument, purge_scans, purge_scans_report};

use crate::common::TestApp;

struct Setup {
    app: TestApp,
    owner: i64,
    network: i64,
}

/// One network with its owner.
async fn setup() -> Setup {
    let app = TestApp::new().await;
    let owner = app.user().await;
    let network = app.create_network("Test", owner.id).await;
    Setup {
        owner: owner.id,
        network,
        app,
    }
}

/// A scan created `days_ago` days before now.
async fn create_scan_at(s: &Setup, text: &str, days_ago: i64) {
    s.app
        .create_scan_at(
            s.network,
            s.owner,
            "local",
            text,
            s.app.now() - Duration::days(days_ago),
        )
        .await;
}

async fn texts(app: &TestApp) -> Vec<String> {
    sqlx::query_scalar("select raw_text from network_scans order by id")
        .fetch_all(&app.pool)
        .await
        .expect("scans")
}

#[tokio::test]
async fn deletes_scans_older_than_configured_retention() {
    let s = setup().await;
    create_scan_at(&s, "Old scan", 8).await;
    create_scan_at(&s, "Recent scan", 3).await;
    let purged = purge_scans(&s.app.pool, s.app.now(), 7)
        .await
        .expect("the purge");
    assert_eq!(purged, 1);
    assert_eq!(texts(&s.app).await, ["Recent scan"]);
}

#[tokio::test]
async fn respects_days_override() {
    let s = setup().await;
    create_scan_at(&s, "Two days old", 2).await;
    let days = parse_days_argument(&["--days=1".to_owned()], 7).expect("an argument");
    assert_eq!(days, 1);
    purge_scans(&s.app.pool, s.app.now(), days)
        .await
        .expect("the purge");
    assert!(texts(&s.app).await.is_empty());
}

#[tokio::test]
async fn keeps_all_scans_when_none_are_expired() {
    let s = setup().await;
    s.app
        .create_scan(s.network, s.owner, "local", "Fresh scan")
        .await;
    let purged = purge_scans(&s.app.pool, s.app.now(), 7)
        .await
        .expect("the purge");
    assert_eq!(purged, 0);
    assert_eq!(texts(&s.app).await, ["Fresh scan"]);
}

#[tokio::test]
async fn respects_config_value() {
    let s = setup().await;
    create_scan_at(&s, "Four days old", 4).await;
    create_scan_at(&s, "Two days old", 2).await;
    purge_scans(&s.app.pool, s.app.now(), 3)
        .await
        .expect("the purge");
    assert_eq!(texts(&s.app).await, ["Two days old"]);
}

/// A retention of 0 days keeps every scan.
#[tokio::test]
async fn zero_days_keeps_every_scan() {
    let s = setup().await;
    create_scan_at(&s, "Ancient scan", 400).await;
    s.app
        .create_scan(s.network, s.owner, "local", "Fresh scan")
        .await;
    assert_eq!(
        purge_scans(&s.app.pool, s.app.now(), 0)
            .await
            .expect("the purge"),
        0
    );
    let days = parse_days_argument(&["--days=0".to_owned()], 7).expect("an argument");
    assert_eq!(
        purge_scans(&s.app.pool, s.app.now(), days)
            .await
            .expect("the purge"),
        0
    );
    assert_eq!(texts(&s.app).await.len(), 2);
}

/// The command's arguments and its line of output.
#[test]
fn reads_the_days_argument_and_reports() {
    assert_eq!(parse_days_argument(&[], 7), Ok(7));
    assert_eq!(
        parse_days_argument(&["--days".to_owned(), "3".to_owned()], 7),
        Ok(3)
    );
    assert!(parse_days_argument(&["--days=soon".to_owned()], 7).is_err());
    assert!(parse_days_argument(&["--weeks=1".to_owned()], 7).is_err());
    assert_eq!(
        purge_scans_report(3, 7),
        "Purged 3 scans older than 7 days."
    );
    assert_eq!(
        purge_scans_report(0, 0),
        "Kept every scan (retention 0 days)."
    );
}

/// A scan exactly at the boundary stays (`created_at < now - days`).
#[tokio::test]
async fn a_scan_at_the_boundary_stays() {
    let s = setup().await;
    create_scan_at(&s, "Exactly seven days", 7).await;
    purge_scans(&s.app.pool, s.app.now(), 7)
        .await
        .expect("the purge");
    assert_eq!(texts(&s.app).await, ["Exactly seven days"]);
}
