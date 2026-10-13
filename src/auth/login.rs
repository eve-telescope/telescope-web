//! The EVE login (EveAuthController, EveAuthService) and the local demo
//! login.
//!
//! `/eve` (`?desktop` for the desktop app) sends the browser to EVE SSO
//! with a random state, kept with the kind of login in the
//! `telescope_sso` cookie for this attempt alone (FIXES.md). The callback
//! page's props (`/api/pages/eve/callback`) check the state once (419
//! "Page Expired" otherwise, FIXES.md), read the character from SSO,
//! refresh its corporation and alliance from ESI and store the pilot.
//! A desktop login answers a new "Telescope Desktop" token for the token
//! page; a website login starts a session and goes home.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::Json;
use axum::extract::{RawQuery, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::cookie::{Cookie, SameSite};
use serde_json::json;
use time::{Duration, OffsetDateTime};

use super::session::{self, cookie_value};
use super::sso::Character;
use super::tokens::{self, random_string};
use crate::affiliations;
use crate::input::Input;
use crate::models::{USER_COLUMNS, User};
use crate::server::AppState;
use crate::server::errors;
use crate::server::pages::shared_props;

pub const STATE_COOKIE: &str = "telescope_sso";
/// How long a login may take, as Laravel's session lasted.
const STATE_LIFETIME: Duration = Duration::hours(2);
/// The name of the tokens the desktop login hands out.
pub const DESKTOP_TOKEN_NAME: &str = "Telescope Desktop";

/// States already used, so a callback works once.
#[derive(Clone, Default)]
pub struct UsedStates(Arc<Mutex<HashMap<String, OffsetDateTime>>>);

impl UsedStates {
    /// Marks a state used; false when it was.
    fn take(&self, state: &str, now: OffsetDateTime) -> bool {
        let mut used = self.0.lock().expect("the used states");
        used.retain(|_, at| now - *at < STATE_LIFETIME);
        used.insert(state.to_owned(), now).is_none()
    }
}

fn state_cookie(value: String, secure: bool, max_age: Duration) -> HeaderValue {
    let cookie = Cookie::build((STATE_COOKIE, value))
        .path("/eve")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(secure)
        .max_age(max_age)
        .build();
    cookie.to_string().parse().expect("a cookie header")
}

fn clear_state(secure: bool) -> HeaderValue {
    state_cookie(String::new(), secure, Duration::ZERO)
}

fn home(state: &AppState) -> Response {
    let mut response = StatusCode::FOUND.into_response();
    response.headers_mut().insert(
        header::LOCATION,
        state
            .config
            .app_url
            .parse()
            .expect("APP_URL is a header value"),
    );
    response
}

/// `GET /eve`.
pub async fn redirect(State(state): State<AppState>, RawQuery(query): RawQuery) -> Response {
    let desktop = Input::from_query(query.as_deref()).has("desktop");
    let login_state = random_string(40);
    let mut response = StatusCode::FOUND.into_response();
    let location = state.sso.authorize_url(&login_state);
    response.headers_mut().insert(
        header::LOCATION,
        location.parse().expect("an authorize URL"),
    );
    let kind = if desktop { "desktop" } else { "website" };
    response.headers_mut().append(
        header::SET_COOKIE,
        state_cookie(
            format!("{login_state}.{kind}"),
            state.config.secure_cookies(),
            STATE_LIFETIME,
        ),
    );
    response
}

/// `/api/pages/eve/callback?code=...&state=...`.
pub async fn callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
) -> Response {
    let secure = state.config.secure_cookies();
    let mut response = match complete(&state, &headers, query.as_deref()).await {
        Ok(response) | Err(response) => response,
    };
    response
        .headers_mut()
        .append(header::SET_COOKIE, clear_state(secure));
    response
}

async fn complete(
    state: &AppState,
    headers: &HeaderMap,
    query: Option<&str>,
) -> Result<Response, Response> {
    let input = Input::from_query(query);
    // Cancelled on EVE's page (FIXES.md).
    if input.has("error") {
        return Ok(home(state));
    }
    let expired = || errors::http_error(StatusCode::from_u16(419).expect("419"), "Page Expired");
    let cookie = cookie_value(headers, STATE_COOKIE).ok_or_else(expired)?;
    let (expected, kind) = cookie.rsplit_once('.').ok_or_else(expired)?;
    let now = state.clock.now();
    let (Some(given), Some(code)) = (input.str("state"), input.str("code")) else {
        return Err(expired());
    };
    if given != expected || !state.used_states.take(expected, now) {
        return Err(expired());
    }
    let character = state
        .sso
        .character(code)
        .await
        .map_err(errors::server_error)?;
    let user = store_pilot(state, &character, now)
        .await
        .map_err(errors::server_error)?;
    finish(state, headers, &user, kind == "desktop").await
}

/// Stores the pilot with its corporation and alliance; when ESI did not
/// answer, the ones it had stay (FIXES.md).
async fn store_pilot(
    state: &AppState,
    character: &Character,
    now: OffsetDateTime,
) -> sqlx::Result<User> {
    let affiliation =
        affiliations::for_character(&state.pool, &state.esi, character.id, now).await?;
    let (corporation, alliance, known) = match affiliation {
        Some((corporation, alliance)) => (corporation, alliance, true),
        None => (None, None, false),
    };
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "insert into users (character_id, character_name, character_owner_hash, corporation_id,
            alliance_id, created_at, updated_at)
         values ($1, $2, $3, $4, $5, $6, $6)
         on conflict (character_id) do update set
            character_name = excluded.character_name,
            character_owner_hash = excluded.character_owner_hash,
            corporation_id = case when $7 then excluded.corporation_id else users.corporation_id end,
            alliance_id = case when $7 then excluded.alliance_id else users.alliance_id end,
            updated_at = case when (users.character_name, users.character_owner_hash,
                    users.corporation_id, users.alliance_id)
                is distinct from (excluded.character_name, excluded.character_owner_hash,
                    case when $7 then excluded.corporation_id else users.corporation_id end,
                    case when $7 then excluded.alliance_id else users.alliance_id end)
                then excluded.updated_at else users.updated_at end
         returning {USER_COLUMNS}"
    )))
    .bind(character.id)
    .bind(&character.name)
    .bind(&character.owner_hash)
    .bind(corporation)
    .bind(alliance)
    .bind(now)
    .bind(known)
    .fetch_one(&state.pool)
    .await
}

/// The end of a login: a desktop token for the token page, or a website
/// session and the home page.
async fn finish(
    state: &AppState,
    headers: &HeaderMap,
    user: &User,
    desktop: bool,
) -> Result<Response, Response> {
    let now = state.clock.now();
    if desktop {
        let (_, token) = tokens::create(&state.pool, user.id, DESKTOP_TOKEN_NAME, now)
            .await
            .map_err(errors::server_error)?;
        let mut props = shared_props(state, headers).await?;
        props["token"] = json!(token);
        props["deepLink"] = json!(format!("telescope://auth?token={token}"));
        props["characterName"] = json!(user.character_name);
        return Ok(Json(props).into_response());
    }
    let token = session::start(&state.pool, user.id, now)
        .await
        .map_err(errors::server_error)?;
    let cookie = session::cookie(
        token,
        state.config.session_lifetime_days,
        state.config.secure_cookies(),
    );
    let mut response = home(state);
    response.headers_mut().append(
        header::SET_COOKIE,
        cookie.to_string().parse().expect("a cookie header"),
    );
    Ok(response)
}

/// `/api/pages/demo-login[?desktop]`: the end of a login for the demo
/// pilot, without EVE SSO, when DEMO_ENABLED (FIXES.md, deliberate
/// changes); not found otherwise.
pub async fn demo(
    State(state): State<AppState>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
) -> Result<Response, Response> {
    if !state.config.demo.enabled {
        return Err(errors::http_error(
            StatusCode::NOT_FOUND,
            "The route api/pages/demo-login could not be found.",
        ));
    }
    let desktop = Input::from_query(query.as_deref()).has("desktop");
    let user = crate::demo::pilot(
        &state.pool,
        state.clock.now(),
        state.config.demo.character_id,
    )
    .await
    .map_err(errors::server_error)?;
    finish(&state, &headers, &user, desktop).await
}
