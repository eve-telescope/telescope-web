//! RefreshAffiliations once: every pilot's corporation and alliance from
//! ESI.

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let config = telescope::config::Config::from_env();
    let pool = telescope::db::connect().await.expect("database connection");
    let state = telescope::server::AppState::new(pool, config, telescope::clock::Clock::system());
    let updated = telescope::affiliations::refresh_all(&state).await;
    println!("Refreshed the affiliations of {updated} users.");
}
