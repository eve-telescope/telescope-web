//! Intel networks: the user's networks, a new
//! one with its owner row, a network with its entries and access, a new
//! name, and deletion by the owner.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

use super::permission::{self, Permission};
use super::{Network, access, entries};
use crate::auth::tokens::{ApiUser, random_string};
use crate::dates;
use crate::input::Input;
use crate::server::AppState;
use crate::server::errors;
use crate::slug::slug;
use crate::validation::Validator;

/// The networks the user may see, with their entry counts, by id.
pub async fn index(State(state): State<AppState>, caller: ApiUser) -> Result<Response, Response> {
    let rows: Vec<(Network, i64)> = sqlx::query_as::<
        _,
        (
            i64,
            String,
            String,
            i64,
            time::OffsetDateTime,
            time::OffsetDateTime,
            i64,
        ),
    >(
        "select n.id, n.name, n.slug, n.owner_id, n.created_at, n.updated_at,
            (select count(*) from intel_entries e where e.intel_network_id = n.id)
         from intel_networks n
         where exists (select 1 from network_access a
             where a.intel_network_id = n.id and a.accessible_id = any($1)
               and (a.expires_at is null or a.expires_at > $2))
         order by n.id",
    )
    .bind(caller.user.accessible_ids())
    .bind(state.clock.now())
    .fetch_all(&state.pool)
    .await
    .map_err(errors::server_error)?
    .into_iter()
    .map(
        |(id, name, slug, owner_id, created_at, updated_at, count)| {
            (
                Network {
                    id,
                    name,
                    slug,
                    owner_id,
                    created_at,
                    updated_at,
                },
                count,
            )
        },
    )
    .collect();
    let answer: Vec<Value> = rows
        .into_iter()
        .map(|(network, count)| {
            let mut json = network.json();
            json["entries_count"] = json!(count);
            json
        })
        .collect();
    Ok(Json(answer).into_response())
}

fn name_rules(validator: &mut Validator<'_>) {
    validator.field("name").required().string().max(255);
}

/// A new network, owned by the caller's character with a manager row.
pub async fn store(
    State(state): State<AppState>,
    caller: ApiUser,
    input: Input,
) -> Result<Response, Response> {
    let now = state.clock.now();
    let mut validator = Validator::new(&input, now);
    name_rules(&mut validator);
    validator.finish()?;
    let name = input.str("name").expect("a validated name");
    let slug = format!("{}-{}", slug(name), random_string(6));
    let mut transaction = state.pool.begin().await.map_err(errors::server_error)?;
    let id: i64 = sqlx::query_scalar(
        "insert into intel_networks (name, slug, owner_id, created_at, updated_at)
         values ($1, $2, $3, $4, $4) returning id",
    )
    .bind(name)
    .bind(&slug)
    .bind(caller.user.id)
    .bind(now)
    .fetch_one(&mut *transaction)
    .await
    .map_err(errors::server_error)?;
    sqlx::query(
        "insert into network_access (intel_network_id, accessible_type, accessible_id,
            permission, is_owner, created_at, updated_at)
         values ($1, 'App\\Models\\User', $2, 'manager', true, $3, $3)",
    )
    .bind(id)
    .bind(caller.user.character_id)
    .bind(now)
    .execute(&mut *transaction)
    .await
    .map_err(errors::server_error)?;
    transaction.commit().await.map_err(errors::server_error)?;
    let accesses: Vec<Value> = access::of_network(&state.pool, id)
        .await
        .map_err(errors::server_error)?
        .iter()
        .map(access::Access::raw_json)
        .collect();
    // A new network answers in the order its attributes were set.
    let body = json!({
        "name": name,
        "slug": slug,
        "owner_id": caller.user.id,
        "updated_at": dates::model(now),
        "created_at": dates::model(now),
        "id": id,
        "accesses": accesses,
    });
    Ok((StatusCode::CREATED, Json(body)).into_response())
}

/// A network with its entry count, entries and access rows.
pub async fn show(
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
    let entries = entries::of_network(&state.pool, network.id)
        .await
        .map_err(errors::server_error)?;
    let rows = access::of_network(&state.pool, network.id)
        .await
        .map_err(errors::server_error)?;
    let accesses = access::dtos(&state.pool, &rows)
        .await
        .map_err(errors::server_error)?;
    let mut body = network.json();
    body["entries_count"] = json!(entries.len());
    body["entries"] = Value::Array(entries);
    body["accesses"] = Value::Array(accesses);
    Ok(Json(body).into_response())
}

/// A new name; `updated_at` moves only when it changes.
pub async fn update(
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
    name_rules(&mut validator);
    validator.finish()?;
    let name = input.str("name").expect("a validated name");
    let updated: Network = sqlx::query_as(
        "update intel_networks set name = $2,
            updated_at = case when name is distinct from $2 then $3 else updated_at end
         where id = $1
         returning id, name, slug, owner_id, created_at, updated_at",
    )
    .bind(network.id)
    .bind(name)
    .bind(now)
    .fetch_one(&state.pool)
    .await
    .map_err(errors::server_error)?;
    Ok(Json(updated.json()).into_response())
}

/// Deletes the network with everything in it; only its owner may.
pub async fn destroy(
    State(state): State<AppState>,
    caller: ApiUser,
    Path(network): Path<String>,
) -> Result<Response, Response> {
    let network = Network::bind(&state.pool, &network).await?;
    let owner = permission::is_owner(&state.pool, network.id, &caller.user)
        .await
        .map_err(errors::server_error)?;
    if !owner {
        return Err(errors::forbidden());
    }
    sqlx::query("delete from intel_networks where id = $1")
        .bind(network.id)
        .execute(&state.pool)
        .await
        .map_err(errors::server_error)?;
    Ok(Json(json!({ "message": "Network deleted" })).into_response())
}
