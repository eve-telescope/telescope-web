//! Sanctum's personal access tokens, the desktop clients' credential.
//!
//! The plain token is `<id>|<40 random letters and digits><crc32b of
//! them>`; the table stores the SHA-256 (hex) of the part after the pipe.
//! A bearer token with a pipe is looked up by its id and compared in
//! constant time, one without by its hash. A token past its `expires_at`
//! is refused; every accepted request stamps `last_used_at` (and so
//! `updated_at`). Rows migrated from Laravel keep working unchanged.

use axum::extract::FromRequestParts;
use axum::http::header;
use axum::http::request::Parts;
use axum::response::Response;
use rand::RngExt;
use rand::distr::Alphanumeric;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::models::{USER_COLUMNS, User};
use crate::server::AppState;
use crate::server::errors;

/// The stored form of a token's secret.
pub fn hash(secret: &str) -> String {
    hex::encode(Sha256::digest(secret.as_bytes()))
}

/// `Str::random($length)`: letters and digits from a CSPRNG.
pub fn random_string(length: usize) -> String {
    rand::rng()
        .sample_iter(Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

/// A new token's secret: 40 random characters and their CRC32 in hex.
pub fn new_secret() -> String {
    let random = random_string(40);
    format!("{random}{:08x}", crc32fast::hash(random.as_bytes()))
}

/// `$user->createToken($name)`: stores a token; answers its id and the
/// plain text token (`<id>|<secret>`), which is shown once.
pub async fn create(
    pool: &PgPool,
    user_id: i64,
    name: &str,
    now: OffsetDateTime,
) -> sqlx::Result<(i64, String)> {
    let secret = new_secret();
    let id: i64 = sqlx::query_scalar(
        "insert into personal_access_tokens (tokenable_type, tokenable_id, name, token,
            abilities, created_at, updated_at)
         values ('App\\Models\\User', $1, $2, $3, '[\"*\"]', $4, $4) returning id",
    )
    .bind(user_id)
    .bind(name)
    .bind(hash(&secret))
    .bind(now)
    .fetch_one(pool)
    .await?;
    Ok((id, format!("{id}|{secret}")))
}

/// The bearer token of a request, as Laravel's `bearerToken()` reads it:
/// after the last `Bearer `, up to a comma.
pub fn bearer(parts: &Parts) -> Option<String> {
    let header = parts.headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let position = header.rfind("Bearer ")?;
    let token = &header[position + "Bearer ".len()..];
    let token = token.split(',').next().unwrap_or_default();
    (!token.is_empty()).then(|| token.to_owned())
}

/// The token row a bearer token names, if it is valid now.
async fn find(pool: &PgPool, token: &str, now: OffsetDateTime) -> sqlx::Result<Option<(i64, i64)>> {
    let row: Option<(i64, i64, String, Option<OffsetDateTime>)> = match token.split_once('|') {
        Some((id, secret)) => {
            let Ok(id) = id.parse::<i64>() else {
                return Ok(None);
            };
            let row: Option<(i64, i64, String, Option<OffsetDateTime>)> = sqlx::query_as(
                "select id, tokenable_id, token, expires_at from personal_access_tokens
                 where id = $1 and tokenable_type = 'App\\Models\\User'",
            )
            .bind(id)
            .fetch_optional(pool)
            .await?;
            row.filter(|(_, _, stored, _)| constant_time_eq(stored, &hash(secret)))
        }
        None => {
            sqlx::query_as(
                "select id, tokenable_id, token, expires_at from personal_access_tokens
                 where token = $1 and tokenable_type = 'App\\Models\\User'",
            )
            .bind(hash(token))
            .fetch_optional(pool)
            .await?
        }
    };
    Ok(row
        .filter(|(_, _, _, expires_at)| expires_at.is_none_or(|expires_at| expires_at > now))
        .map(|(id, user, _, _)| (id, user)))
}

/// Compares two hex hashes without an early exit.
fn constant_time_eq(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |difference, (x, y)| difference | (x ^ y))
            == 0
}

/// The user of a valid bearer token (`auth:sanctum`); a request without
/// one is answered 401 before anything else happens.
#[derive(Debug, Clone)]
pub struct ApiUser {
    pub user: User,
    pub token_id: i64,
}

impl FromRequestParts<AppState> for ApiUser {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Response> {
        let Some(token) = bearer(parts) else {
            return Err(errors::unauthenticated());
        };
        let now = state.clock.now();
        let Some((token_id, user_id)) = find(&state.pool, &token, now)
            .await
            .map_err(errors::server_error)?
        else {
            return Err(errors::unauthenticated());
        };
        sqlx::query(
            "update personal_access_tokens set last_used_at = $2, updated_at = $2 where id = $1",
        )
        .bind(token_id)
        .bind(now)
        .execute(&state.pool)
        .await
        .map_err(errors::server_error)?;
        let user: Option<User> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "select {USER_COLUMNS} from users where id = $1"
        )))
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(errors::server_error)?;
        match user {
            Some(user) => Ok(Self { user, token_id }),
            None => Err(errors::unauthenticated()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_carries_its_checksum() {
        let secret = new_secret();
        assert_eq!(secret.len(), 48);
        let (random, checksum) = secret.split_at(40);
        assert_eq!(
            checksum,
            format!("{:08x}", crc32fast::hash(random.as_bytes()))
        );
    }

    #[test]
    fn reads_the_bearer_like_laravel() {
        let parts = |value: &str| {
            let request = axum::http::Request::builder()
                .header(header::AUTHORIZATION, value)
                .body(())
                .expect("a request");
            request.into_parts().0
        };
        assert_eq!(bearer(&parts("Bearer 1|abc")), Some("1|abc".to_owned()));
        assert_eq!(
            bearer(&parts("Bearer 1|abc,more")),
            Some("1|abc".to_owned())
        );
        assert_eq!(
            bearer(&parts("Basic x, Bearer 2|z")),
            Some("2|z".to_owned())
        );
        assert_eq!(bearer(&parts("Bearer ")), None);
        assert_eq!(bearer(&parts("Token 1|abc")), None);
    }
}
