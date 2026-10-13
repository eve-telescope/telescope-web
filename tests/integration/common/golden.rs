//! The golden fixtures recorded from the Laravel reference
//! (scripts/laravel-reference/record.py and pusher.mjs), the dataset they
//! start from (tests/fixtures/dataset.json) and the normalizations every
//! comparison applies. The only accepted differences are listed here and
//! in FIXES.md.
//!
//! - N1: page props are compared without Inertia's shared `errors` bag,
//!   and without `demoLoginEnabled`, which the home page adds (FIXES.md,
//!   deliberate changes).
//! - N2: the recorder rewrote the reference's own origin
//!   (`http://127.0.0.1:8318`) to APP_URL, where Laravel built URLs from
//!   the request's host.
//! - N3: a timestamp written while the scenario ran (between its start and
//!   end) is `<now>`; the replay's clock stands at the scenario's start, so
//!   its own timestamps are `<now>` too.
//! - N4: what the recording made up at random: the six characters at the
//!   end of a new network's slug, a new share's code (and its link), a new
//!   token's secret and stored hash, the login's `state`, and the socket
//!   ids of the websocket frames.
//! - N5: the EVE SSO address and client id of `/eve`'s redirect (the mock
//!   and the test application, the real one and the reference's).
//! - N6: JSON is compared parsed (Laravel escaped `/` and pretty-printed
//!   exceptions, FIXES.md), keys in order, except the row snapshots, whose
//!   keys MySQL's `JSON_OBJECT` sorted.
//! - N7: HTML error pages are compared by status and title; Laravel's 405
//!   page was Symfony's fallback, ours is the minimal page of every other
//!   status.
//! - N8: the page props come from `/api/pages/*`, under the API's CORS;
//!   the pages themselves (SvelteKit) answer without CORS headers.

use serde_json::{Map, Value};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;

use super::{TestApp, repo_path};

pub const NOW: &str = "<now>";

pub fn read_json(relative: &str) -> Value {
    let path = repo_path(&format!("tests/fixtures/{relative}"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The names of the fixtures in a directory of tests/fixtures/golden.
pub fn fixture_names(directory: &str) -> Vec<String> {
    let mut names: Vec<String> =
        std::fs::read_dir(repo_path(&format!("tests/fixtures/golden/{directory}")))
            .expect("fixtures directory")
            .filter_map(|entry| {
                let name = entry.ok()?.file_name().into_string().ok()?;
                name.strip_suffix(".json").map(str::to_owned)
            })
            .collect();
    names.sort();
    names
}

/// The tables of the dataset, in the order their rows can be inserted.
pub const DATASET_TABLES: [&str; 9] = [
    "alliances",
    "corporations",
    "users",
    "personal_access_tokens",
    "intel_networks",
    "network_access",
    "intel_entries",
    "network_scans",
    "shares",
];

/// The plain text token of a dataset token id (`_plain`).
pub fn dataset_token(id: i64) -> String {
    let dataset = read_json("dataset.json");
    dataset["personal_access_tokens"]
        .as_array()
        .expect("tokens")
        .iter()
        .find(|token| token["id"] == id)
        .and_then(|token| token["_plain"].as_str())
        .unwrap_or_else(|| panic!("no dataset token {id}"))
        .to_owned()
}

/// A dataset timestamp (`2026-10-01 12:00:00`, UTC) as Postgres reads it.
fn resolve(value: &Value) -> Value {
    match value {
        Value::String(text)
            if text.len() == 19 && text.as_bytes()[10] == b' ' && text.as_bytes()[4] == b'-' =>
        {
            Value::String(format!("{text}+00"))
        }
        other => other.clone(),
    }
}

impl TestApp {
    /// A fresh app with tests/fixtures/dataset.json seeded, its clock at
    /// `now`.
    pub async fn with_dataset(now: OffsetDateTime) -> Self {
        let app = Self::at(now).await;
        app.seed_dataset().await;
        app
    }

    pub async fn seed_dataset(&self) {
        let dataset = read_json("dataset.json");
        for table in DATASET_TABLES {
            for row in dataset[table].as_array().expect("rows") {
                let row: Map<String, Value> = row
                    .as_object()
                    .expect("a row")
                    .iter()
                    .filter(|(key, _)| !key.starts_with('_'))
                    .map(|(key, value)| (key.clone(), resolve(value)))
                    .collect();
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "insert into {table} select * from jsonb_populate_record(null::{table}, $1)"
                )))
                .bind(Value::Object(row))
                .execute(&self.pool)
                .await
                .unwrap_or_else(|error| panic!("{table}: {error}"));
            }
            if !["alliances", "corporations"].contains(&table) {
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "select setval(pg_get_serial_sequence('{table}', 'id'), (select max(id) from {table}))"
                )))
                .execute(&self.pool)
                .await
                .expect("sequence resets");
            }
        }
    }

    /// The rows of a table as the recorder wrote them: every column, MySQL's
    /// `Y-m-d H:i:s` timestamps, by id.
    pub async fn snapshot(&self, table: &str) -> Vec<Value> {
        let rows: Vec<Value> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "select to_jsonb(t) from {table} t order by id"
        )))
        .fetch_all(&self.pool)
        .await
        .expect("rows read");
        rows.into_iter()
            .map(|row| {
                Value::Object(
                    row.as_object()
                        .expect("a row")
                        .iter()
                        .map(|(key, value)| {
                            let value = match value {
                                Value::String(text) if is_timestamp_column(key) => {
                                    Value::String(mysql_timestamp(text))
                                }
                                other => other.clone(),
                            };
                            (key.clone(), value)
                        })
                        .collect(),
                )
            })
            .collect()
    }
}

fn is_timestamp_column(column: &str) -> bool {
    [
        "last_updated",
        "last_active_at",
        "last_used_at",
        "expires_at",
        "created_at",
        "updated_at",
    ]
    .contains(&column)
}

/// `2026-10-13T08:30:00+00:00` (Postgres' JSON) as `2026-10-13 08:30:00`.
fn mysql_timestamp(text: &str) -> String {
    let moment = OffsetDateTime::parse(text, &Rfc3339)
        .unwrap_or_else(|error| panic!("{text}: {error}"))
        .to_offset(time::UtcOffset::UTC);
    moment
        .format(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second]"
        ))
        .expect("a timestamp")
}

/// The moments a timestamp string can stand for, in the three forms the
/// API and the snapshots write: `.000000Z`, `+00:00` and MySQL's.
fn parse_any_timestamp(text: &str) -> Option<OffsetDateTime> {
    if let Ok(moment) = OffsetDateTime::parse(text, &Rfc3339) {
        return Some(moment);
    }
    time::PrimitiveDateTime::parse(
        text,
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second]"),
    )
    .ok()
    .map(time::PrimitiveDateTime::assume_utc)
}

/// N3: every timestamp within `[from, to]` (unix seconds) becomes `<now>`.
pub fn normalize_now(value: &Value, from: i64, to: i64) -> Value {
    match value {
        Value::String(text) => match parse_any_timestamp(text) {
            Some(moment) if text.len() >= 19 && (from..=to).contains(&moment.unix_timestamp()) => {
                Value::String(NOW.to_owned())
            }
            _ => value.clone(),
        },
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| normalize_now(item, from, to))
                .collect(),
        ),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, item)| (key.clone(), normalize_now(item, from, to)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// N4: a slug's random suffix.
pub fn normalize_slug(slug: &str) -> String {
    match slug.rsplit_once('-') {
        Some((base, suffix)) if suffix.len() == 6 => format!("{base}-<random>"),
        _ => slug.to_owned(),
    }
}

/// N4 over a JSON value: `slug` fields that are new, and the named random
/// strings replaced wherever they appear.
pub fn normalize_random(value: &Value, randoms: &[(String, &str)]) -> Value {
    match value {
        Value::String(text) => {
            let mut text = text.clone();
            for (random, placeholder) in randoms {
                if !random.is_empty() {
                    text = text.replace(random.as_str(), placeholder);
                }
            }
            Value::String(text)
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| normalize_random(item, randoms))
                .collect(),
        ),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, item)| (key.clone(), normalize_random(item, randoms)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Sorts the keys of every object, for the row snapshots (N6).
pub fn sort_keys(value: &Value) -> Value {
    match value {
        Value::Object(fields) => {
            let mut keys: Vec<&String> = fields.keys().collect();
            keys.sort();
            Value::Object(
                keys.into_iter()
                    .map(|key| (key.clone(), sort_keys(&fields[key])))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.iter().map(sort_keys).collect()),
        other => other.clone(),
    }
}

/// N7: the title of an HTML page.
pub fn html_title(html: &str) -> Option<String> {
    let start = html.find("<title>")? + "<title>".len();
    let end = html[start..].find("</title>")? + start;
    Some(html[start..end].trim().to_owned())
}
