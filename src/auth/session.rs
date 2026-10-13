//! The website's sign-in: a random token in the `telescope_session`
//! cookie (HttpOnly, SameSite=Lax, Secure on HTTPS), stored as its SHA-256
//! in `sessions`, good for `SESSION_LIFETIME_DAYS` from the login. It only
//! feeds the pages' shared `auth.user`; the API never reads it.

use axum::http::{HeaderMap, header};
use axum_extra::extract::cookie::{Cookie, SameSite};
use sqlx::PgPool;
use time::{Duration, OffsetDateTime};

use super::tokens::{hash, random_string};
use crate::models::{USER_COLUMNS, User};

pub const SESSION_COOKIE: &str = "telescope_session";

/// Starts a session for a user; answers the cookie's token.
pub async fn start(pool: &PgPool, user_id: i64, now: OffsetDateTime) -> sqlx::Result<String> {
    let token = random_string(40);
    sqlx::query(
        "insert into sessions (id, user_id, last_activity, created_at) values ($1, $2, $3, $3)",
    )
    .bind(hash(&token))
    .bind(user_id)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(token)
}

/// The session cookie for a token.
pub fn cookie(token: String, lifetime_days: i64, secure: bool) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(secure)
        .max_age(Duration::days(lifetime_days))
        .build()
}

/// A cookie's value in a request's `Cookie` headers.
pub fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| Cookie::parse(pair.trim().to_owned()).ok())
        .find(|cookie| cookie.name() == name)
        .map(|cookie| cookie.value().to_owned())
}

/// The signed-in user of a request's session cookie, if the session is
/// younger than its lifetime.
pub async fn user(
    pool: &PgPool,
    headers: &HeaderMap,
    now: OffsetDateTime,
    lifetime_days: i64,
) -> sqlx::Result<Option<User>> {
    let Some(token) = cookie_value(headers, SESSION_COOKIE) else {
        return Ok(None);
    };
    let user: Option<User> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "update sessions s set last_activity = $2
         from users u
         where s.id = $1 and u.id = s.user_id and s.created_at > $2 - make_interval(days => $3::int)
         returning {}",
        USER_COLUMNS
            .split(", ")
            .map(|column| format!("u.{column}"))
            .collect::<Vec<_>>()
            .join(", ")
    )))
    .bind(hash(&token))
    .bind(now)
    .bind(lifetime_days as i32)
    .fetch_optional(pool)
    .await?;
    Ok(user)
}

/// Deletes the sessions past their lifetime; answers how many.
pub async fn delete_expired(
    pool: &PgPool,
    now: OffsetDateTime,
    lifetime_days: i64,
) -> sqlx::Result<u64> {
    let result =
        sqlx::query("delete from sessions where created_at <= $1 - make_interval(days => $2::int)")
            .bind(now)
            .bind(lifetime_days as i32)
            .execute(pool)
            .await?;
    Ok(result.rows_affected())
}
