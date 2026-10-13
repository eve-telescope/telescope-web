//! A user's permission in a network is the highest of
//! the unexpired access rows naming their character, corporation or
//! alliance id (the type is not compared; EVE's id ranges do not overlap).
//! Deleting a network takes an owner row with the user's character id.

use axum::response::Response;
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::models::User;
use crate::server::errors;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Permission {
    Viewer,
    Member,
    Manager,
}

impl Permission {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "viewer" => Some(Self::Viewer),
            "member" => Some(Self::Member),
            "manager" => Some(Self::Manager),
            _ => None,
        }
    }
}

/// The user's permission in a network, if any.
pub async fn of(
    pool: &PgPool,
    network_id: i64,
    user: &User,
    now: OffsetDateTime,
) -> sqlx::Result<Option<Permission>> {
    let rows: Vec<String> = sqlx::query_scalar(
        "select permission from network_access
         where intel_network_id = $1 and accessible_id = any($2)
           and (expires_at is null or expires_at > $3)",
    )
    .bind(network_id)
    .bind(user.accessible_ids())
    .bind(now)
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().filter_map(|text| Permission::parse(text)).max())
}

/// The policy check of a handler: 403 unless the user has at least
/// `needed` in the network.
pub async fn require(
    pool: &PgPool,
    network_id: i64,
    user: &User,
    needed: Permission,
    now: OffsetDateTime,
) -> Result<Permission, Response> {
    match of(pool, network_id, user, now)
        .await
        .map_err(errors::server_error)?
    {
        Some(permission) if permission >= needed => Ok(permission),
        _ => Err(errors::forbidden()),
    }
}

/// Whether the user owns the network (an owner row with their character).
pub async fn is_owner(pool: &PgPool, network_id: i64, user: &User) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "select exists (select 1 from network_access
         where intel_network_id = $1 and accessible_id = $2 and is_owner)",
    )
    .bind(network_id)
    .bind(user.character_id)
    .fetch_one(pool)
    .await
}
