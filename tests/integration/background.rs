//! The hourly schedule: `network:purge-expired-access` and when the
//! hourly tasks run.

use time::Duration;
use time::macros::datetime;

use telescope::scheduler::{next_hour, purge_expired_access, purge_expired_access_report};

use crate::common::TestApp;
use crate::common::factories::USER;

#[tokio::test]
async fn purges_expired_access() {
    let app = TestApp::new().await;
    let owner = app.user().await;
    let network = app.create_network("Net", owner.id).await;
    app.grant_owner(network, &owner).await;
    let expired = app
        .grant(
            network,
            USER,
            1,
            "viewer",
            false,
            Some(app.now() - Duration::seconds(1)),
        )
        .await;
    let now = app
        .grant(network, USER, 2, "viewer", false, Some(app.now()))
        .await;
    let later = app
        .grant(
            network,
            USER,
            3,
            "viewer",
            false,
            Some(app.now() + Duration::days(1)),
        )
        .await;
    let purged = purge_expired_access(&app.pool, app.now())
        .await
        .expect("the purge");
    assert_eq!(purged, 1);
    assert_eq!(app.row("network_access", expired).await, None);
    assert!(app.row("network_access", now).await.is_some());
    assert!(app.row("network_access", later).await.is_some());
    assert_eq!(app.count("network_access").await, 3);
    assert_eq!(
        purge_expired_access_report(1),
        "Purged 1 expired network access entries."
    );
}

#[test]
fn the_tasks_run_at_the_top_of_every_hour_in_utc() {
    assert_eq!(
        next_hour(datetime!(2026-10-13 08:30:00 UTC)),
        datetime!(2026-10-13 09:00:00 UTC)
    );
    assert_eq!(
        next_hour(datetime!(2026-10-13 09:00:00 UTC)),
        datetime!(2026-10-13 10:00:00 UTC)
    );
    assert_eq!(
        next_hour(datetime!(2026-10-13 23:59:59 UTC)),
        datetime!(2026-10-14 00:00:00 UTC)
    );
}
