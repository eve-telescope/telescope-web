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
pub mod pages;

use std::sync::Arc;

use axum::Router;
use axum::extract::{DefaultBodyLimit, Request};
use axum::http::{Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use sqlx::PgPool;
use tower_http::normalize_path::NormalizePath;

use crate::auth::login::UsedStates;
use crate::auth::sso::SsoClient;
use crate::clock::Clock;
use crate::config::Config;
use crate::esi::EsiClient;
use crate::github::LatestRelease;
use crate::pusher::Hub;

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
    pub esi: EsiClient,
    pub sso: SsoClient,
    pub github: LatestRelease,
    pub pusher: Hub,
    pub used_states: UsedStates,
}

impl AppState {
    pub fn new(pool: PgPool, config: Config, clock: Clock) -> Self {
        Self {
            pool,
            esi: EsiClient::new(&config.esi_base_url),
            sso: SsoClient::new(config.eve.clone()),
            github: LatestRelease::new(&config.github_api_url, clock.clone()),
            pusher: Hub::new(),
            used_states: UsedStates::default(),
            config: Arc::new(config),
            clock,
        }
    }
}

/// The whole app. A trailing slash reaches the same route, as it did.
/// CORS and the error pages wrap the routing, so they see every answer as
/// the router finished it (the `Allow` header of a 405 included).
pub fn router(state: AppState) -> Router {
    let routes = routes(state);
    let app = tower::ServiceBuilder::new()
        .layer(axum::middleware::from_fn(cors::handle))
        .layer(axum::middleware::from_fn(errors::negotiate))
        .service(NormalizePath::trim_trailing_slash(routes));
    Router::new().fallback_service(app)
}

fn routes(state: AppState) -> Router {
    use crate::auth::login;
    use crate::pusher::{auth, socket};
    Router::new()
        .route("/up", get(health::up))
        // The EVE login, the CSRF cookie nothing uses and the local disk,
        // where nothing is stored.
        .route("/eve", get(login::redirect))
        .route("/sanctum/csrf-cookie", get(csrf_cookie))
        .route("/storage/{*path}", get(storage).put(storage))
        // The realtime channel: subscription signatures and the socket.
        .route(
            "/broadcasting/auth",
            get(auth::authenticate).post(auth::authenticate),
        )
        .route("/app/{key}", get(socket::upgrade))
        .nest("/api", api_router())
        .fallback(errors::not_found)
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            reload_inertia_requests,
        ))
        .with_state(state)
}

/// The desktop clients' API, the props of the pages and the health check.
fn api_router() -> Router<AppState> {
    use crate::auth::login;
    use crate::intel::{access, entries, networks, scans, search};
    use crate::shares;
    Router::new()
        .route("/health", get(health::health))
        .route("/pages/home", get(pages::home))
        .route("/pages/s/{code}", get(shares::page))
        .route("/pages/eve/callback", get(login::callback))
        .route("/pages/demo-login", get(login::demo))
        .route("/user", get(user))
        .route("/share", post(shares::store))
        .route("/share/{code}", get(shares::fetch))
        .route("/tokens", post(tokens_store))
        .route("/tokens/{token}", delete(tokens_destroy))
        .route("/networks", get(networks::index).post(networks::store))
        .route(
            "/networks/{network}",
            get(networks::show)
                .put(networks::update)
                .delete(networks::destroy),
        )
        .route(
            "/networks/{network}/access",
            get(access::index).post(access::store),
        )
        .route(
            "/networks/{network}/access/{access}",
            put(access::update).delete(access::destroy),
        )
        .route(
            "/networks/{network}/entries",
            get(entries::index).post(entries::store),
        )
        .route(
            "/networks/{network}/entries/{entry}",
            put(entries::update).delete(entries::destroy),
        )
        .route(
            "/networks/{network}/scans",
            get(scans::index).post(scans::store),
        )
        .route("/networks/{network}/scans/{scan}", get(scans::show))
        .route("/intel/lookup", get(entries::lookup))
        .route("/search", get(search::search))
}

/// `GET /api/user`: the token's user.
async fn user(caller: crate::auth::tokens::ApiUser) -> Response {
    axum::Json(caller.user.json()).into_response()
}

/// `POST /api/tokens`: a new token for the caller, shown once.
async fn tokens_store(
    axum::extract::State(state): axum::extract::State<AppState>,
    caller: crate::auth::tokens::ApiUser,
    input: crate::input::Input,
) -> Result<Response, Response> {
    let now = state.clock.now();
    let mut validator = crate::validation::Validator::new(&input, now);
    validator.field("name").required().string().max(255);
    validator.finish()?;
    let name = input.str("name").expect("a validated name");
    let (id, token) = crate::auth::tokens::create(&state.pool, caller.user.id, name, now)
        .await
        .map_err(errors::server_error)?;
    Ok((
        StatusCode::CREATED,
        axum::Json(serde_json::json!({ "token": token, "name": name, "id": id })),
    )
        .into_response())
}

/// `DELETE /api/tokens/{tokenId}`: one of the caller's own tokens; the
/// same answer whether it existed or not. A non-numeric id is not found
/// (FIXES.md).
async fn tokens_destroy(
    axum::extract::State(state): axum::extract::State<AppState>,
    caller: crate::auth::tokens::ApiUser,
    axum::extract::Path(token): axum::extract::Path<String>,
) -> Result<Response, Response> {
    let Some(id) = crate::intel::route_id(&token) else {
        return Err(errors::http_error(StatusCode::NOT_FOUND, "Not Found"));
    };
    sqlx::query(
        "delete from personal_access_tokens
         where id = $1 and tokenable_type = 'App\\Models\\User' and tokenable_id = $2",
    )
    .bind(id)
    .bind(caller.user.id)
    .execute(&state.pool)
    .await
    .map_err(errors::server_error)?;
    Ok(axum::Json(serde_json::json!({ "message": "Token revoked" })).into_response())
}

/// `GET /sanctum/csrf-cookie`: 204, no cookies (FIXES.md, deliberate
/// changes).
async fn csrf_cookie() -> StatusCode {
    StatusCode::NO_CONTENT
}

/// The local disk's routes: nothing is stored there.
async fn storage() -> Response {
    errors::http_error(StatusCode::NOT_FOUND, "Not Found")
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
