//! The hourly tasks, at minute 0 UTC: the corporation and alliance
//! refresh from ESI, `network:purge-expired-access` and `scans:purge`.
//! Only with SCHEDULER_ENABLED. The tasks also run once as commands
//! (src/bin).

use std::time::Duration as StdDuration;

use sqlx::PgPool;
use time::{Duration, OffsetDateTime, Time};

use crate::server::AppState;

/// The next full hour after `now`.
pub fn next_hour(now: OffsetDateTime) -> OffsetDateTime {
    let hour = now.replace_time(Time::from_hms(now.hour(), 0, 0).expect("an hour"));
    hour + Duration::hours(1)
}

/// `scans:purge`: deletes the scans older than `days` days; 0 keeps them
/// all.
pub async fn purge_scans(pool: &PgPool, now: OffsetDateTime, days: i64) -> sqlx::Result<u64> {
    if days <= 0 {
        return Ok(0);
    }
    let result = sqlx::query("delete from network_scans where created_at < $1")
        .bind(now - Duration::days(days))
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// The command's line of output.
pub fn purge_scans_report(count: u64, days: i64) -> String {
    if days <= 0 {
        format!("Kept every scan (retention {days} days).")
    } else {
        format!("Purged {count} scans older than {days} days.")
    }
}

/// `--days=N` or `--days N` overrides the retention.
pub fn parse_days_argument(args: &[String], default: i64) -> Result<i64, String> {
    let mut days = default;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let value = if let Some(value) = arg.strip_prefix("--days=") {
            value.to_owned()
        } else if arg == "--days" {
            args.next().cloned().ok_or("--days needs a number")?
        } else {
            return Err(format!("unknown argument {arg}"));
        };
        days = value
            .parse()
            .map_err(|_| format!("--days is {value:?}, expected a number"))?;
    }
    Ok(days)
}

/// `network:purge-expired-access`.
pub async fn purge_expired_access(pool: &PgPool, now: OffsetDateTime) -> sqlx::Result<u64> {
    let result =
        sqlx::query("delete from network_access where expires_at is not null and expires_at < $1")
            .bind(now)
            .execute(pool)
            .await?;
    Ok(result.rows_affected())
}

pub fn purge_expired_access_report(count: u64) -> String {
    format!("Purged {count} expired network access entries.")
}

/// Runs every hourly task once.
pub async fn run_hourly(state: &AppState) {
    let updated = crate::affiliations::refresh_all(state).await;
    tracing::info!(updated, "affiliations refreshed");
    let now = state.clock.now();
    match purge_expired_access(&state.pool, now).await {
        Ok(count) => tracing::info!("{}", purge_expired_access_report(count)),
        Err(error) => tracing::error!(%error, "expired access not purged"),
    }
    let days = state.config.scan_retention_days;
    match purge_scans(&state.pool, now, days).await {
        Ok(count) => tracing::info!("{}", purge_scans_report(count, days)),
        Err(error) => tracing::error!(%error, "old scans not purged"),
    }
}

/// Starts the hourly loop: housekeeping (expired website sessions)
/// always, the hourly tasks when the schedule is on.
pub fn spawn(state: AppState) {
    if !state.config.scheduler_enabled {
        tracing::info!("the hourly tasks are off (SCHEDULER_ENABLED)");
    }
    tokio::spawn(async move {
        loop {
            let now = state.clock.now();
            let wait = (next_hour(now) - now).whole_milliseconds().max(0) as u64;
            tokio::time::sleep(StdDuration::from_millis(wait)).await;
            if let Err(error) = crate::auth::session::delete_expired(
                &state.pool,
                state.clock.now(),
                state.config.session_lifetime_days,
            )
            .await
            {
                tracing::error!(%error, "expired sessions not cleared");
            }
            if state.config.scheduler_enabled {
                run_hourly(&state).await;
            }
        }
    });
}
