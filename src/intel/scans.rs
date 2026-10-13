//! Shared scans (NetworkScanController, NetworkScanData): local and
//! d-scans shared with a network, its history paged like Laravel's
//! LengthAwarePaginator, newest first.

use axum::Json;
use axum::extract::{OriginalUri, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use time::OffsetDateTime;

use super::Network;
use super::permission::{self, Permission};
use crate::auth::tokens::ApiUser;
use crate::dates;
use crate::input::Input;
use crate::pusher::events::Event;
use crate::server::AppState;
use crate::server::errors;
use crate::validation::{Validator, integer};

const PER_PAGE: i64 = 20;
/// Pages on each side of the current one in the links (`onEachSide`).
const ON_EACH_SIDE: i64 = 3;

#[derive(Debug, Clone, sqlx::FromRow)]
struct Scan {
    id: i64,
    intel_network_id: i64,
    scan_type: String,
    raw_text: String,
    solar_system: Option<String>,
    created_at: OffsetDateTime,
    character_id: Option<i64>,
    character_name: Option<String>,
}

const SELECT: &str = "select s.id, s.intel_network_id, s.scan_type, s.raw_text, s.solar_system,
    s.created_at, u.character_id, u.character_name
    from network_scans s left join users u on u.id = s.user_id";

impl Scan {
    /// NetworkScanData.
    fn json(&self) -> Value {
        let submitted_by = match (self.character_id, &self.character_name) {
            (Some(id), Some(name)) => json!({ "id": id, "character_name": name }),
            _ => Value::Null,
        };
        json!({
            "id": self.id,
            "scan_type": self.scan_type,
            "raw_text": self.raw_text,
            "solar_system": self.solar_system,
            "created_at": dates::iso8601(self.created_at),
            "submitted_by": submitted_by,
        })
    }
}

/// The page a request asks for: a whole number from 1, else the first.
fn current_page(input: &Input) -> i64 {
    input
        .get("page")
        .and_then(integer)
        .filter(|page| *page >= 1)
        .unwrap_or(1)
}

/// The pages the links show, Laravel's UrlWindow: all of them when there
/// are fewer than 14, else the start, a slider around the current page
/// and the end, with `...` between.
fn window(current: i64, last: i64) -> Vec<Option<i64>> {
    if last < ON_EACH_SIDE * 2 + 8 {
        return (1..=last).map(Some).collect();
    }
    let window = ON_EACH_SIDE + 4;
    let (first, slider, end): (Vec<i64>, Option<Vec<i64>>, Vec<i64>) = if current <= window {
        (
            (1..=window + ON_EACH_SIDE).collect(),
            None,
            vec![last - 1, last],
        )
    } else if current > last - window {
        (
            vec![1, 2],
            None,
            (last - (window + (ON_EACH_SIDE - 1))..=last).collect(),
        )
    } else {
        (
            vec![1, 2],
            Some((current - ON_EACH_SIDE..=current + ON_EACH_SIDE).collect()),
            vec![last - 1, last],
        )
    };
    let mut pages: Vec<Option<i64>> = first.into_iter().map(Some).collect();
    if let Some(slider) = slider {
        pages.push(None);
        pages.extend(slider.into_iter().map(Some));
    }
    pages.push(None);
    pages.extend(end.into_iter().map(Some));
    pages
}

/// The paginator's JSON around a page of items.
fn paginate(path: &str, current: i64, total: i64, items: Vec<Value>) -> Value {
    let last = ((total + PER_PAGE - 1) / PER_PAGE).max(1);
    let url = |page: i64| format!("{path}?page={}", page.max(1));
    let count = items.len() as i64;
    let from = (count > 0).then(|| (current - 1) * PER_PAGE + 1);
    let to = from.map(|from| from + count - 1);
    let previous = (current > 1).then(|| url(current - 1));
    let next = (current < last).then(|| url(current + 1));
    let mut links = vec![json!({
        "url": previous,
        "label": "&laquo; Previous",
        "page": (current > 1).then_some(current - 1),
        "active": false,
    })];
    for page in window(current, last) {
        links.push(match page {
            Some(page) => json!({
                "url": url(page),
                "label": page.to_string(),
                "page": page,
                "active": page == current,
            }),
            None => json!({ "url": null, "label": "...", "active": false }),
        });
    }
    links.push(json!({
        "url": next,
        "label": "Next &raquo;",
        "page": (current < last).then_some(current + 1),
        "active": false,
    }));
    json!({
        "current_page": current,
        "data": items,
        "first_page_url": url(1),
        "from": from,
        "last_page": last,
        "last_page_url": url(last),
        "links": links,
        "next_page_url": next,
        "path": path,
        "per_page": PER_PAGE,
        "prev_page_url": previous,
        "to": to,
        "total": total,
    })
}

pub async fn index(
    State(state): State<AppState>,
    caller: ApiUser,
    Path(network): Path<String>,
    OriginalUri(uri): OriginalUri,
    input: Input,
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
    let current = current_page(&input);
    let total: i64 =
        sqlx::query_scalar("select count(*) from network_scans where intel_network_id = $1")
            .bind(network.id)
            .fetch_one(&state.pool)
            .await
            .map_err(errors::server_error)?;
    let scans: Vec<Scan> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SELECT} where s.intel_network_id = $1
         order by s.created_at desc, s.id desc limit $2 offset $3"
    )))
    .bind(network.id)
    .bind(PER_PAGE)
    .bind((current - 1).saturating_mul(PER_PAGE))
    .fetch_all(&state.pool)
    .await
    .map_err(errors::server_error)?;
    let path = state.config.url(uri.path().trim_end_matches('/'));
    let items = scans.iter().map(Scan::json).collect();
    Ok(Json(paginate(&path, current, total, items)).into_response())
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
    validator
        .field("scan_type")
        .required()
        .string()
        .one_of(&["local", "dscan"]);
    validator.field("raw_text").required().string().max(500_000);
    validator.field("solar_system").nullable().string().max(255);
    validator.finish()?;
    let id: i64 = sqlx::query_scalar(
        "insert into network_scans (intel_network_id, user_id, scan_type, raw_text, solar_system,
            created_at, updated_at)
         values ($1, $2, $3, $4, $5, $6, $6) returning id",
    )
    .bind(network.id)
    .bind(caller.user.id)
    .bind(input.str("scan_type"))
    .bind(input.str("raw_text"))
    .bind(input.str("solar_system"))
    .bind(now)
    .fetch_one(&state.pool)
    .await
    .map_err(errors::server_error)?;
    state.pusher.broadcast(Event::ScanShared {
        network_id: network.id,
        scan_id: id,
    });
    let scan: Scan = sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} where s.id = $1")))
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(errors::server_error)?;
    Ok((StatusCode::CREATED, Json(scan.json())).into_response())
}

/// One scan: it must exist, the network be visible, and the scan be the
/// network's (404 with an empty message, as `abort_unless` answered).
pub async fn show(
    State(state): State<AppState>,
    caller: ApiUser,
    Path((network, scan)): Path<(String, String)>,
) -> Result<Response, Response> {
    let network = Network::bind(&state.pool, &network).await?;
    let found: Option<Scan> = match super::route_id(&scan) {
        Some(id) => sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} where s.id = $1")))
            .bind(id)
            .fetch_optional(&state.pool)
            .await
            .map_err(errors::server_error)?,
        None => None,
    };
    let scan = found.ok_or_else(|| errors::model_not_found("NetworkScan", &scan))?;
    permission::require(
        &state.pool,
        network.id,
        &caller.user,
        Permission::Viewer,
        state.clock.now(),
    )
    .await?;
    if scan.intel_network_id != network.id {
        return Err(errors::http_error(StatusCode::NOT_FOUND, ""));
    }
    Ok(Json(scan.json()).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pages(current: i64, last: i64) -> String {
        window(current, last)
            .into_iter()
            .map(|page| page.map_or("...".to_owned(), |page| page.to_string()))
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn windows_the_links_like_laravel() {
        assert_eq!(pages(1, 1), "1");
        assert_eq!(pages(5, 13), "1 2 3 4 5 6 7 8 9 10 11 12 13");
        assert_eq!(pages(1, 93), "1 2 3 4 5 6 7 8 9 10 ... 92 93");
        assert_eq!(pages(7, 93), "1 2 3 4 5 6 7 8 9 10 ... 92 93");
        assert_eq!(pages(8, 93), "1 2 ... 5 6 7 8 9 10 11 ... 92 93");
        assert_eq!(pages(86, 93), "1 2 ... 83 84 85 86 87 88 89 ... 92 93");
        assert_eq!(pages(87, 93), "1 2 ... 84 85 86 87 88 89 90 91 92 93");
    }
}
