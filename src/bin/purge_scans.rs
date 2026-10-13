//! `scans:purge [--days=N]`: deletes the shared scans older than
//! SCAN_RETENTION_DAYS (7 by default; 0 keeps them all) or `--days`.

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let config = telescope::config::Config::from_env();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let days = telescope::scheduler::parse_days_argument(&args, config.scan_retention_days)
        .unwrap_or_else(|error| {
            eprintln!("{error}");
            std::process::exit(2);
        });
    let pool = telescope::db::connect().await.expect("database connection");
    let now = telescope::clock::Clock::system().now();
    let count = telescope::scheduler::purge_scans(&pool, now, days)
        .await
        .expect("the purge");
    println!("{}", telescope::scheduler::purge_scans_report(count, days));
}
