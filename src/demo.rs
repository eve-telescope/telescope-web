//! The local demo login. Not ported yet.

use sqlx::PgPool;
use time::OffsetDateTime;

pub const DEMO_CHARACTER_NAME: &str = "Demo Pilot";

pub async fn seed(_pool: &PgPool, _now: OffsetDateTime, _character_id: i64) -> sqlx::Result<()> {
    unimplemented!("the demo seed")
}
