//! Intel entries (IntelEntryController): a character, corporation or
//! alliance tagged in a network with a color, a label and notes, one
//! entry per entity and network. Every change is broadcast on the
//! network's channel.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use regex::Regex;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::LazyLock;
use time::OffsetDateTime;

use super::Network;
use super::permission::{self, Permission};
use crate::auth::tokens::ApiUser;
use crate::dates;
use crate::input::Input;
use crate::models::User;
use crate::pusher::events::Event;
use crate::server::AppState;
use crate::server::errors;
use crate::validation::{Validator, integer};

static COLOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^#[0-9A-Fa-f]{6}$").expect("the color pattern"));

/// One row of the bulk lookup.
#[derive(sqlx::FromRow)]
struct LookupRow {
    id: i64,
    intel_network_id: i64,
    network_name: String,
    entity_type: String,
    entity_id: i64,
    entity_name: String,
    color: Option<String>,
    label: Option<String>,
    notes: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Entry {
    pub id: i64,
    pub intel_network_id: i64,
    pub entity_type: String,
    pub entity_id: i64,
    pub entity_name: String,
    pub color: Option<String>,
    pub label: Option<String>,
    pub notes: Option<String>,
    pub added_by: i64,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

const COLUMNS: &str = "id, intel_network_id, entity_type, entity_id, entity_name, color, label, notes, added_by, created_at, updated_at";

impl Entry {
    /// A stored entry with `added_by` replaced by the user who added it
    /// (`with('addedBy')`), columns in order.
    pub fn json(&self, added_by: Option<&User>) -> Value {
        json!({
            "id": self.id,
            "intel_network_id": self.intel_network_id,
            "entity_type": self.entity_type,
            "entity_id": self.entity_id,
            "entity_name": self.entity_name,
            "color": self.color,
            "label": self.label,
            "notes": self.notes,
            "added_by": added_by.map_or(Value::Null, User::json),
            "created_at": dates::model(self.created_at),
            "updated_at": dates::model(self.updated_at),
        })
    }

    /// A new entry, in the order its attributes were set.
    fn json_created(&self, added_by: &User) -> Value {
        json!({
            "entity_type": self.entity_type,
            "entity_id": self.entity_id,
            "entity_name": self.entity_name,
            "color": self.color,
            "label": self.label,
            "notes": self.notes,
            "added_by": added_by.json(),
            "intel_network_id": self.intel_network_id,
            "updated_at": dates::model(self.updated_at),
            "created_at": dates::model(self.created_at),
            "id": self.id,
        })
    }
}

async fn find(pool: &PgPool, id: i64) -> sqlx::Result<Entry> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select {COLUMNS} from intel_entries where id = $1"
    )))
    .bind(id)
    .fetch_one(pool)
    .await
}

/// A network's entries with their users, in the order MySQL's unique
/// index returned them: by entity type, then entity id.
pub async fn of_network(pool: &PgPool, network_id: i64) -> sqlx::Result<Vec<Value>> {
    let entries: Vec<Entry> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select {COLUMNS} from intel_entries where intel_network_id = $1
         order by entity_type collate \"C\", entity_id"
    )))
    .bind(network_id)
    .fetch_all(pool)
    .await?;
    let mut answer = Vec::with_capacity(entries.len());
    for entry in entries {
        let user = User::find(pool, entry.added_by).await?;
        answer.push(entry.json(user.as_ref()));
    }
    Ok(answer)
}

/// The `{entry}` of a route: it must be one of the network's entries
/// (FIXES.md).
async fn bind(pool: &PgPool, network: &Network, raw: &str) -> Result<Entry, Response> {
    let entry: Option<Entry> = match super::route_id(raw) {
        Some(id) => sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "select {COLUMNS} from intel_entries where id = $1 and intel_network_id = $2"
        )))
        .bind(id)
        .bind(network.id)
        .fetch_optional(pool)
        .await
        .map_err(errors::server_error)?,
        None => None,
    };
    entry.ok_or_else(|| errors::model_not_found("IntelEntry", raw))
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
        Permission::Viewer,
        state.clock.now(),
    )
    .await?;
    let entries = of_network(&state.pool, network.id)
        .await
        .map_err(errors::server_error)?;
    Ok(Json(entries).into_response())
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
        Permission::Member,
        now,
    )
    .await?;
    let mut validator = Validator::new(&input, now);
    validator.field("entity_type").required().string().one_of(&[
        "character",
        "corporation",
        "alliance",
    ]);
    validator.field("entity_id").required().integer();
    validator.field("entity_name").required().string().max(255);
    validator.field("color").nullable().string().regex(&COLOR);
    validator.field("label").nullable().string().max(255);
    validator.field("notes").nullable().string().max(10_000);
    validator.finish()?;

    let entity_type = input.str("entity_type").expect("a validated type");
    let entity_id = input
        .get("entity_id")
        .and_then(integer)
        .expect("a validated id");
    let entity_name = input.str("entity_name").expect("a validated name");
    let (color, label, notes) = (input.str("color"), input.str("label"), input.str("notes"));
    let existing: Option<i64> = sqlx::query_scalar(
        "select id from intel_entries
         where intel_network_id = $1 and entity_type = $2 and entity_id = $3",
    )
    .bind(network.id)
    .bind(entity_type)
    .bind(entity_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(errors::server_error)?;
    let created = existing.is_none();
    let id = match existing {
        Some(id) => {
            sqlx::query(
                "update intel_entries set entity_name = $2, color = $3, label = $4, notes = $5,
                    added_by = $6,
                    updated_at = case when (entity_name, color, label, notes, added_by)
                        is distinct from ($2, $3, $4, $5, $6) then $7 else updated_at end
                 where id = $1",
            )
            .bind(id)
            .bind(entity_name)
            .bind(color)
            .bind(label)
            .bind(notes)
            .bind(caller.user.id)
            .bind(now)
            .execute(&state.pool)
            .await
            .map_err(errors::server_error)?;
            id
        }
        None => sqlx::query_scalar(
            "insert into intel_entries (intel_network_id, entity_type, entity_id, entity_name,
                color, label, notes, added_by, created_at, updated_at)
             values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9) returning id",
        )
        .bind(network.id)
        .bind(entity_type)
        .bind(entity_id)
        .bind(entity_name)
        .bind(color)
        .bind(label)
        .bind(notes)
        .bind(caller.user.id)
        .bind(now)
        .fetch_one(&state.pool)
        .await
        .map_err(errors::server_error)?,
    };
    let entry = find(&state.pool, id).await.map_err(errors::server_error)?;
    state.pusher.broadcast(Event::IntelEntryCreated {
        network_id: network.id,
        entry_id: id,
    });
    let body = if created {
        entry.json_created(&caller.user)
    } else {
        entry.json(Some(&caller.user))
    };
    Ok((StatusCode::CREATED, Json(body)).into_response())
}

pub async fn update(
    State(state): State<AppState>,
    caller: ApiUser,
    Path((network, entry)): Path<(String, String)>,
    input: Input,
) -> Result<Response, Response> {
    let network = Network::bind(&state.pool, &network).await?;
    let entry = bind(&state.pool, &network, &entry).await?;
    let now = state.clock.now();
    permission::require(
        &state.pool,
        network.id,
        &caller.user,
        Permission::Member,
        now,
    )
    .await?;
    let mut validator = Validator::new(&input, now);
    // FIXES.md: the color and the label may be cleared.
    validator
        .field("color")
        .sometimes()
        .nullable()
        .string()
        .regex(&COLOR);
    validator
        .field("label")
        .sometimes()
        .nullable()
        .string()
        .max(255);
    validator.field("entity_name").sometimes().string().max(255);
    validator.field("notes").nullable().string().max(10_000);
    validator.finish()?;

    let pick = |key: &str, current: Option<String>| -> Option<String> {
        if input.has(key) {
            input.str(key).map(str::to_owned)
        } else {
            current
        }
    };
    let color = pick("color", entry.color.clone());
    let label = pick("label", entry.label.clone());
    let notes = pick("notes", entry.notes.clone());
    let entity_name = pick("entity_name", Some(entry.entity_name.clone()))
        .unwrap_or_else(|| entry.entity_name.clone());
    sqlx::query(
        "update intel_entries set entity_name = $2, color = $3, label = $4, notes = $5,
            updated_at = case when (entity_name, color, label, notes)
                is distinct from ($2, $3, $4, $5) then $6 else updated_at end
         where id = $1",
    )
    .bind(entry.id)
    .bind(&entity_name)
    .bind(&color)
    .bind(&label)
    .bind(&notes)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(errors::server_error)?;
    let entry = find(&state.pool, entry.id)
        .await
        .map_err(errors::server_error)?;
    let user = User::find(&state.pool, entry.added_by)
        .await
        .map_err(errors::server_error)?;
    state.pusher.broadcast(Event::IntelEntryUpdated {
        network_id: network.id,
        entry_id: entry.id,
    });
    Ok(Json(entry.json(user.as_ref())).into_response())
}

pub async fn destroy(
    State(state): State<AppState>,
    caller: ApiUser,
    Path((network, entry)): Path<(String, String)>,
) -> Result<Response, Response> {
    let network = Network::bind(&state.pool, &network).await?;
    let entry = bind(&state.pool, &network, &entry).await?;
    permission::require(
        &state.pool,
        network.id,
        &caller.user,
        Permission::Member,
        state.clock.now(),
    )
    .await?;
    sqlx::query("delete from intel_entries where id = $1")
        .bind(entry.id)
        .execute(&state.pool)
        .await
        .map_err(errors::server_error)?;
    state.pusher.broadcast(Event::IntelEntryDeleted {
        network_id: network.id,
        entry_id: entry.id,
    });
    Ok(Json(json!({ "message": "Entry deleted" })).into_response())
}

/// `GET /api/intel/lookup?entity_ids[]=...`: the entries of these
/// entities in every network the user may see, by id.
pub async fn lookup(
    State(state): State<AppState>,
    caller: ApiUser,
    input: Input,
) -> Result<Response, Response> {
    let now = state.clock.now();
    let mut validator = Validator::new(&input, now);
    validator.field("entity_ids").required().array();
    validator.each("entity_ids", |item| {
        item.integer();
    });
    validator.finish()?;
    let ids: Vec<i64> = match input.get("entity_ids") {
        Some(Value::Array(items)) => items.iter().filter_map(integer).collect(),
        Some(Value::Object(items)) => items.values().filter_map(integer).collect(),
        _ => Vec::new(),
    };
    let rows: Vec<LookupRow> = sqlx::query_as(
        "select e.id, e.intel_network_id, n.name as network_name, e.entity_type, e.entity_id,
            e.entity_name, e.color, e.label, e.notes
         from intel_entries e join intel_networks n on n.id = e.intel_network_id
         where e.entity_id = any($1)
           and exists (select 1 from network_access a
               where a.intel_network_id = e.intel_network_id and a.accessible_id = any($2)
                 and (a.expires_at is null or a.expires_at > $3))
         order by e.id",
    )
    .bind(&ids)
    .bind(caller.user.accessible_ids())
    .bind(now)
    .fetch_all(&state.pool)
    .await
    .map_err(errors::server_error)?;
    let answer: Vec<Value> = rows
        .into_iter()
        .map(|row| {
            json!({
                "id": row.id,
                "intel_network_id": row.intel_network_id,
                "network_name": row.network_name,
                "entity_type": row.entity_type,
                "entity_id": row.entity_id,
                "entity_name": row.entity_name,
                "color": row.color,
                "label": row.label,
                "notes": row.notes,
            })
        })
        .collect();
    Ok(Json(answer).into_response())
}
