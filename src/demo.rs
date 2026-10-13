//! The local demo (DEMO_ENABLED, never in production): a demo pilot the
//! demo login signs in as, and `seed_demo`'s sample data around it, so the
//! site and the API can be tried without EVE SSO. The seed replaces its
//! own rows and can run again.

use serde_json::json;
use sqlx::PgPool;
use time::{Duration, OffsetDateTime};

use crate::models::{USER_COLUMNS, User};

pub const DEMO_CHARACTER_NAME: &str = "Demo Pilot";
pub const DEMO_SHARE_CODE: &str = "DEMO2026";
const DEMO_CORPORATION: (i64, &str, &str) = (98_999_001, "Telescope Demo Corp", "TDEMO");
const DEMO_ALLIANCE: (i64, &str, &str) = (99_999_001, "Telescope Demo Alliance", "TDA");

async fn upsert_user(
    pool: &PgPool,
    character_id: i64,
    name: &str,
    corporation: Option<i64>,
    alliance: Option<i64>,
    now: OffsetDateTime,
) -> sqlx::Result<User> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "insert into users (character_id, character_name, character_owner_hash, corporation_id,
            alliance_id, created_at, updated_at)
         values ($1, $2, 'demo', $3, $4, $5, $5)
         on conflict (character_id) do update set character_name = excluded.character_name
         returning {USER_COLUMNS}"
    )))
    .bind(character_id)
    .bind(name)
    .bind(corporation)
    .bind(alliance)
    .bind(now)
    .fetch_one(pool)
    .await
}

/// The demo pilot, created when it is missing.
pub async fn pilot(pool: &PgPool, now: OffsetDateTime, character_id: i64) -> sqlx::Result<User> {
    upsert_user(pool, character_id, DEMO_CHARACTER_NAME, None, None, now).await
}

/// The demo pilot in a corporation and alliance, two squad mates, two
/// networks with entries and scans, and a share.
pub async fn seed(pool: &PgPool, now: OffsetDateTime, character_id: i64) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "insert into alliances (id, name, ticker, last_updated, created_at, updated_at)
         values ($1, $2, $3, $4, $4, $4) on conflict (id) do nothing",
    )
    .bind(DEMO_ALLIANCE.0)
    .bind(DEMO_ALLIANCE.1)
    .bind(DEMO_ALLIANCE.2)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "insert into corporations (id, name, ticker, alliance_id, member_count, last_updated,
            created_at, updated_at)
         values ($1, $2, $3, $4, 3, $5, $5, $5) on conflict (id) do nothing",
    )
    .bind(DEMO_CORPORATION.0)
    .bind(DEMO_CORPORATION.1)
    .bind(DEMO_CORPORATION.2)
    .bind(DEMO_ALLIANCE.0)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let corporation = Some(DEMO_CORPORATION.0);
    let alliance = Some(DEMO_ALLIANCE.0);
    let demo = upsert_user(
        pool,
        character_id,
        DEMO_CHARACTER_NAME,
        corporation,
        alliance,
        now,
    )
    .await?;
    sqlx::query("update users set corporation_id = $2, alliance_id = $3 where id = $1")
        .bind(demo.id)
        .bind(corporation)
        .bind(alliance)
        .execute(pool)
        .await?;
    let wingmate = upsert_user(
        pool,
        character_id + 1,
        "Demo Wingmate",
        corporation,
        alliance,
        now,
    )
    .await?;
    let scout = upsert_user(pool, character_id + 2, "Demo Scout", None, alliance, now).await?;

    // The previous seed's rows go; the users stay.
    sqlx::query("delete from intel_networks where owner_id = any($1)")
        .bind(vec![demo.id, wingmate.id, scout.id])
        .execute(pool)
        .await?;
    sqlx::query("delete from shares where code = $1")
        .bind(DEMO_SHARE_CODE)
        .execute(pool)
        .await?;

    let home = network(pool, "Home Defense", "home-defense-demo01", demo.id, now).await?;
    grant(
        pool,
        home,
        "App\\Models\\User",
        demo.character_id,
        "manager",
        true,
        now,
    )
    .await?;
    grant(
        pool,
        home,
        "App\\Models\\User",
        wingmate.character_id,
        "member",
        false,
        now,
    )
    .await?;
    grant(
        pool,
        home,
        "App\\Models\\Corporation",
        DEMO_CORPORATION.0,
        "viewer",
        false,
        now,
    )
    .await?;
    let alliance_intel = network(
        pool,
        "Alliance Intel",
        "alliance-intel-demo02",
        wingmate.id,
        now,
    )
    .await?;
    grant(
        pool,
        alliance_intel,
        "App\\Models\\User",
        wingmate.character_id,
        "manager",
        true,
        now,
    )
    .await?;
    grant(
        pool,
        alliance_intel,
        "App\\Models\\Alliance",
        DEMO_ALLIANCE.0,
        "member",
        false,
        now,
    )
    .await?;

    for (network_id, entity_type, entity_id, name, color, label, notes, by) in [
        (
            home,
            "character",
            2_112_000_001_i64,
            "Gatecamp Gary",
            Some("#FF3B3B"),
            Some("HOSTILE"),
            Some("Camps the Jita undock in a Sabre."),
            demo.id,
        ),
        (
            home,
            "corporation",
            98_999_777,
            "Spy Agency Inc",
            None,
            Some("SPY"),
            Some("Alts in our channels."),
            wingmate.id,
        ),
        (
            home,
            "alliance",
            99_999_777,
            "Red Menace Alliance",
            Some("#FFAA22"),
            Some("KOS"),
            None,
            demo.id,
        ),
        (
            alliance_intel,
            "character",
            2_112_000_001,
            "Gatecamp Gary",
            Some("#44DDAA"),
            Some("NEUTRAL"),
            None,
            wingmate.id,
        ),
        (
            alliance_intel,
            "character",
            2_112_000_002,
            "Friendly Fred",
            Some("#00D4FF"),
            Some("BLUE"),
            Some("Fleet commander of our allies."),
            scout.id,
        ),
    ] {
        sqlx::query(
            "insert into intel_entries (intel_network_id, entity_type, entity_id, entity_name,
                color, label, notes, added_by, created_at, updated_at)
             values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9)",
        )
        .bind(network_id)
        .bind(entity_type)
        .bind(entity_id)
        .bind(name)
        .bind(color)
        .bind(label)
        .bind(notes)
        .bind(by)
        .bind(now)
        .execute(pool)
        .await?;
    }

    for (index, (network_id, scan_type, text, system, by)) in [
        (
            home,
            "local",
            "Gatecamp Gary\nFriendly Fred\nDemo Scout",
            Some("Jita"),
            demo.id,
        ),
        (
            home,
            "dscan",
            "11198\tSabre\tSabre\t1,200 km\n587\tRifter\tRifter\t-",
            Some("Jita"),
            wingmate.id,
        ),
        (
            home,
            "local",
            "Gatecamp Gary\nSome Neutral",
            Some("Perimeter"),
            scout.id,
        ),
        (
            alliance_intel,
            "local",
            "Friendly Fred\nDemo Wingmate",
            None,
            wingmate.id,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let at = now - Duration::minutes(15 * (4 - index as i64));
        sqlx::query(
            "insert into network_scans (intel_network_id, user_id, scan_type, raw_text,
                solar_system, created_at, updated_at)
             values ($1, $2, $3, $4, $5, $6, $6)",
        )
        .bind(network_id)
        .bind(by)
        .bind(scan_type)
        .bind(text)
        .bind(system)
        .bind(at)
        .execute(pool)
        .await?;
    }

    let pilots: Vec<String> = (1..=12).map(|n| format!("Demo Hostile {n:02}")).collect();
    sqlx::query(
        "insert into shares (code, pilots, pilot_count, views, created_at, updated_at)
         values ($1, $2, $3, 0, $4, $4)",
    )
    .bind(DEMO_SHARE_CODE)
    .bind(json!(pilots))
    .bind(pilots.len() as i32)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

async fn network(
    pool: &PgPool,
    name: &str,
    slug: &str,
    owner: i64,
    now: OffsetDateTime,
) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        "insert into intel_networks (name, slug, owner_id, created_at, updated_at)
         values ($1, $2, $3, $4, $4)
         on conflict (slug) do update set name = excluded.name
         returning id",
    )
    .bind(name)
    .bind(slug)
    .bind(owner)
    .bind(now)
    .fetch_one(pool)
    .await
}

async fn grant(
    pool: &PgPool,
    network: i64,
    class: &str,
    id: i64,
    permission: &str,
    owner: bool,
    now: OffsetDateTime,
) -> sqlx::Result<()> {
    sqlx::query(
        "insert into network_access (intel_network_id, accessible_type, accessible_id, permission,
            is_owner, created_at, updated_at)
         values ($1, $2, $3, $4, $5, $6, $6)",
    )
    .bind(network)
    .bind(class)
    .bind(id)
    .bind(permission)
    .bind(owner)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}
