//! Shared harness for the integration suites: the real router on the test
//! database with a fixed clock, mock ESI, EVE SSO and GitHub servers on
//! ephemeral ports, request helpers that speak like the desktop clients,
//! the factories the PHP tests used and the JSON assertions.

// Not every suite uses every helper.
#![allow(dead_code)]

pub mod factories;
pub mod mocks;
pub mod socket;

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use time::OffsetDateTime;
use time::macros::datetime;
use tower::ServiceExt;

use telescope::clock::Clock;
use telescope::config::{Config, DemoConfig, EveConfig, PusherConfig};
use telescope::server::AppState;

pub use mocks::{MockEsi, MockGitHub, MockSso};

/// The site URL, as production has it and the reference recorded it.
pub const APP_URL: &str = "https://eve-telescope.com";
pub const HOST: &str = "eve-telescope.com";
/// The Pusher key every desktop client ships with.
pub const PUSHER_KEY: &str = "04pcwy13bvcyjoio6mf6";
pub const PUSHER_SECRET: &str = "test-pusher-secret";
pub const EVE_CLIENT_ID: &str = "test-client-id";
pub const EVE_CLIENT_SECRET: &str = "test-client-secret";

/// The moment the ported PHP tests run at (they used `now()`).
pub const DEFAULT_NOW: OffsetDateTime = datetime!(2026-10-13 08:30:00 UTC);

pub fn repo_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// One app under test: its router, the database it writes to, the clock
/// it reads and the mock services it calls.
pub struct TestApp {
    pub router: Router,
    pub state: AppState,
    pub pool: PgPool,
    pub esi: MockEsi,
    pub sso: MockSso,
    pub github: MockGitHub,
    pub config: Config,
    pub clock: Clock,
}

pub async fn test_pool() -> PgPool {
    telescope::db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`")
}

/// Clears every table; ids restart at 1, like Laravel's RefreshDatabase.
pub async fn reset_database(pool: &PgPool) {
    sqlx::query(
        "truncate alliances, corporations, users, personal_access_tokens, sessions,
            intel_networks, network_access, intel_entries, network_scans, shares
         restart identity cascade",
    )
    .execute(pool)
    .await
    .expect("tables truncate");
}

pub fn test_config(esi: &MockEsi, sso: &MockSso, github: &MockGitHub) -> Config {
    Config {
        app_name: "Telescope".to_owned(),
        app_url: APP_URL.to_owned(),
        eve: EveConfig {
            sso_base_url: sso.url(),
            client_id: EVE_CLIENT_ID.to_owned(),
            client_secret: EVE_CLIENT_SECRET.to_owned(),
            callback_url: format!("{APP_URL}/eve/callback"),
        },
        pusher: PusherConfig {
            app_id: "telescope".to_owned(),
            key: PUSHER_KEY.to_owned(),
            secret: PUSHER_SECRET.to_owned(),
            activity_timeout: 30,
            ping_interval: Duration::from_secs(60),
            max_message_size: 10_000,
        },
        esi_base_url: esi.url(),
        github_api_url: github.url(),
        scan_retention_days: 7,
        scheduler_enabled: false,
        session_lifetime_days: 30,
        demo: DemoConfig {
            enabled: false,
            character_id: 2_100_000_001,
        },
    }
}

impl TestApp {
    /// A fresh app on an empty database, its clock at `DEFAULT_NOW`.
    pub async fn new() -> Self {
        Self::at(DEFAULT_NOW).await
    }

    pub async fn at(now: OffsetDateTime) -> Self {
        let pool = test_pool().await;
        reset_database(&pool).await;
        let esi = MockEsi::start().await;
        let sso = MockSso::start().await;
        let github = MockGitHub::start().await;
        let config = test_config(&esi, &sso, &github);
        let clock = Clock::fixed(now);
        let state = AppState::new(pool.clone(), config.clone(), clock.clone());
        Self {
            router: telescope::server::router(state.clone()),
            state,
            pool,
            esi,
            sso,
            github,
            config,
            clock,
        }
    }

    /// Rebuilds the router with a changed configuration, on the same
    /// database, clock and mocks.
    pub fn reconfigure(&mut self, change: impl FnOnce(&mut Config)) {
        change(&mut self.config);
        self.state = AppState::new(self.pool.clone(), self.config.clone(), self.clock.clone());
        self.router = telescope::server::router(self.state.clone());
    }

    /// Serves the router on an ephemeral port, for the websocket tests;
    /// returns its address.
    pub async fn serve(&self) -> SocketAddr {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind the test server");
        let address = listener.local_addr().expect("an address");
        let router = self.router.clone();
        tokio::spawn(async move {
            axum::serve(listener, router).await.expect("serve");
        });
        address
    }

    /// A guest GET as a browser sends it.
    pub async fn get(&self, path: &str) -> TestResponse {
        self.get_with(path, &[]).await
    }

    pub async fn get_with(&self, path: &str, headers: &[(&str, &str)]) -> TestResponse {
        let mut all = vec![("host", HOST)];
        all.extend_from_slice(headers);
        self.request(Method::GET, path, header_map(&all), Body::empty())
            .await
    }

    /// A request the way the desktop clients send them: JSON both ways,
    /// with the bearer token when one is given.
    pub async fn json(
        &self,
        method: Method,
        path: &str,
        token: Option<&str>,
        body: Option<&Value>,
    ) -> TestResponse {
        let authorization = token.map(|token| format!("Bearer {token}"));
        let mut headers = vec![
            ("host", HOST),
            ("accept", "application/json"),
            ("content-type", "application/json"),
            ("user-agent", "Telescope | https://eve-telescope.com"),
        ];
        if let Some(authorization) = &authorization {
            headers.push(("authorization", authorization));
        }
        let body = body.map_or_else(Body::empty, |body| Body::from(body.to_string()));
        self.request(method, path, header_map(&headers), body).await
    }

    /// `getJson` as a user.
    pub async fn get_json(&self, token: &str, path: &str) -> TestResponse {
        self.json(Method::GET, path, Some(token), None).await
    }

    pub async fn post_json(&self, token: &str, path: &str, body: &Value) -> TestResponse {
        self.json(Method::POST, path, Some(token), Some(body)).await
    }

    pub async fn put_json(&self, token: &str, path: &str, body: &Value) -> TestResponse {
        self.json(Method::PUT, path, Some(token), Some(body)).await
    }

    pub async fn patch_json(&self, token: &str, path: &str, body: &Value) -> TestResponse {
        self.json(Method::PATCH, path, Some(token), Some(body))
            .await
    }

    pub async fn delete_json(&self, token: &str, path: &str) -> TestResponse {
        self.json(Method::DELETE, path, Some(token), None).await
    }

    pub async fn request(
        &self,
        method: Method,
        path: &str,
        headers: HeaderMap,
        body: Body,
    ) -> TestResponse {
        let mut request = Request::builder().method(method).uri(path);
        for (name, value) in &headers {
            request = request.header(name, value);
        }
        let response = self
            .router
            .clone()
            .oneshot(request.body(body).expect("request builds"))
            .await
            .expect("router answers");
        let status = response.status();
        let headers = response.headers().clone();
        let body = response
            .into_body()
            .collect()
            .await
            .expect("body reads")
            .to_bytes()
            .to_vec();
        TestResponse {
            status,
            headers,
            body,
        }
    }

    pub async fn count(&self, table: &str) -> i64 {
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!("select count(*) from {table}")))
            .fetch_one(&self.pool)
            .await
            .expect("count")
    }
}

pub fn header_map(headers: &[(&str, &str)]) -> HeaderMap {
    let mut map = HeaderMap::new();
    for (name, value) in headers {
        map.append(
            axum::http::HeaderName::from_bytes(name.as_bytes()).expect("header name"),
            value.parse().expect("header value"),
        );
    }
    map
}

pub struct TestResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

impl TestResponse {
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or_else(|error| {
            panic!(
                "{} is not JSON ({error}): {}",
                self.status,
                String::from_utf8_lossy(&self.body)
            )
        })
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|value| value.to_str().ok())
    }

    /// Asserts the status, showing the body when it differs.
    #[track_caller]
    pub fn assert_status(&self, status: u16) -> &Self {
        assert_eq!(self.status.as_u16(), status, "{}", self.text());
        self
    }

    /// The target of a redirect.
    #[track_caller]
    pub fn location(&self) -> &str {
        assert!(
            self.status.is_redirection(),
            "{} is no redirect: {}",
            self.status,
            self.text()
        );
        self.header("location").expect("a location")
    }

    /// Every `Set-Cookie` header.
    pub fn set_cookies(&self) -> Vec<String> {
        self.headers
            .get_all("set-cookie")
            .iter()
            .filter_map(|value| value.to_str().ok())
            .map(str::to_owned)
            .collect()
    }

    /// The `name=value` pair of a cookie this answer sets, for the next
    /// request's `Cookie` header.
    pub fn cookie(&self, name: &str) -> Option<String> {
        self.set_cookies().into_iter().find_map(|header| {
            let pair = header.split(';').next()?.trim().to_owned();
            pair.starts_with(&format!("{name}=")).then_some(pair)
        })
    }
}

/// Asserts two JSON values are equal with the same keys in the same order
/// at every level, naming the first path that differs.
#[track_caller]
pub fn assert_json_eq(expected: &Value, actual: &Value) {
    if let Some(difference) = json_difference("$", expected, actual) {
        panic!(
            "{difference}\n  expected {}\n  actual   {}",
            serde_json::to_string(expected).unwrap_or_default(),
            serde_json::to_string(actual).unwrap_or_default()
        );
    }
}

pub fn json_difference(path: &str, expected: &Value, actual: &Value) -> Option<String> {
    match (expected, actual) {
        (Value::Object(expected), Value::Object(actual)) => {
            let expected_keys: Vec<_> = expected.keys().collect();
            let actual_keys: Vec<_> = actual.keys().collect();
            if expected_keys != actual_keys {
                return Some(format!(
                    "{path}: keys differ\n  expected {expected_keys:?}\n  actual   {actual_keys:?}"
                ));
            }
            expected.iter().find_map(|(key, value)| {
                json_difference(&format!("{path}.{key}"), value, &actual[key])
            })
        }
        (Value::Array(expected), Value::Array(actual)) => {
            if expected.len() != actual.len() {
                return Some(format!(
                    "{path}: expected {} items, got {}",
                    expected.len(),
                    actual.len()
                ));
            }
            expected
                .iter()
                .zip(actual)
                .enumerate()
                .find_map(|(index, (expected, actual))| {
                    json_difference(&format!("{path}[{index}]"), expected, actual)
                })
        }
        (expected, actual) if expected != actual => {
            Some(format!("{path}: expected {expected}, got {actual}"))
        }
        _ => None,
    }
}

/// The keys of a JSON object, in order.
#[track_caller]
pub fn keys(value: &Value) -> Vec<String> {
    value
        .as_object()
        .unwrap_or_else(|| panic!("{value} is no object"))
        .keys()
        .cloned()
        .collect()
}

/// The routes of the Laravel app: (method, path), HEAD left out where
/// GET has it.
pub fn read_routes() -> Vec<(String, String)> {
    let text = std::fs::read_to_string(repo_path("tests/fixtures/routes.json")).expect("routes");
    let routes: Vec<Value> = serde_json::from_str(&text).expect("routes JSON");
    let mut all = Vec::new();
    for route in routes {
        let uri = route["uri"].as_str().expect("a uri");
        let uri = if uri == "/" {
            "/".to_owned()
        } else {
            format!("/{uri}")
        };
        for method in route["method"].as_str().expect("methods").split('|') {
            all.push((method.to_owned(), uri.clone()));
        }
    }
    all
}
