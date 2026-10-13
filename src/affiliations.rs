//! Which corporation and alliance each pilot belongs to, from ESI: at
//! login and hourly for everyone. Corporations and alliances are fetched
//! when they are missing or were last fetched more than 24 hours ago (or
//! never). A pilot is only pointed at a corporation or alliance that is
//! stored, so an ESI failure never breaks the foreign keys.

use sqlx::PgPool;
use time::{Duration, OffsetDateTime};

use crate::esi::{Affiliation, EsiClient};
use crate::server::AppState;

/// ESI's limit of characters per affiliation request.
const CHUNK: usize = 1000;

/// A row is fresh for 24 hours after it was fetched.
fn is_stale(last_updated: Option<OffsetDateTime>, now: OffsetDateTime) -> bool {
    last_updated.is_none_or(|fetched| now - fetched > Duration::hours(24))
}

/// `None` when the row does not exist; `Some(last_updated)` otherwise.
async fn stored(
    pool: &PgPool,
    table: &str,
    id: i64,
) -> sqlx::Result<Option<Option<OffsetDateTime>>> {
    sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "select last_updated from {table} where id = $1"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Makes sure an alliance is stored and fresh; answers whether it is
/// stored now.
pub async fn ensure_alliance(
    pool: &PgPool,
    esi: &EsiClient,
    id: i64,
    now: OffsetDateTime,
) -> sqlx::Result<bool> {
    let row = stored(pool, "alliances", id).await?;
    if row.is_none_or(|last_updated| is_stale(last_updated, now))
        && let Some(alliance) = esi.alliance(id).await
    {
        sqlx::query(
            "insert into alliances (id, name, ticker, last_updated, created_at, updated_at)
             values ($1, $2, $3, $4, $4, $4)
             on conflict (id) do update set name = excluded.name, ticker = excluded.ticker,
                last_updated = excluded.last_updated, updated_at = excluded.updated_at",
        )
        .bind(id)
        .bind(&alliance.name)
        .bind(&alliance.ticker)
        .bind(now)
        .execute(pool)
        .await?;
        return Ok(true);
    }
    Ok(row.is_some())
}

/// Makes sure a corporation is stored and fresh, its alliance too;
/// answers whether it is stored now.
pub async fn ensure_corporation(
    pool: &PgPool,
    esi: &EsiClient,
    id: i64,
    now: OffsetDateTime,
) -> sqlx::Result<bool> {
    let row = stored(pool, "corporations", id).await?;
    if row.is_none_or(|last_updated| is_stale(last_updated, now))
        && let Some(corporation) = esi.corporation(id).await
    {
        // An alliance that cannot be stored is left out.
        let alliance_id = match corporation.alliance_id {
            Some(alliance) if ensure_alliance(pool, esi, alliance, now).await? => Some(alliance),
            _ => None,
        };
        sqlx::query(
            "insert into corporations (id, name, ticker, alliance_id, member_count, last_updated,
                created_at, updated_at)
             values ($1, $2, $3, $4, $5, $6, $6, $6)
             on conflict (id) do update set name = excluded.name, ticker = excluded.ticker,
                alliance_id = excluded.alliance_id, member_count = excluded.member_count,
                last_updated = excluded.last_updated, updated_at = excluded.updated_at",
        )
        .bind(id)
        .bind(&corporation.name)
        .bind(&corporation.ticker)
        .bind(alliance_id)
        .bind(corporation.member_count.map(|count| count as i32))
        .bind(now)
        .execute(pool)
        .await?;
        return Ok(true);
    }
    Ok(row.is_some())
}

/// The corporation and alliance of an affiliation, as far as they are
/// stored (the alliance is fetched first).
pub async fn resolve(
    pool: &PgPool,
    esi: &EsiClient,
    affiliation: &Affiliation,
    now: OffsetDateTime,
) -> sqlx::Result<(Option<i64>, Option<i64>)> {
    let alliance = match affiliation.alliance_id {
        Some(id) if ensure_alliance(pool, esi, id, now).await? => Some(id),
        _ => None,
    };
    let corporation = match affiliation.corporation_id {
        Some(id) if ensure_corporation(pool, esi, id, now).await? => Some(id),
        _ => None,
    };
    Ok((corporation, alliance))
}

/// The affiliation of one character at login; `None` when ESI did not
/// answer (the user keeps what it had).
pub async fn for_character(
    pool: &PgPool,
    esi: &EsiClient,
    character_id: i64,
    now: OffsetDateTime,
) -> sqlx::Result<Option<(Option<i64>, Option<i64>)>> {
    let Some(answer) = esi.affiliations(&[character_id]).await else {
        return Ok(None);
    };
    let Some(affiliation) = answer.first() else {
        return Ok(None);
    };
    resolve(pool, esi, affiliation, now).await.map(Some)
}

/// Sets a user's corporation and alliance; `updated_at` moves only when
/// they change.
pub async fn update_user(
    pool: &PgPool,
    character_id: i64,
    corporation: Option<i64>,
    alliance: Option<i64>,
    now: OffsetDateTime,
) -> sqlx::Result<()> {
    sqlx::query(
        "update users set corporation_id = $2, alliance_id = $3,
            updated_at = case when (corporation_id, alliance_id) is distinct from ($2, $3)
                then $4 else updated_at end
         where character_id = $1",
    )
    .bind(character_id)
    .bind(corporation)
    .bind(alliance)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

/// RefreshAffiliations: every user's corporation and alliance, in chunks
/// of 1000 characters. A chunk ESI does not answer is skipped, characters
/// it leaves out keep theirs. Answers how many users were updated.
pub async fn refresh_all(state: &AppState) -> usize {
    match refresh(&state.pool, &state.esi, state.clock.now()).await {
        Ok(updated) => updated,
        Err(error) => {
            tracing::error!(%error, "affiliations not refreshed");
            0
        }
    }
}

async fn refresh(pool: &PgPool, esi: &EsiClient, now: OffsetDateTime) -> sqlx::Result<usize> {
    let characters: Vec<i64> = sqlx::query_scalar("select character_id from users order by id")
        .fetch_all(pool)
        .await?;
    let mut updated = 0;
    for chunk in characters.chunks(CHUNK) {
        let Some(affiliations) = esi.affiliations(chunk).await else {
            continue;
        };
        for affiliation in affiliations {
            if !chunk.contains(&affiliation.character_id) {
                continue;
            }
            let (corporation, alliance) = resolve(pool, esi, &affiliation, now).await?;
            update_user(pool, affiliation.character_id, corporation, alliance, now).await?;
            updated += 1;
        }
    }
    Ok(updated)
}
