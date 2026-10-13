//! Seeds the demo pilot (DEMO_CHARACTER_ID) and its sample networks,
//! entries, scans and share, for local stacks. Migrates first; safe to
//! repeat.

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let config = telescope::config::Config::from_env();
    let pool = telescope::db::connect().await.expect("database connection");
    telescope::db::migrate(&pool)
        .await
        .expect("database migrations");
    let now = telescope::clock::Clock::system().now();
    telescope::demo::seed(&pool, now, config.demo.character_id)
        .await
        .expect("the demo seed");
    println!(
        "Seeded {} ({}) and the share {}.",
        telescope::demo::DEMO_CHARACTER_NAME,
        config.demo.character_id,
        telescope::demo::DEMO_SHARE_CODE
    );
}
