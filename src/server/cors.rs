//! CORS on `/api/*` and `/broadcasting/auth`, the paths of
//! `config/cors.php`: every origin, method and header, no credentials,
//! `max-age` 0. The Tauri webview of the desktop clients v0.2.1 to v0.4.0
//! calls `/broadcasting/auth` cross-origin with a bearer token, so the
//! preflight must keep answering as it did.

use axum::extract::Request;
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// Whether a path is one CORS applies to (`api/*`, `broadcasting/auth`).
pub fn applies_to(path: &str) -> bool {
    path.starts_with("/api/") || path == "/broadcasting/auth"
}

fn is_preflight(method: &Method, headers: &HeaderMap) -> bool {
    method == Method::OPTIONS && headers.contains_key(header::ACCESS_CONTROL_REQUEST_METHOD)
}

pub async fn handle(request: Request, next: Next) -> Response {
    let path = request.uri().path().trim_end_matches('/');
    if !applies_to(path) {
        return next.run(request).await;
    }
    if is_preflight(request.method(), request.headers()) {
        return preflight(request.headers());
    }
    let options = request.method() == Method::OPTIONS;
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    if options {
        headers.append(
            header::VARY,
            HeaderValue::from_static("Access-Control-Request-Method"),
        );
    }
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    response
}

/// The preflight answer: the requested method and headers echoed back.
fn preflight(request: &HeaderMap) -> Response {
    let mut response = StatusCode::NO_CONTENT.into_response();
    let method = request
        .get(header::ACCESS_CONTROL_REQUEST_METHOD)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_uppercase();
    let requested_headers = request
        .get(header::ACCESS_CONTROL_REQUEST_HEADERS)
        .cloned()
        .unwrap_or_else(|| HeaderValue::from_static(""));
    let headers = response.headers_mut();
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    headers.insert(
        header::VARY,
        HeaderValue::from_static("Access-Control-Request-Method, Access-Control-Request-Headers"),
    );
    if let Ok(method) = HeaderValue::from_str(&method) {
        headers.insert(header::ACCESS_CONTROL_ALLOW_METHODS, method);
    }
    headers.insert(header::ACCESS_CONTROL_ALLOW_HEADERS, requested_headers);
    headers.insert(
        header::ACCESS_CONTROL_MAX_AGE,
        HeaderValue::from_static("0"),
    );
    response
}
