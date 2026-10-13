//! The hourly tasks of routes/console.php. Not ported yet.

use sqlx::PgPool;
use time::OffsetDateTime;

pub async fn purge_scans(_pool: &PgPool, _now: OffsetDateTime, _days: i64) -> sqlx::Result<u64> {
    unimplemented!("scans:purge")
}

pub fn parse_days_argument(_args: &[String], _default: i64) -> Result<i64, String> {
    unimplemented!("scans:purge --days")
}

pub fn purge_scans_report(_count: u64, _days: i64) -> String {
    unimplemented!("scans:purge output")
}

pub async fn purge_expired_access(_pool: &PgPool, _now: OffsetDateTime) -> sqlx::Result<u64> {
    unimplemented!("network:purge-expired-access")
}

pub fn purge_expired_access_report(_count: u64) -> String {
    unimplemented!("network:purge-expired-access output")
}

pub fn next_hour(_now: OffsetDateTime) -> OffsetDateTime {
    unimplemented!("the schedule")
}
