//! `network:purge-expired-access`: deletes the network access rows whose
//! expiry has passed.

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let pool = telescope::db::connect().await.expect("database connection");
    let now = telescope::clock::Clock::system().now();
    let count = telescope::scheduler::purge_expired_access(&pool, now)
        .await
        .expect("the purge");
    println!(
        "{}",
        telescope::scheduler::purge_expired_access_report(count)
    );
}
