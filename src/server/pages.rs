//! The props of the SvelteKit pages (`/api/pages/*`): the shared `name`, `auth.user` (the website
//! session's user) and `appVersion`, merged with each page's own.

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

use super::AppState;
use super::errors;
use crate::auth::session;

/// The props every page shares.
pub async fn shared_props(state: &AppState, headers: &HeaderMap) -> Result<Value, Response> {
    let user = session::user(
        &state.pool,
        headers,
        state.clock.now(),
        state.config.session_lifetime_days,
    )
    .await
    .map_err(errors::server_error)?;
    Ok(json!({
        "name": state.config.app_name,
        "auth": { "user": user.as_ref().map(crate::models::User::json) },
        "appVersion": state.github.version().await,
    }))
}

/// The landing page (Welcome), and whether the demo login is on.
pub async fn home(State(state): State<AppState>, headers: HeaderMap) -> Result<Response, Response> {
    let mut props = shared_props(&state, &headers).await?;
    props["demoLoginEnabled"] = json!(state.config.demo.enabled);
    Ok(Json(props).into_response())
}
