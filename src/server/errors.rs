//! Error answers. JSON for clients that ask for it (`Accept:
//! application/json`, as every desktop client sends), the minimal error
//! page otherwise, the way Laravel's exception handler answered.

use axum::Json;
use axum::extract::Request;
use axum::http::{HeaderMap, Method, StatusCode, Uri, header};
use axum::middleware::Next;
use axum::response::{Html, IntoResponse, Response};
use serde_json::json;

/// The minimal error page with the status and its message.
const ERROR_PAGE: &str = include_str!("error.html");

/// The message of a status, as the error pages show it.
pub fn message(status: StatusCode) -> &'static str {
    match status.as_u16() {
        401 => "Unauthorized",
        402 => "Payment Required",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        419 => "Page Expired",
        429 => "Too Many Requests",
        503 => "Service Unavailable",
        _ => "Server Error",
    }
}

/// `{"message": ...}` with the status, always JSON.
pub fn json_error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "message": message }))).into_response()
}

pub fn error_page(status: StatusCode) -> Response {
    let page = ERROR_PAGE
        .replace("{{status}}", status.as_str())
        .replace("{{message}}", message(status));
    (status, Html(page)).into_response()
}

/// Marks an error answer that becomes the error page for a client that
/// does not ask for JSON (see `negotiate`).
#[derive(Clone, Copy)]
struct PageForBrowsers;

/// An error from the exception handler: `{"message": ...}` for JSON
/// clients, the error page for the others.
pub fn http_error(status: StatusCode, message: &str) -> Response {
    let mut response = json_error(status, message);
    response.extensions_mut().insert(PageForBrowsers);
    response
}

/// A route parameter that names no row: Laravel's model binding message.
pub fn model_not_found(model: &str, id: &str) -> Response {
    http_error(
        StatusCode::NOT_FOUND,
        &format!("No query results for model [App\\Models\\{model}] {id}"),
    )
}

/// A policy said no.
pub fn forbidden() -> Response {
    http_error(StatusCode::FORBIDDEN, "This action is unauthorized.")
}

/// No valid token. Always JSON: Laravel answered a client that did not
/// ask for JSON with a 500, looking for a login page the site never had
/// (FIXES.md).
pub fn unauthenticated() -> Response {
    json_error(StatusCode::UNAUTHORIZED, "Unauthenticated.")
}

/// Any failure the client cannot fix: logged, answered with the generic
/// message.
pub fn server_error(error: impl std::fmt::Display) -> Response {
    tracing::error!(%error, "request failed");
    http_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        message(StatusCode::INTERNAL_SERVER_ERROR),
    )
}

/// Laravel's route path in messages: without the leading slash, `/` for
/// the root.
fn route_path(uri: &Uri) -> String {
    let path = uri.path().trim_start_matches('/');
    if path.is_empty() {
        "/".to_owned()
    } else {
        path.to_owned()
    }
}

/// Unknown paths.
pub async fn not_found(uri: Uri) -> Response {
    http_error(
        StatusCode::NOT_FOUND,
        &format!("The route {} could not be found.", route_path(&uri)),
    )
}

/// A known path with another method. The supported methods come from the
/// `Allow` header the router adds (`negotiate` writes them into the
/// message).
pub async fn method_not_allowed() -> Response {
    http_error(StatusCode::METHOD_NOT_ALLOWED, "")
}

/// The answer of every route that is registered but not ported yet.
pub async fn not_implemented() -> Response {
    json_error(StatusCode::NOT_IMPLEMENTED, "Not Implemented")
}

/// Whether the client asked for JSON: the most wanted type of its
/// `Accept` header is JSON, or it is an XHR that takes anything
/// (Laravel's `expectsJson`).
pub fn expects_json(headers: &HeaderMap) -> bool {
    let accept = headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let mut types: Vec<(f32, usize, String)> = accept
        .split(',')
        .enumerate()
        .filter_map(|(index, part)| {
            let mut pieces = part.split(';');
            let media = pieces.next()?.trim().to_ascii_lowercase();
            if media.is_empty() {
                return None;
            }
            let quality = pieces
                .filter_map(|parameter| parameter.trim().strip_prefix("q="))
                .find_map(|q| q.parse::<f32>().ok())
                .unwrap_or(1.0);
            Some((quality, index, media))
        })
        .collect();
    types.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let first = types.first().map(|(_, _, media)| media.as_str());
    let wants_json = first.is_some_and(|media| media.contains("/json") || media.contains("+json"));
    let xhr = headers
        .get("x-requested-with")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == "XMLHttpRequest");
    let accepts_anything = matches!(first, None | Some("*/*") | Some("*"));
    wants_json || (xhr && accepts_anything)
}

/// Renders the error answers for the client: the error page unless it
/// asked for JSON, and the supported methods of a 405 in Laravel's words.
pub async fn negotiate(request: Request, next: Next) -> Response {
    let wants_json = expects_json(request.headers());
    let method = request.method().clone();
    let uri = request.uri().clone();
    let mut response = next.run(request).await;
    if response.status() == StatusCode::METHOD_NOT_ALLOWED {
        response = method_not_allowed_answer(&method, &uri, response);
    }
    if response.extensions().get::<PageForBrowsers>().is_some() && !wants_json {
        let headers = response.headers().clone();
        let mut page = error_page(response.status());
        if let Some(allow) = headers.get(header::ALLOW) {
            page.headers_mut().insert(header::ALLOW, allow.clone());
        }
        return page;
    }
    response
}

fn method_not_allowed_answer(method: &Method, uri: &Uri, response: Response) -> Response {
    let methods: Vec<String> = response
        .headers()
        .get(header::ALLOW)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .split(',')
        .map(|method| method.trim().to_owned())
        .filter(|method| !method.is_empty())
        .collect();
    let allow = methods.join(", ");
    let mut answer = http_error(
        StatusCode::METHOD_NOT_ALLOWED,
        &format!(
            "The {method} method is not supported for route {}. Supported methods: {allow}.",
            route_path(uri)
        ),
    );
    if let Ok(value) = allow.parse() {
        answer.headers_mut().insert(header::ALLOW, value);
    }
    answer
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accept(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::ACCEPT, value.parse().expect("a header"));
        headers
    }

    #[test]
    fn reads_the_most_wanted_type() {
        assert!(expects_json(&accept("application/json")));
        assert!(expects_json(&accept("application/vnd.api+json")));
        assert!(expects_json(&accept("text/html;q=0.5, application/json")));
        assert!(!expects_json(&accept("text/html, application/json")));
        assert!(!expects_json(&accept("*/*")));
        assert!(!expects_json(&HeaderMap::new()));
    }

    #[test]
    fn an_xhr_taking_anything_gets_json() {
        let mut headers = accept("*/*");
        headers.insert(
            "x-requested-with",
            "XMLHttpRequest".parse().expect("a header"),
        );
        assert!(expects_json(&headers));
        headers.insert(header::ACCEPT, "text/html".parse().expect("a header"));
        assert!(!expects_json(&headers));
    }
}
