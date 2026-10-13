//! Entity search: characters by name, corporations and alliances by name
//! or ticker, up to 15 per category in id order, matched ignoring case
//! and accents (`%` and `_` in the query are wildcards). A category
//! without a match is empty; ESI's search is not asked.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

use crate::auth::tokens::ApiUser;
use crate::input::Input;
use crate::intel::access::{CHARACTER_SELECT, CORPORATION_SELECT, CharacterRow, CorporationRow};
use crate::server::AppState;
use crate::server::errors;
use crate::validation::Validator;

const LIMIT: i64 = 15;
const CATEGORIES: [&str; 3] = ["character", "corporation", "alliance"];

pub async fn search(
    State(state): State<AppState>,
    _caller: ApiUser,
    input: Input,
) -> Result<Response, Response> {
    let mut validator = Validator::new(&input, state.clock.now());
    validator.field("query").required().string().min(2).max(255);
    validator
        .field("category")
        .nullable()
        .string()
        .one_of(&CATEGORIES);
    validator.finish()?;
    let query = input.str("query").expect("a validated query");
    let pattern = format!("%{query}%");
    let categories: Vec<&str> = match input.str("category") {
        Some(category) => vec![category],
        None => CATEGORIES.to_vec(),
    };
    let mut results: Vec<Value> = Vec::new();
    for category in categories {
        let found = match category {
            "character" => characters(&state, &pattern).await,
            "corporation" => corporations(&state, &pattern).await,
            _ => alliances(&state, &pattern).await,
        }
        .map_err(errors::server_error)?;
        results.extend(found);
    }
    Ok(Json(results).into_response())
}

/// `lower(unaccent(column)) like lower(unaccent(pattern))`.
fn matches(column: &str) -> String {
    format!("lower(unaccent({column})) like lower(unaccent($1))")
}

async fn characters(state: &AppState, pattern: &str) -> sqlx::Result<Vec<Value>> {
    let rows: Vec<CharacterRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{CHARACTER_SELECT} where {} order by u.id limit {LIMIT}",
        matches("u.character_name")
    )))
    .bind(pattern)
    .fetch_all(&state.pool)
    .await?;
    Ok(rows
        .iter()
        .map(|row| {
            json!({
                "id": row.id,
                "name": row.name,
                "category": "character",
                "ticker": null,
                "corporation": row.corporation(),
                "alliance": row.alliance(),
            })
        })
        .collect())
}

async fn corporations(state: &AppState, pattern: &str) -> sqlx::Result<Vec<Value>> {
    let rows: Vec<CorporationRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{CORPORATION_SELECT} where {} or {} order by c.id limit {LIMIT}",
        matches("c.name"),
        matches("c.ticker")
    )))
    .bind(pattern)
    .fetch_all(&state.pool)
    .await?;
    Ok(rows
        .iter()
        .map(|row| {
            json!({
                "id": row.id,
                "name": row.name,
                "category": "corporation",
                "ticker": row.ticker,
                "corporation": null,
                "alliance": row.alliance(),
            })
        })
        .collect())
}

async fn alliances(state: &AppState, pattern: &str) -> sqlx::Result<Vec<Value>> {
    let rows: Vec<(i64, String, String)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select id, name, ticker from alliances where {} or {} order by id limit {LIMIT}",
        matches("name"),
        matches("ticker")
    )))
    .bind(pattern)
    .fetch_all(&state.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name, ticker)| {
            json!({
                "id": id,
                "name": name,
                "category": "alliance",
                "ticker": ticker,
                "corporation": null,
                "alliance": null,
            })
        })
        .collect())
}
