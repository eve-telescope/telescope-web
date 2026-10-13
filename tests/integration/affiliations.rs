//! jobs/RefreshAffiliations.php, which the PHP suite never covered: every
//! user's corporation and alliance from the mock ESI, in chunks of 1000,
//! fetching corporations and alliances that are missing or older than 24
//! hours. The hourly task and the `refresh_affiliations` command.

use serde_json::{Value, json};
use time::Duration;

use telescope::affiliations::refresh_all;

use crate::common::TestApp;
use crate::common::factories::NewUser;

#[tokio::test]
async fn refreshes_every_user_from_esi() {
    let app = TestApp::new().await;
    let first = app.user().await;
    let second = app.user().await;
    let gone = app.user().await;
    app.esi
        .affiliation(first.character_id, 98_000_001, Some(99_000_001));
    app.esi.affiliation(second.character_id, 98_000_002, None);
    app.esi
        .alliance(99_000_001, "Goonswarm Federation", "CONDI");
    app.esi
        .corporation(98_000_001, "Karmafleet", "KARMA", Some(99_000_001), 5321);
    app.esi
        .corporation(98_000_002, "Solo Corp", "SOLO", None, 1);
    refresh_all(&app.state).await;
    let user = app.row("users", first.id).await.expect("a user");
    assert_eq!(user["corporation_id"], 98_000_001);
    assert_eq!(user["alliance_id"], 99_000_001);
    let user = app.row("users", second.id).await.expect("a user");
    assert_eq!(user["corporation_id"], 98_000_002);
    assert_eq!(user["alliance_id"], Value::Null);
    // ESI did not answer for this character: it keeps what it had.
    let user = app.row("users", gone.id).await.expect("a user");
    assert_eq!(user["corporation_id"], Value::Null);
    let corporation = app
        .row("corporations", 98_000_002)
        .await
        .expect("a corporation");
    assert_eq!(corporation["name"], "Solo Corp");
    assert_eq!(corporation["last_updated"], "2026-10-13T08:30:00+00:00");
    let requests = app.esi.requests();
    assert_eq!(
        requests[0].json(),
        json!([first.character_id, second.character_id, gone.character_id])
    );
    assert_eq!(
        requests[0].header("user-agent"),
        Some("Telescope | https://eve-telescope.com")
    );
}

#[tokio::test]
async fn asks_in_chunks_of_1000() {
    let app = TestApp::new().await;
    sqlx::query(
        "insert into users (character_id, character_name, character_owner_hash, created_at, updated_at)
         select 90000000 + n, 'pilot' || n, 'hash', $1, $1 from generate_series(1, 1001) n",
    )
    .bind(app.now())
    .execute(&app.pool)
    .await
    .expect("users insert");
    refresh_all(&app.state).await;
    let chunks: Vec<usize> = app
        .esi
        .requests()
        .iter()
        .map(|request| request.json().as_array().map_or(0, Vec::len))
        .collect();
    assert_eq!(chunks, [1000, 1]);
}

/// A failed chunk is skipped; the users keep their affiliation.
#[tokio::test]
async fn a_failed_chunk_changes_nothing() {
    let app = TestApp::new().await;
    app.create_corporation(98_000_001, "Karmafleet", "KARMA", None)
        .await;
    let user = app
        .create_user(NewUser {
            corporation_id: Some(98_000_001),
            ..NewUser::default()
        })
        .await;
    app.esi.fail_with(502);
    refresh_all(&app.state).await;
    let row = app.row("users", user.id).await.expect("a user");
    assert_eq!(row["corporation_id"], 98_000_001);
}

/// Corporations and alliances fetched within 24 hours are kept; older
/// ones are fetched again.
#[tokio::test]
async fn refetches_only_stale_rows() {
    let app = TestApp::new().await;
    let user = app.user().await;
    app.create_alliance(99_000_001, "Old Name", "OLD").await;
    app.create_corporation(98_000_001, "Karmafleet", "KARMA", Some(99_000_001))
        .await;
    sqlx::query("update alliances set last_updated = $1")
        .bind(app.now() - Duration::hours(25))
        .execute(&app.pool)
        .await
        .expect("alliance stamped");
    sqlx::query("update corporations set last_updated = $1")
        .bind(app.now() - Duration::hours(2))
        .execute(&app.pool)
        .await
        .expect("corporation stamped");
    app.esi
        .affiliation(user.character_id, 98_000_001, Some(99_000_001));
    app.esi
        .alliance(99_000_001, "Goonswarm Federation", "CONDI");
    refresh_all(&app.state).await;
    assert_eq!(
        app.esi.paths(),
        [
            "POST /latest/characters/affiliation/",
            "GET /latest/alliances/99000001/",
        ]
    );
    let alliance = app.row("alliances", 99_000_001).await.expect("an alliance");
    assert_eq!(alliance["name"], "Goonswarm Federation");
}

/// FIXES.md: Laravel never refreshed a row without `last_updated`.
#[tokio::test]
async fn a_row_never_fetched_is_fetched() {
    let app = TestApp::new().await;
    let user = app.user().await;
    app.create_corporation(98_000_001, "Placeholder", "TMP", None)
        .await;
    app.esi.affiliation(user.character_id, 98_000_001, None);
    app.esi
        .corporation(98_000_001, "Karmafleet", "KARMA", None, 10);
    refresh_all(&app.state).await;
    let corporation = app
        .row("corporations", 98_000_001)
        .await
        .expect("a corporation");
    assert_eq!(corporation["name"], "Karmafleet");
}

/// FIXES.md: Laravel's job failed on the foreign key and stopped.
#[tokio::test]
async fn a_corporation_esi_cannot_load_does_not_stop_the_refresh() {
    let app = TestApp::new().await;
    let first = app.user().await;
    let second = app.user().await;
    app.esi.affiliation(first.character_id, 98_000_001, None);
    app.esi.affiliation(second.character_id, 98_000_002, None);
    app.esi.fail_path("/latest/corporations/98000001/", 500);
    // This corporation names an alliance ESI does not know.
    app.esi
        .corporation(98_000_002, "Solo Corp", "SOLO", Some(99_000_404), 1);
    refresh_all(&app.state).await;
    let user = app.row("users", first.id).await.expect("a user");
    assert_eq!(user["corporation_id"], Value::Null);
    let user = app.row("users", second.id).await.expect("a user");
    assert_eq!(user["corporation_id"], 98_000_002);
    let corporation = app
        .row("corporations", 98_000_002)
        .await
        .expect("a corporation");
    assert_eq!(corporation["alliance_id"], Value::Null);
}

/// No users, no request.
#[tokio::test]
async fn nobody_to_refresh_asks_nothing() {
    let app = TestApp::new().await;
    refresh_all(&app.state).await;
    assert!(app.esi.requests().is_empty());
}
