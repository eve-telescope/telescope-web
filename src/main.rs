#[tokio::main]
async fn main() {
    // Structured logs; tune with RUST_LOG (e.g. RUST_LOG=telescope=debug).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Local configuration from .env, if present; real environment wins.
    dotenvy::dotenv().ok();

    let config = telescope::config::Config::from_env();
    let addr = telescope::server::bind_addr();

    let pool = telescope::db::connect().await.expect("database connection");
    telescope::db::migrate(&pool)
        .await
        .expect("database migrations");

    let state = telescope::server::AppState::new(pool, config, telescope::clock::Clock::system());
    let app = telescope::server::router(state);

    tracing::info!("listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("bind address");
    axum::serve(listener, app).await.expect("serve");
}
