//! The HTTP side: shared state, the router and the cross-cutting layers.
//! One router serves both host names: eve-telescope.com (the JSON API of
//! the desktop clients, the props of the SvelteKit pages, the EVE login)
//! and ws.eve-telescope.com (the Pusher protocol websocket).
//!
//! No route takes a cookie-authenticated mutation: the API authenticates
//! with bearer tokens, which no foreign page can attach, and stays open
//! to every origin (CORS `*`) as before; the website only reads. So there
//! is no CSRF token and no same-origin check.

pub mod cors;
pub mod errors;
pub mod health;

use std::sync::Arc;

use axum::Router;
use axum::extract::{DefaultBodyLimit, Request};
use axum::http::{Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use sqlx::PgPool;
use tower_http::normalize_path::NormalizePath;

use crate::clock::Clock;
use crate::config::Config;

/// The default bind address of the API server; the SvelteKit dev proxy and
/// the reverse proxy point at it.
const DEFAULT_BIND_ADDR: &str = "127.0.0.1:3000";

/// The listen address from `BIND_ADDR`, with the local default.
pub fn bind_addr() -> String {
    std::env::var("BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_owned())
}

/// A shared scan may hold 500 000 characters, up to four bytes each.
const BODY_LIMIT: usize = 8 * 1024 * 1024;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
    pub clock: Clock,
}

impl AppState {
    pub fn new(pool: PgPool, config: Config, clock: Clock) -> Self {
        Self {
            pool,
            config: Arc::new(config),
            clock,
        }
    }
}

/// The whole app. A trailing slash reaches the same route, as it did.
pub fn router(state: AppState) -> Router {
    let routes = routes(state);
    Router::new().fallback_service(NormalizePath::trim_trailing_slash(routes))
}

fn routes(state: AppState) -> Router {
    use errors::not_implemented;
    Router::new()
        .route("/up", get(health::up))
        // The EVE login, the CSRF cookie nothing uses and the local disk.
        .route("/eve", get(not_implemented))
        .route("/sanctum/csrf-cookie", get(not_implemented))
        .route(
            "/storage/{*path}",
            get(not_implemented).put(not_implemented),
        )
        // The realtime channel: subscription signatures and the socket.
        .route(
            "/broadcasting/auth",
            get(not_implemented).post(not_implemented),
        )
        .route("/app/{key}", get(not_implemented))
        .nest("/api", api_router())
        .fallback(errors::not_found)
        .method_not_allowed_fallback(errors::method_not_allowed)
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            reload_inertia_requests,
        ))
        .layer(axum::middleware::from_fn(errors::negotiate))
        .layer(axum::middleware::from_fn(cors::handle))
        .with_state(state)
}

/// The desktop clients' API, the props of the pages and the health check.
fn api_router() -> Router<AppState> {
    use errors::not_implemented;
    Router::new()
        .route("/health", get(health::health))
        .route("/pages/home", get(not_implemented))
        .route("/pages/s/{code}", get(not_implemented))
        .route("/pages/eve/callback", get(not_implemented))
        .route("/pages/demo-login", get(not_implemented))
        .route("/user", get(not_implemented))
        .route("/share", post(not_implemented))
        .route("/share/{code}", get(not_implemented))
        .route("/tokens", post(not_implemented))
        .route("/tokens/{token}", axum::routing::delete(not_implemented))
        .route("/networks", get(not_implemented).post(not_implemented))
        .route(
            "/networks/{network}",
            get(not_implemented)
                .put(not_implemented)
                .delete(not_implemented),
        )
        .route(
            "/networks/{network}/access",
            get(not_implemented).post(not_implemented),
        )
        .route(
            "/networks/{network}/access/{access}",
            axum::routing::put(not_implemented).delete(not_implemented),
        )
        .route(
            "/networks/{network}/entries",
            get(not_implemented).post(not_implemented),
        )
        .route(
            "/networks/{network}/entries/{entry}",
            axum::routing::put(not_implemented).delete(not_implemented),
        )
        .route(
            "/networks/{network}/scans",
            get(not_implemented).post(not_implemented),
        )
        .route("/networks/{network}/scans/{scan}", get(not_implemented))
        .route("/intel/lookup", get(not_implemented))
        .route("/search", get(not_implemented))
}

/// Tabs left open on the Laravel site still send Inertia's XHR
/// (`X-Inertia: true`). A 409 with `X-Inertia-Location` makes Inertia load
/// that URL in full: the same page for a GET, the page the request came
/// from for anything else (its Referer, or the home page).
async fn reload_inertia_requests(
    axum::extract::State(state): axum::extract::State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    if !request.headers().contains_key("x-inertia") {
        return next.run(request).await;
    }
    let headers = request.headers();
    let location = if matches!(*request.method(), Method::GET | Method::HEAD) {
        let scheme = headers
            .get("x-forwarded-proto")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_else(|| state.config.app_url.split("://").next().unwrap_or("https"));
        let host = headers
            .get(header::HOST)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        let path = request
            .uri()
            .path_and_query()
            .map_or("/", |path| path.as_str());
        format!("{scheme}://{host}{path}")
    } else {
        headers
            .get(header::REFERER)
            .and_then(|value| value.to_str().ok())
            .filter(|referer| !is_foreign_url(referer, headers))
            .map_or_else(|| format!("{}/", state.config.app_url), str::to_owned)
    };
    let mut response = StatusCode::CONFLICT.into_response();
    if let Ok(location) = location.parse() {
        response
            .headers_mut()
            .insert("x-inertia-location", location);
    }
    response
}

/// Whether a URL (an `Origin` or `Referer`) names a host other than the
/// one serving the request; an opaque `null` origin counts as foreign.
fn is_foreign_url(url: &str, headers: &axum::http::HeaderMap) -> bool {
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let authority = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .and_then(|rest| rest.split('/').next())
        .unwrap_or_default();
    authority.is_empty() || !authority.eq_ignore_ascii_case(host)
}
