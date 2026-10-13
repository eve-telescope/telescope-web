//! Applies the migrations to DATABASE_URL and exits: the deploy step
//! before the new api starts.

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let pool = telescope::db::connect().await.expect("database connection");
    telescope::db::migrate(&pool)
        .await
        .expect("database migrations");
    println!("Migrations applied.");
}
