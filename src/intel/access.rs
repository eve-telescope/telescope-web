//! Network access (NetworkAccessController, NetworkAccessData): who may
//! use a network. Rows keep the PHP class names as their type; the API
//! answers plain types with the named entity, except `POST
//! /api/networks`, which has always answered the raw row.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use sqlx::PgPool;
use time::OffsetDateTime;

use super::Network;
use super::permission::{self, Permission};
use crate::auth::tokens::ApiUser;
use crate::dates;
use crate::input::Input;
use crate::models::{self, ALLIANCE_CLASS, CORPORATION_CLASS, USER_CLASS};
use crate::server::AppState;
use crate::server::errors;
use crate::validation::{Validator, integer};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Access {
    pub id: i64,
    pub intel_network_id: i64,
    pub accessible_type: String,
    pub accessible_id: i64,
    pub permission: String,
    pub is_owner: bool,
    pub expires_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

const COLUMNS: &str = "id, intel_network_id, accessible_type, accessible_id, permission, is_owner, expires_at, created_at, updated_at";

impl Access {
    /// The raw model, as `POST /api/networks` answers it.
    pub fn raw_json(&self) -> Value {
        json!({
            "id": self.id,
            "intel_network_id": self.intel_network_id,
            "accessible_type": self.accessible_type,
            "accessible_id": self.accessible_id,
            "permission": self.permission,
            "is_owner": self.is_owner,
            "expires_at": self.expires_at.map(dates::model),
            "created_at": dates::model(self.created_at),
            "updated_at": dates::model(self.updated_at),
        })
    }

    /// NetworkAccessData: plain types, the entity it names.
    pub async fn json(&self, pool: &PgPool) -> sqlx::Result<Value> {
        Ok(json!({
            "id": self.id,
            "accessible_type": models::plain_type(&self.accessible_type),
            "accessible_id": self.accessible_id,
            "permission": self.permission,
            "is_owner": self.is_owner,
            "expires_at": self.expires_at.map(dates::iso8601),
            "entity": entity(pool, &self.accessible_type, self.accessible_id).await?,
        }))
    }
}

/// The rows of a network in the order MySQL's unique index returned them:
/// by the PHP class name (Alliance, Corporation, User), then the id.
pub async fn of_network(pool: &PgPool, network_id: i64) -> sqlx::Result<Vec<Access>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select {COLUMNS} from network_access where intel_network_id = $1
         order by accessible_type collate \"C\", accessible_id"
    )))
    .bind(network_id)
    .fetch_all(pool)
    .await
}

pub async fn dtos(pool: &PgPool, rows: &[Access]) -> sqlx::Result<Vec<Value>> {
    let mut dtos = Vec::with_capacity(rows.len());
    for row in rows {
        dtos.push(row.json(pool).await?);
    }
    Ok(dtos)
}

/// A character with its corporation and alliance, as EntityData and the
/// search show them.
#[derive(sqlx::FromRow)]
pub struct CharacterRow {
    pub id: i64,
    pub name: String,
    pub corporation_id: Option<i64>,
    pub corporation_name: Option<String>,
    pub corporation_ticker: Option<String>,
    pub alliance_id: Option<i64>,
    pub alliance_name: Option<String>,
    pub alliance_ticker: Option<String>,
}

/// A corporation with its alliance.
#[derive(sqlx::FromRow)]
pub struct CorporationRow {
    pub id: i64,
    pub name: String,
    pub ticker: String,
    pub alliance_id: Option<i64>,
    pub alliance_name: Option<String>,
    pub alliance_ticker: Option<String>,
}

/// The select of `CharacterRow`, from `users u`.
pub const CHARACTER_SELECT: &str = "select u.character_id as id, u.character_name as name,
    c.id as corporation_id, c.name as corporation_name, c.ticker as corporation_ticker,
    a.id as alliance_id, a.name as alliance_name, a.ticker as alliance_ticker
    from users u
    left join corporations c on c.id = u.corporation_id
    left join alliances a on a.id = u.alliance_id";

/// The select of `CorporationRow`, from `corporations c`.
pub const CORPORATION_SELECT: &str = "select c.id, c.name, c.ticker,
    a.id as alliance_id, a.name as alliance_name, a.ticker as alliance_ticker
    from corporations c left join alliances a on a.id = c.alliance_id";

impl CharacterRow {
    pub fn corporation(&self) -> Value {
        models::affiliation(
            self.corporation_id,
            self.corporation_name.clone(),
            self.corporation_ticker.clone(),
        )
    }

    pub fn alliance(&self) -> Value {
        models::affiliation(
            self.alliance_id,
            self.alliance_name.clone(),
            self.alliance_ticker.clone(),
        )
    }
}

impl CorporationRow {
    pub fn alliance(&self) -> Value {
        models::affiliation(
            self.alliance_id,
            self.alliance_name.clone(),
            self.alliance_ticker.clone(),
        )
    }
}

/// EntityData: the character, corporation or alliance an access row
/// names, `null` when it is not stored.
pub async fn entity(pool: &PgPool, class: &str, id: i64) -> sqlx::Result<Value> {
    match class {
        USER_CLASS => {
            let row: Option<CharacterRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
                "{CHARACTER_SELECT} where u.character_id = $1"
            )))
            .bind(id)
            .fetch_optional(pool)
            .await?;
            Ok(row.map_or(Value::Null, |row| {
                json!({
                    "id": row.id,
                    "name": row.name,
                    "type": "character",
                    "ticker": null,
                    "corporation": row.corporation(),
                    "alliance": row.alliance(),
                })
            }))
        }
        CORPORATION_CLASS => {
            let row: Option<CorporationRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
                "{CORPORATION_SELECT} where c.id = $1"
            )))
            .bind(id)
            .fetch_optional(pool)
            .await?;
            Ok(row.map_or(Value::Null, |row| {
                json!({
                    "id": row.id,
                    "name": row.name,
                    "type": "corporation",
                    "ticker": row.ticker,
                    "corporation": null,
                    "alliance": row.alliance(),
                })
            }))
        }
        ALLIANCE_CLASS => {
            let row: Option<(i64, String, String)> =
                sqlx::query_as("select id, name, ticker from alliances where id = $1")
                    .bind(id)
                    .fetch_optional(pool)
                    .await?;
            Ok(row.map_or(Value::Null, |(id, name, ticker)| {
                json!({
                    "id": id,
                    "name": name,
                    "type": "alliance",
                    "ticker": ticker,
                    "corporation": null,
                    "alliance": null,
                })
            }))
        }
        _ => Ok(Value::Null),
    }
}

/// The `{access}` of a route: it must be one of the network's rows
/// (FIXES.md).
async fn bind(pool: &PgPool, network: &Network, raw: &str) -> Result<Access, Response> {
    let row: Option<Access> = match super::route_id(raw) {
        Some(id) => sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "select {COLUMNS} from network_access where id = $1 and intel_network_id = $2"
        )))
        .bind(id)
        .bind(network.id)
        .fetch_optional(pool)
        .await
        .map_err(errors::server_error)?,
        None => None,
    };
    row.ok_or_else(|| errors::model_not_found("NetworkAccess", raw))
}

async fn find(pool: &PgPool, id: i64) -> sqlx::Result<Access> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select {COLUMNS} from network_access where id = $1"
    )))
    .bind(id)
    .fetch_one(pool)
    .await
}

/// `expires_at` rules: nullable, a date, after now.
fn expiry_rules(validator: &mut Validator<'_>) {
    validator.field("expires_at").nullable().date().after_now();
}

/// The expiry a request sets: `None` to clear it.
fn expiry(input: &Input) -> Option<OffsetDateTime> {
    input.str("expires_at").and_then(crate::dates::parse)
}

pub async fn index(
    State(state): State<AppState>,
    caller: ApiUser,
    Path(network): Path<String>,
) -> Result<Response, Response> {
    let network = Network::bind(&state.pool, &network).await?;
    permission::require(
        &state.pool,
        network.id,
        &caller.user,
        Permission::Manager,
        state.clock.now(),
    )
    .await?;
    let rows = of_network(&state.pool, network.id)
        .await
        .map_err(errors::server_error)?;
    let dtos = dtos(&state.pool, &rows)
        .await
        .map_err(errors::server_error)?;
    Ok(Json(dtos).into_response())
}

pub async fn store(
    State(state): State<AppState>,
    caller: ApiUser,
    Path(network): Path<String>,
    input: Input,
) -> Result<Response, Response> {
    let network = Network::bind(&state.pool, &network).await?;
    let now = state.clock.now();
    permission::require(
        &state.pool,
        network.id,
        &caller.user,
        Permission::Manager,
        now,
    )
    .await?;
    let mut validator = Validator::new(&input, now);
    validator
        .field("accessible_type")
        .required()
        .string()
        .one_of(&["character", "corporation", "alliance"]);
    validator.field("accessible_id").required().integer();
    validator
        .field("accessible_name")
        .required()
        .string()
        .max(255);
    validator
        .field("permission")
        .required()
        .string()
        .one_of(&["viewer", "member", "manager"]);
    expiry_rules(&mut validator);
    validator.finish()?;

    let class = models::morph_class(input.str("accessible_type").unwrap_or_default())
        .expect("a validated type");
    let accessible_id = input
        .get("accessible_id")
        .and_then(integer)
        .expect("a validated id");
    let permission = input.str("permission").expect("a validated permission");
    let existing: Option<Access> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select {COLUMNS} from network_access
         where intel_network_id = $1 and accessible_type = $2 and accessible_id = $3"
    )))
    .bind(network.id)
    .bind(class)
    .bind(accessible_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(errors::server_error)?;
    let expires_at = expiry(&input);
    let id = match existing {
        // FIXES.md: a grant may not change the owner's row.
        Some(row) if row.is_owner => {
            return Ok(errors::json_error(
                StatusCode::FORBIDDEN,
                "Cannot modify owner access",
            ));
        }
        Some(row) => {
            sqlx::query(
                "update network_access set permission = $2, expires_at = $3,
                    updated_at = case when (permission, expires_at) is distinct from ($2, $3)
                        then $4 else updated_at end
                 where id = $1",
            )
            .bind(row.id)
            .bind(permission)
            .bind(expires_at)
            .bind(now)
            .execute(&state.pool)
            .await
            .map_err(errors::server_error)?;
            row.id
        }
        None => sqlx::query_scalar(
            "insert into network_access (intel_network_id, accessible_type, accessible_id,
                permission, expires_at, created_at, updated_at)
             values ($1, $2, $3, $4, $5, $6, $6) returning id",
        )
        .bind(network.id)
        .bind(class)
        .bind(accessible_id)
        .bind(permission)
        .bind(expires_at)
        .bind(now)
        .fetch_one(&state.pool)
        .await
        .map_err(errors::server_error)?,
    };
    let row = find(&state.pool, id).await.map_err(errors::server_error)?;
    let dto = row.json(&state.pool).await.map_err(errors::server_error)?;
    Ok((StatusCode::CREATED, Json(dto)).into_response())
}

pub async fn update(
    State(state): State<AppState>,
    caller: ApiUser,
    Path((network, access)): Path<(String, String)>,
    input: Input,
) -> Result<Response, Response> {
    let network = Network::bind(&state.pool, &network).await?;
    let access = bind(&state.pool, &network, &access).await?;
    let now = state.clock.now();
    permission::require(
        &state.pool,
        network.id,
        &caller.user,
        Permission::Manager,
        now,
    )
    .await?;
    if access.is_owner {
        return Ok(errors::json_error(
            StatusCode::FORBIDDEN,
            "Cannot modify owner access",
        ));
    }
    let mut validator = Validator::new(&input, now);
    validator
        .field("permission")
        .sometimes()
        .string()
        .one_of(&["viewer", "member", "manager"]);
    expiry_rules(&mut validator);
    validator.finish()?;

    let permission = if input.has("permission") {
        input
            .str("permission")
            .expect("a validated permission")
            .to_owned()
    } else {
        access.permission.clone()
    };
    let expires_at = if input.has("expires_at") {
        expiry(&input)
    } else {
        access.expires_at
    };
    sqlx::query(
        "update network_access set permission = $2, expires_at = $3,
            updated_at = case when (permission, expires_at) is distinct from ($2, $3)
                then $4 else updated_at end
         where id = $1",
    )
    .bind(access.id)
    .bind(&permission)
    .bind(expires_at)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(errors::server_error)?;
    let row = find(&state.pool, access.id)
        .await
        .map_err(errors::server_error)?;
    let dto = row.json(&state.pool).await.map_err(errors::server_error)?;
    Ok(Json(dto).into_response())
}

pub async fn destroy(
    State(state): State<AppState>,
    caller: ApiUser,
    Path((network, access)): Path<(String, String)>,
) -> Result<Response, Response> {
    let network = Network::bind(&state.pool, &network).await?;
    let access = bind(&state.pool, &network, &access).await?;
    permission::require(
        &state.pool,
        network.id,
        &caller.user,
        Permission::Manager,
        state.clock.now(),
    )
    .await?;
    if access.is_owner {
        return Ok(errors::json_error(
            StatusCode::FORBIDDEN,
            "Cannot revoke owner access",
        ));
    }
    sqlx::query("delete from network_access where id = $1")
        .bind(access.id)
        .execute(&state.pool)
        .await
        .map_err(errors::server_error)?;
    Ok(Json(json!({ "message": "Access revoked" })).into_response())
}
