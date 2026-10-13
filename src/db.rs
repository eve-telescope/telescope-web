//! Postgres access: connection pool, migrations and the test database.

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

/// The local docker-compose Postgres (see `docker-compose.yml`).
pub const DEFAULT_DATABASE_URL: &str = "postgres://telescope:telescope@127.0.0.1:5448/telescope";

/// Small enough for the shared Postgres.
const MAX_POOL_CONNECTIONS: u32 = 10;

/// Postgres' unique_violation SQLSTATE, raised when a concurrent test
/// binary wins the create-database race.
const PG_UNIQUE_VIOLATION: &str = "23505";

pub fn database_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_owned())
}

pub async fn connect() -> sqlx::Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(MAX_POOL_CONNECTIONS)
        .connect(&database_url())
        .await
}

pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!().run(pool).await
}

/// The dedicated test database, so test seeding never wipes development
/// data.
pub const DEFAULT_TEST_DATABASE_URL: &str =
    "postgres://telescope:telescope@127.0.0.1:5448/telescope_test";

pub fn test_database_url() -> String {
    std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| DEFAULT_TEST_DATABASE_URL.to_owned())
}

/// Connects to the test database, creating and migrating it on first use.
pub async fn test_pool() -> sqlx::Result<PgPool> {
    let url = test_database_url();
    ensure_database(&url).await?;

    let pool = PgPoolOptions::new()
        .max_connections(MAX_POOL_CONNECTIONS)
        .connect(&url)
        .await?;
    migrate(&pool).await.map_err(sqlx::Error::from)?;
    Ok(pool)
}

/// Creates the database of the given URL if it does not exist yet, via the
/// server's maintenance database.
async fn ensure_database(url: &str) -> sqlx::Result<()> {
    let (server_url, database) = url
        .rsplit_once('/')
        .expect("database URL with a database name");

    // Guard the identifier: it is interpolated into CREATE DATABASE, which
    // cannot take bind parameters.
    assert!(
        database
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "unsafe test database name: {database}",
    );

    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&format!("{server_url}/postgres"))
        .await?;

    let exists: Option<i32> = sqlx::query_scalar("select 1 from pg_database where datname = $1")
        .bind(database)
        .fetch_optional(&admin)
        .await?;

    if exists.is_none()
        && let Err(error) = sqlx::query(sqlx::AssertSqlSafe(format!(
            r#"create database "{database}""#
        )))
        .execute(&admin)
        .await
    {
        // Concurrent test binaries race this create; losing the race
        // (unique_violation on the database name) means it exists now.
        let lost_create_race = error
            .as_database_error()
            .and_then(|db_error| db_error.code())
            .is_some_and(|code| code == PG_UNIQUE_VIOLATION);

        if !lost_create_race {
            return Err(error);
        }
    }

    Ok(())
}
