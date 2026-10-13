//! Local stand-ins for the services the app calls: ESI, EVE SSO (token
//! endpoint and JWKS) and GitHub's releases API. Each listens on an
//! ephemeral port, records every request and can be told to fail. They
//! answer like the real services, with data the test sets.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};

/// One request a mock received.
#[derive(Debug, Clone)]
pub struct Recorded {
    pub method: Method,
    pub path: String,
    pub query: Option<String>,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

impl Recorded {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|value| value.to_str().ok())
    }

    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).expect("a JSON body")
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

async fn listen(router: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind a mock");
    let address = listener.local_addr().expect("a mock address");
    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("serve a mock");
    });
    format!("http://{address}")
}

// ---------------------------------------------------------------------------
// ESI
// ---------------------------------------------------------------------------

#[derive(Default)]
struct EsiData {
    /// character id to (corporation id, alliance id).
    affiliations: HashMap<i64, (i64, Option<i64>)>,
    alliances: HashMap<i64, Value>,
    corporations: HashMap<i64, Value>,
    /// Paths that answer this status instead.
    failures: HashMap<String, u16>,
    /// Every path answers this status.
    fail_all: Option<u16>,
    requests: Vec<Recorded>,
}

/// ESI: character affiliations, alliances, corporations, and the search
/// and names endpoints the entity search no longer calls.
#[derive(Clone)]
pub struct MockEsi {
    url: String,
    data: Arc<Mutex<EsiData>>,
}

impl MockEsi {
    pub async fn start() -> Self {
        let data = Arc::new(Mutex::new(EsiData::default()));
        let router = Router::new()
            .route("/latest/characters/affiliation/", post(esi_affiliation))
            .route("/latest/alliances/{id}/", get(esi_alliance))
            .route("/latest/corporations/{id}/", get(esi_corporation))
            .fallback(esi_fallback)
            .with_state(data.clone());
        Self {
            url: listen(router).await,
            data,
        }
    }

    pub fn url(&self) -> String {
        self.url.clone()
    }

    /// A character's corporation and alliance, as the affiliation endpoint
    /// answers them.
    pub fn affiliation(&self, character_id: i64, corporation_id: i64, alliance_id: Option<i64>) {
        self.data
            .lock()
            .expect("ESI lock")
            .affiliations
            .insert(character_id, (corporation_id, alliance_id));
    }

    pub fn alliance(&self, id: i64, name: &str, ticker: &str) {
        self.data.lock().expect("ESI lock").alliances.insert(
            id,
            json!({
                "creator_corporation_id": 98_000_001,
                "creator_id": 90_000_001,
                "date_founded": "2010-06-01T05:36:00Z",
                "executor_corporation_id": 98_000_001,
                "name": name,
                "ticker": ticker,
            }),
        );
    }

    pub fn corporation(
        &self,
        id: i64,
        name: &str,
        ticker: &str,
        alliance_id: Option<i64>,
        member_count: i64,
    ) {
        let mut corporation = json!({
            "ceo_id": 90_000_001,
            "creator_id": 90_000_001,
            "member_count": member_count,
            "name": name,
            "tax_rate": 0.1,
            "ticker": ticker,
        });
        if let Some(alliance_id) = alliance_id {
            corporation["alliance_id"] = json!(alliance_id);
        }
        self.data
            .lock()
            .expect("ESI lock")
            .corporations
            .insert(id, corporation);
    }

    /// One path answers a status instead.
    pub fn fail_path(&self, path: &str, status: u16) {
        self.data
            .lock()
            .expect("ESI lock")
            .failures
            .insert(path.to_owned(), status);
    }

    /// Every path answers a status.
    pub fn fail_with(&self, status: u16) {
        self.data.lock().expect("ESI lock").fail_all = Some(status);
    }

    pub fn requests(&self) -> Vec<Recorded> {
        self.data.lock().expect("ESI lock").requests.clone()
    }

    /// The paths requested, in order.
    pub fn paths(&self) -> Vec<String> {
        self.requests()
            .into_iter()
            .map(|request| format!("{} {}", request.method, request.path))
            .collect()
    }
}

fn esi_failure(data: &EsiData, path: &str) -> Option<Response> {
    data.fail_all
        .or_else(|| data.failures.get(path).copied())
        .map(|status| {
            (
                StatusCode::from_u16(status).expect("a status"),
                Json(json!({ "error": "mock failure" })),
            )
                .into_response()
        })
}

async fn esi_affiliation(
    State(data): State<Arc<Mutex<EsiData>>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let mut data = data.lock().expect("ESI lock");
    data.requests.push(Recorded {
        method,
        path: uri.path().to_owned(),
        query: uri.query().map(str::to_owned),
        headers,
        body: body.to_vec(),
    });
    if let Some(failure) = esi_failure(&data, uri.path()) {
        return failure;
    }
    let ids: Vec<i64> = serde_json::from_slice(&body).unwrap_or_default();
    let answer: Vec<Value> = ids
        .iter()
        .filter_map(|id| {
            let (corporation_id, alliance_id) = data.affiliations.get(id)?;
            let mut row = json!({ "character_id": id, "corporation_id": corporation_id });
            if let Some(alliance_id) = alliance_id {
                row["alliance_id"] = json!(alliance_id);
            }
            Some(row)
        })
        .collect();
    Json(answer).into_response()
}

async fn esi_alliance(
    State(data): State<Arc<Mutex<EsiData>>>,
    Path(id): Path<String>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    let mut data = data.lock().expect("ESI lock");
    data.requests.push(Recorded {
        method,
        path: uri.path().to_owned(),
        query: uri.query().map(str::to_owned),
        headers,
        body: Vec::new(),
    });
    if let Some(failure) = esi_failure(&data, uri.path()) {
        return failure;
    }
    match id.parse().ok().and_then(|id: i64| data.alliances.get(&id)) {
        Some(alliance) => Json(alliance.clone()).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Alliance not found" })),
        )
            .into_response(),
    }
}

async fn esi_corporation(
    State(data): State<Arc<Mutex<EsiData>>>,
    Path(id): Path<String>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    let mut data = data.lock().expect("ESI lock");
    data.requests.push(Recorded {
        method,
        path: uri.path().to_owned(),
        query: uri.query().map(str::to_owned),
        headers,
        body: Vec::new(),
    });
    if let Some(failure) = esi_failure(&data, uri.path()) {
        return failure;
    }
    match id
        .parse()
        .ok()
        .and_then(|id: i64| data.corporations.get(&id))
    {
        Some(corporation) => Json(corporation.clone()).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Corporation not found" })),
        )
            .into_response(),
    }
}

/// Everything else, `/latest/search/` included, is gone like on the real
/// ESI: 404.
async fn esi_fallback(
    State(data): State<Arc<Mutex<EsiData>>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let mut data = data.lock().expect("ESI lock");
    data.requests.push(Recorded {
        method,
        path: uri.path().to_owned(),
        query: uri.query().map(str::to_owned),
        headers,
        body: body.to_vec(),
    });
    (StatusCode::NOT_FOUND, Json(json!({ "error": "Not found" }))).into_response()
}

// ---------------------------------------------------------------------------
// EVE SSO
// ---------------------------------------------------------------------------

/// The key id of the mock's signing key.
pub const SSO_KEY_ID: &str = "JWT-Signature-Key";

/// The character the mock SSO signs tokens for.
#[derive(Debug, Clone)]
pub struct SsoCharacter {
    pub id: i64,
    pub name: String,
    pub owner: String,
    pub issuer: String,
    /// Seconds from now until the token expires; negative is expired.
    pub expires_in: i64,
}

struct SsoData {
    character: SsoCharacter,
    /// The authorization code the token endpoint accepts.
    code: String,
    token_status: Option<u16>,
    jwks_status: Option<u16>,
    requests: Vec<Recorded>,
}

/// EVE SSO: the token endpoint (authorization code grant, basic auth)
/// and the JWKS the access token is checked against. Tokens are RS256
/// JWTs like EVE's, signed with a fixed test key
/// (tests/fixtures/sso/private-key.pem).
#[derive(Clone)]
pub struct MockSso {
    url: String,
    data: Arc<Mutex<SsoData>>,
}

impl MockSso {
    pub async fn start() -> Self {
        let data = Arc::new(Mutex::new(SsoData {
            character: SsoCharacter {
                id: 90_000_001,
                name: "Test Pilot".to_owned(),
                owner: "owner-hash-1".to_owned(),
                issuer: "https://login.eveonline.com".to_owned(),
                expires_in: 1199,
            },
            code: "valid-code".to_owned(),
            token_status: None,
            jwks_status: None,
            requests: Vec::new(),
        }));
        let router = Router::new()
            .route("/v2/oauth/token", post(sso_token))
            .route("/oauth/jwks", get(sso_jwks))
            .with_state(data.clone());
        Self {
            url: listen(router).await,
            data,
        }
    }

    pub fn url(&self) -> String {
        self.url.clone()
    }

    /// The character the next tokens are for.
    pub fn sign_in_as(&self, id: i64, name: &str, owner: &str) {
        let mut data = self.data.lock().expect("SSO lock");
        data.character.id = id;
        data.character.name = name.to_owned();
        data.character.owner = owner.to_owned();
    }

    pub fn issuer(&self, issuer: &str) {
        self.data.lock().expect("SSO lock").character.issuer = issuer.to_owned();
    }

    pub fn expires_in(&self, seconds: i64) {
        self.data.lock().expect("SSO lock").character.expires_in = seconds;
    }

    /// The code the token endpoint accepts.
    pub fn code(&self) -> String {
        self.data.lock().expect("SSO lock").code.clone()
    }

    pub fn fail_token_with(&self, status: u16) {
        self.data.lock().expect("SSO lock").token_status = Some(status);
    }

    pub fn fail_jwks_with(&self, status: u16) {
        self.data.lock().expect("SSO lock").jwks_status = Some(status);
    }

    pub fn requests(&self) -> Vec<Recorded> {
        self.data.lock().expect("SSO lock").requests.clone()
    }
}

/// A token signed with the test key for a character.
pub fn signed_access_token(character: &SsoCharacter) -> String {
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let claims = json!({
        "scp": [],
        "jti": "test-jti",
        "kid": SSO_KEY_ID,
        "sub": format!("CHARACTER:EVE:{}", character.id),
        "azp": super::EVE_CLIENT_ID,
        "tenant": "tranquility",
        "tier": "live",
        "region": "world",
        "aud": [super::EVE_CLIENT_ID, "EVE Online"],
        "name": character.name,
        "owner": character.owner,
        "exp": now + character.expires_in,
        "iat": now,
        "iss": character.issuer,
    });
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(SSO_KEY_ID.to_owned());
    let pem = std::fs::read(super::repo_path("tests/fixtures/sso/private-key.pem"))
        .expect("the test key");
    jsonwebtoken::encode(
        &header,
        &claims,
        &jsonwebtoken::EncodingKey::from_rsa_pem(&pem).expect("an RSA key"),
    )
    .expect("a signed token")
}

async fn sso_token(
    State(data): State<Arc<Mutex<SsoData>>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let mut data = data.lock().expect("SSO lock");
    data.requests.push(Recorded {
        method,
        path: uri.path().to_owned(),
        query: uri.query().map(str::to_owned),
        headers,
        body: body.to_vec(),
    });
    if let Some(status) = data.token_status {
        return (
            StatusCode::from_u16(status).expect("a status"),
            Json(json!({ "error": "invalid_request" })),
        )
            .into_response();
    }
    let form: HashMap<String, String> = form_urlencoded::parse(&body).into_owned().collect();
    if form.get("code") != Some(&data.code) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "invalid_grant", "error_description": "Authorization code not found" })),
        )
            .into_response();
    }
    Json(json!({
        "access_token": signed_access_token(&data.character),
        "expires_in": 1199,
        "token_type": "Bearer",
        "refresh_token": "test-refresh-token",
    }))
    .into_response()
}

async fn sso_jwks(
    State(data): State<Arc<Mutex<SsoData>>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    let mut data = data.lock().expect("SSO lock");
    data.requests.push(Recorded {
        method,
        path: uri.path().to_owned(),
        query: uri.query().map(str::to_owned),
        headers,
        body: Vec::new(),
    });
    if let Some(status) = data.jwks_status {
        return StatusCode::from_u16(status)
            .expect("a status")
            .into_response();
    }
    let modulus = std::fs::read_to_string(super::repo_path("tests/fixtures/sso/modulus.txt"))
        .expect("the test key's modulus");
    Json(json!({
        "keys": [
            {
                "alg": "RS256",
                "e": "AQAB",
                "kid": SSO_KEY_ID,
                "kty": "RSA",
                "n": modulus.trim(),
                "use": "sig",
            },
            {
                "alg": "ES256",
                "crv": "P-256",
                "kid": "8878a23f-b40d-4ae1-9bf6-e4d1f4f9c4f2",
                "kty": "EC",
                "use": "sig",
                "x": "ITcDYJ8WVpDO4QtZ169xXUt7GB1Y6-oMKIwJ3nK1tFU",
                "y": "ZVv1lb3S6s9Zk4_nGdVRVHUM0ZGnXuQVJEUNiNM3ZH0",
            },
        ],
        "SkipUnresolvedJsonWebKeys": true,
    }))
    .into_response()
}

// ---------------------------------------------------------------------------
// GitHub
// ---------------------------------------------------------------------------

struct GitHubData {
    tag: String,
    status: Option<u16>,
    delay: Option<Duration>,
    requests: Vec<Recorded>,
}

/// GitHub's latest release of the desktop app.
#[derive(Clone)]
pub struct MockGitHub {
    url: String,
    data: Arc<Mutex<GitHubData>>,
}

impl MockGitHub {
    pub async fn start() -> Self {
        let data = Arc::new(Mutex::new(GitHubData {
            tag: "v0.5.1".to_owned(),
            status: None,
            delay: None,
            requests: Vec::new(),
        }));
        let router = Router::new()
            .route(
                "/repos/eve-telescope/telescope-app/releases/latest",
                get(github_latest),
            )
            .with_state(data.clone());
        Self {
            url: listen(router).await,
            data,
        }
    }

    pub fn url(&self) -> String {
        self.url.clone()
    }

    pub fn latest(&self, tag: &str) {
        self.data.lock().expect("GitHub lock").tag = tag.to_owned();
    }

    pub fn fail_with(&self, status: u16) {
        self.data.lock().expect("GitHub lock").status = Some(status);
    }

    pub fn recover(&self) {
        self.data.lock().expect("GitHub lock").status = None;
    }

    /// Answers only after a delay.
    pub fn delay(&self, delay: Duration) {
        self.data.lock().expect("GitHub lock").delay = Some(delay);
    }

    pub fn requests(&self) -> Vec<Recorded> {
        self.data.lock().expect("GitHub lock").requests.clone()
    }
}

async fn github_latest(
    State(data): State<Arc<Mutex<GitHubData>>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    let (tag, status, delay) = {
        let mut data = data.lock().expect("GitHub lock");
        data.requests.push(Recorded {
            method,
            path: uri.path().to_owned(),
            query: uri.query().map(str::to_owned),
            headers,
            body: Vec::new(),
        });
        (data.tag.clone(), data.status, data.delay)
    };
    if let Some(delay) = delay {
        tokio::time::sleep(delay).await;
    }
    if let Some(status) = status {
        return (
            StatusCode::from_u16(status).expect("a status"),
            Json(json!({ "message": "API rate limit exceeded" })),
        )
            .into_response();
    }
    Json(json!({
        "url": "https://api.github.com/repos/eve-telescope/telescope-app/releases/1",
        "tag_name": tag,
        "name": tag,
        "draft": false,
        "prerelease": false,
    }))
    .into_response()
}
