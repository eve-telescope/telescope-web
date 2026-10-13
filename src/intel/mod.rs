//! Intel networks: networks shared with characters, corporations and
//! alliances, their access rows, tagged entries, shared scans, the bulk
//! lookup and the entity search.
//!
//! Route ids resolve before anything is authorized: an id that names no
//! row is a 404 that names the model (`errors::model_not_found`). An
//! entry or access row must belong to the network of the URL; a scan is
//! checked after the network is authorized.

pub mod access;
pub mod entries;
pub mod networks;
pub mod permission;
pub mod scans;
pub mod search;

use axum::response::Response;
use serde_json::{Value, json};
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::dates;
use crate::server::errors;

/// A route id: whole digits only (`5abc` is no id).
pub fn route_id(raw: &str) -> Option<i64> {
    (!raw.is_empty() && raw.bytes().all(|c| c.is_ascii_digit()))
        .then(|| raw.parse().ok())
        .flatten()
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Network {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub owner_id: i64,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl Network {
    /// The `{network}` of a route.
    pub async fn bind(pool: &PgPool, raw: &str) -> Result<Network, Response> {
        let network: Option<Network> = match route_id(raw) {
            Some(id) => sqlx::query_as(
                "select id, name, slug, owner_id, created_at, updated_at from intel_networks
                 where id = $1",
            )
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(errors::server_error)?,
            None => None,
        };
        network.ok_or_else(|| errors::model_not_found("IntelNetwork", raw))
    }

    /// The model's JSON, columns in order.
    pub fn json(&self) -> Value {
        json!({
            "id": self.id,
            "name": self.name,
            "slug": self.slug,
            "owner_id": self.owner_id,
            "created_at": dates::model(self.created_at),
            "updated_at": dates::model(self.updated_at),
        })
    }
}
