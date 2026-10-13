//! Shared pilot lists (ShareController): `POST /api/share` and `GET
//! /api/share/{code}` for the desktop clients (every version since
//! v0.1.0, no token), and the share page's props, which count a view.
//! Codes are 8 random letters and digits, unique and looked up ignoring
//! case, as MySQL's collation compared them.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::auth::tokens::random_string;
use crate::dates;
use crate::input::Input;
use crate::server::AppState;
use crate::server::errors;
use crate::server::pages::shared_props;
use crate::validation::Validator;

#[derive(Debug, Clone, sqlx::FromRow)]
struct Share {
    code: String,
    pilots: sqlx::types::Json<Vec<String>>,
    pilot_count: i32,
    views: i64,
    created_at: OffsetDateTime,
}

fn not_found() -> Response {
    errors::http_error(
        axum::http::StatusCode::NOT_FOUND,
        "No query results for model [App\\Models\\Share].",
    )
}

/// The pilots a share keeps: trimmed (PHP's `trim`), without empty names
/// or "0" (`array_filter`), each name once with case (`array_unique`), in
/// the order sent.
pub fn clean_pilots(pilots: &[Value]) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    for pilot in pilots {
        let Some(name) = pilot.as_str() else { continue };
        let name = name.trim_matches([' ', '\t', '\n', '\r', '\0', '\u{0B}']);
        if name.is_empty() || name == "0" || kept.iter().any(|seen| seen == name) {
            continue;
        }
        kept.push(name.to_owned());
    }
    kept
}

/// A random code no share has yet, ignoring case.
async fn unique_code(pool: &PgPool) -> sqlx::Result<String> {
    loop {
        let code = random_string(8);
        let taken: bool = sqlx::query_scalar(
            "select exists (select 1 from shares where lower(code) = lower($1))",
        )
        .bind(&code)
        .fetch_one(pool)
        .await?;
        if !taken {
            return Ok(code);
        }
    }
}

pub async fn store(State(state): State<AppState>, input: Input) -> Result<Response, Response> {
    let now = state.clock.now();
    let mut validator = Validator::new(&input, now);
    validator.field("pilots").required().array().min(1).max(500);
    validator.each("pilots", |pilot| {
        pilot.required().string().max(100);
    });
    validator.finish()?;
    let pilots = match input.get("pilots") {
        Some(Value::Array(items)) => clean_pilots(items),
        Some(Value::Object(items)) => clean_pilots(&items.values().cloned().collect::<Vec<_>>()),
        _ => Vec::new(),
    };
    let code = unique_code(&state.pool)
        .await
        .map_err(errors::server_error)?;
    sqlx::query(
        "insert into shares (code, pilots, pilot_count, created_at, updated_at)
         values ($1, $2, $3, $4, $4)",
    )
    .bind(&code)
    .bind(sqlx::types::Json(&pilots))
    .bind(pilots.len() as i32)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(errors::server_error)?;
    let url = state.config.url(&format!("/s/{code}"));
    Ok(Json(json!({ "code": code, "url": url })).into_response())
}

pub async fn fetch(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Response, Response> {
    let share: Option<Share> = sqlx::query_as(
        "select code, pilots, pilot_count, views, created_at from shares
         where lower(code) = lower($1)",
    )
    .bind(&code)
    .fetch_optional(&state.pool)
    .await
    .map_err(errors::server_error)?;
    let share = share.ok_or_else(not_found)?;
    Ok(Json(json!({
        "code": share.code,
        "pilots": share.pilots.0,
        "pilotCount": share.pilot_count,
    }))
    .into_response())
}

/// The share page's props; every load counts a view (and moves
/// `updated_at`), and the page shows the new count.
pub async fn page(
    State(state): State<AppState>,
    Path(code): Path<String>,
    headers: HeaderMap,
) -> Result<Response, Response> {
    let now = state.clock.now();
    let share: Option<Share> = sqlx::query_as(
        "update shares set views = views + 1, updated_at = $2
         where id = (select id from shares where lower(code) = lower($1))
         returning code, pilots, pilot_count, views, created_at",
    )
    .bind(&code)
    .bind(now)
    .fetch_optional(&state.pool)
    .await
    .map_err(errors::server_error)?;
    let share = share.ok_or_else(not_found)?;
    let mut props = shared_props(&state, &headers).await?;
    props["share"] = json!({
        "code": share.code,
        "pilots": share.pilots.0,
        "pilotCount": share.pilot_count,
        "views": share.views,
        "createdAt": dates::model(share.created_at),
    });
    Ok(Json(props).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_the_pilots_like_php() {
        let pilots = vec![
            json!("Pilot One"),
            json!(" Pilot Two "),
            json!("Pilot One"),
            json!("0"),
            json!("pilot one"),
            json!("\u{a0}Spaced\u{a0}"),
        ];
        assert_eq!(
            clean_pilots(&pilots),
            ["Pilot One", "Pilot Two", "pilot one", "\u{a0}Spaced\u{a0}"]
        );
    }
}
