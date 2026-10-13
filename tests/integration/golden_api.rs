//! Every request recorded from the Laravel reference
//! (tests/fixtures/golden/api, scripts/laravel-reference/record.py),
//! replayed against the api on the same dataset: status, the headers
//! clients read, the body, and the rows it changed, under the
//! normalizations of `common/golden.rs`. Where FIXES.md changes the
//! answer, `fixed` names the old and the new status and the test that
//! pins the new behavior.

use axum::body::Body;
use axum::http::Method;
use serde_json::{Value, json};
use time::OffsetDateTime;

use crate::common::golden::{
    dataset_token, fixture_names, html_title, normalize_now, normalize_random, normalize_slug,
    read_json, sort_keys,
};
use crate::common::{HOST, TestApp, TestResponse, assert_json_eq, header_map};

/// A documented difference: Laravel's status, ours, and the FIXES.md test.
struct Fixed {
    laravel: u16,
    ours: u16,
    test: &'static str,
}

fn fixed(name: &str) -> Option<Fixed> {
    let (laravel, ours, test) = match name {
        "entries_update_clear_label" | "entries_update_v05_client" => (
            422,
            200,
            "intel_entry::updating_can_clear_the_label_and_color",
        ),
        "entries_update_foreign" => (
            200,
            404,
            "intel_entry::updating_an_entry_of_another_network_is_not_found",
        ),
        "entries_destroy_foreign" => (
            200,
            404,
            "intel_entry::deleting_an_entry_of_another_network_is_not_found",
        ),
        "access_update_foreign" => (
            200,
            404,
            "network_access::updating_an_access_of_another_network_is_not_found",
        ),
        "access_destroy_foreign" => (
            200,
            404,
            "network_access::revoking_an_access_of_another_network_is_not_found",
        ),
        "access_store_owner_row" => (
            201,
            403,
            "network_access::granting_access_to_the_owner_is_forbidden",
        ),
        "user_guest_html" | "bauth_guest_html" => (
            500,
            401,
            "api_auth::a_guest_without_a_json_accept_header_gets_401",
        ),
        "networks_store_no_accept" | "share_store_no_accept" => (
            302,
            422,
            "share::a_failed_validation_answers_422_without_a_json_accept_header",
        ),
        "tokens_revoke_non_numeric" => (
            500,
            404,
            "api_tokens::revoking_a_token_by_a_non_numeric_id_is_not_found",
        ),
        "bauth_non_numeric_network" => (
            500,
            403,
            "broadcasting_auth::a_malformed_network_id_is_forbidden",
        ),
        "bauth_missing_socket" | "bauth_bad_socket" => (
            500,
            403,
            "broadcasting_auth::a_malformed_socket_id_is_forbidden",
        ),
        "bauth_public_channel" | "bauth_presence_channel" => {
            (200, 403, "broadcasting_auth::a_presence_channel_is_refused")
        }
        "eve_callback_without_state" => (
            500,
            419,
            "eve_login::a_callback_with_a_wrong_state_has_expired",
        ),
        "networks_show_trailing_letters" => (
            200,
            404,
            "intel_network::an_id_with_trailing_letters_is_not_found",
        ),
        _ => return None,
    };
    Some(Fixed {
        laravel,
        ours,
        test,
    })
}

/// FIXES.md changes inside an otherwise equal answer.
fn patch_expected(name: &str, fixture: &mut Value) {
    if name == "entries_store_string_id" {
        // FIXES.md (deliberate): numbers are always numbers
        // (intel_entry::an_entity_id_sent_as_text_is_answered_as_a_number).
        fixture["response"]["json"]["entity_id"] = json!(90_001_012);
    }
    if name == "access_store_expiry_with_offset" {
        // FIXES.md: the moment is kept
        // (network_access::an_expiry_with_an_offset_keeps_its_moment).
        fixture["response"]["json"]["expires_at"] = json!("2027-06-01T08:00:00+00:00");
        for row in fixture["rows"]["network_access"]
            .as_array_mut()
            .expect("rows")
        {
            if row["expires_at"] == "2027-06-01 10:00:00" {
                row["expires_at"] = json!("2027-06-01 08:00:00");
            }
        }
    }
}

/// The api path that answers a recorded request: the pages' props come
/// from `/api/pages/*`.
fn api_path(request: &Value) -> String {
    let path = request["path"].as_str().expect("a path");
    if request["inertia"] == true {
        return if path == "/" {
            "/api/pages/home".to_owned()
        } else {
            format!("/api/pages{path}")
        };
    }
    if let Some(query) = path.strip_prefix("/eve/callback") {
        return format!("/api/pages/eve/callback{query}");
    }
    path.to_owned()
}

async fn send(app: &TestApp, request: &Value) -> TestResponse {
    let mut headers: Vec<(String, String)> = vec![
        ("host".into(), HOST.into()),
        (
            "user-agent".into(),
            "Telescope | https://eve-telescope.com".into(),
        ),
    ];
    if let Some(accept) = request["accept"].as_str() {
        headers.push(("accept".into(), accept.into()));
    }
    match &request["token"] {
        Value::Number(id) => headers.push((
            "authorization".into(),
            format!("Bearer {}", dataset_token(id.as_i64().expect("an id"))),
        )),
        Value::String(token) => headers.push(("authorization".into(), format!("Bearer {token}"))),
        _ => {}
    }
    if request["inertia"] == true {
        headers.push(("x-requested-with".into(), "XMLHttpRequest".into()));
    }
    let body = if let Some(form) = request["form"].as_object() {
        headers.push((
            "content-type".into(),
            "application/x-www-form-urlencoded".into(),
        ));
        let mut serializer = form_urlencoded::Serializer::new(String::new());
        for (key, value) in form {
            serializer.append_pair(key, value.as_str().expect("a form value"));
        }
        Body::from(serializer.finish())
    } else if let Some(raw) = request["raw"].as_str() {
        Body::from(raw.to_owned())
    } else if !request["body"].is_null() {
        headers.push(("content-type".into(), "application/json".into()));
        Body::from(request["body"].to_string())
    } else {
        if request["json_type"] == true && request["accept"] == "application/json" {
            headers.push(("content-type".into(), "application/json".into()));
        }
        Body::empty()
    };
    if let Some(extra) = request["headers"].as_object() {
        for (name, value) in extra {
            headers.push((
                name.to_lowercase(),
                value.as_str().expect("a header").into(),
            ));
        }
    }
    let pairs: Vec<(&str, &str)> = headers
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    let method: Method = request["method"]
        .as_str()
        .expect("a method")
        .parse()
        .expect("a method");
    app.request(method, &api_path(request), header_map(&pairs), body)
        .await
}

/// N4: the random values of one side of a scenario.
fn randoms(body: &Value) -> Vec<(String, &'static str)> {
    let mut found = Vec::new();
    if let Some(code) = body.get("code").and_then(Value::as_str)
        && body.get("url").is_some()
    {
        found.push((code.to_owned(), "<code>"));
    }
    if let Some(token) = body.get("token").and_then(Value::as_str) {
        found.push((token.to_owned(), "<token>"));
    }
    found
}

fn normalize_slugs(value: &Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, item)| {
                    let item = match (key.as_str(), item) {
                        ("slug", Value::String(slug)) => Value::String(normalize_slug(slug)),
                        _ => normalize_slugs(item),
                    };
                    (key.clone(), item)
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(normalize_slugs).collect()),
        other => other.clone(),
    }
}

/// Every normalization of one side: N3, N4.
fn normalize(value: &Value, from: i64, to: i64, randoms: &[(String, &'static str)]) -> Value {
    normalize_slugs(&normalize_random(&normalize_now(value, from, to), randoms))
}

/// New token rows have a random hash (N4).
fn normalize_token_hashes(rows: &Value, dataset_ids: i64) -> Value {
    let mut rows = rows.clone();
    for row in rows.as_array_mut().into_iter().flatten() {
        if row["id"].as_i64().unwrap_or(0) > dataset_ids {
            row["token"] = json!("<hash>");
        }
    }
    rows
}

async fn replay(name: &str) {
    let mut fixture = read_json(&format!("golden/api/{name}.json"));
    patch_expected(name, &mut fixture);
    let started = fixture["started_at"].as_i64().expect("a start");
    let finished = fixture["finished_at"].as_i64().expect("an end");
    let mut app =
        TestApp::with_dataset(OffsetDateTime::from_unix_timestamp(started).expect("a moment"))
            .await;
    // The reference's Reverb secret (scripts/laravel-reference/env), so
    // the channel signatures compare byte for byte.
    app.reconfigure(|config| config.pusher.secret = "reference-reverb-secret".to_owned());
    // The release GitHub answered when the fixtures were recorded.
    if let Some(version) = read_json("golden/meta.json")["app_version"].as_str() {
        app.github.latest(version);
    }
    let response = send(&app, &fixture["request"]).await;
    let expected = &fixture["response"];
    let recorded_status = expected["status"].as_u64().expect("a status") as u16;

    if let Some(fix) = fixed(name) {
        assert_eq!(recorded_status, fix.laravel, "the recording of {name}");
        assert_eq!(
            response.status.as_u16(),
            fix.ours,
            "{name} (FIXES.md, {}): {}",
            fix.test,
            response.text()
        );
        return;
    }
    if name == "page_home_stale_version" {
        // A tab of the Laravel site reloads: Laravel for its version
        // mismatch, the rewrite for any Inertia request (FIXES.md).
        assert_eq!(recorded_status, 409);
        response.assert_status(409);
        assert!(response.header("x-inertia-location").is_some());
        return;
    }
    assert_eq!(
        response.status.as_u16(),
        recorded_status,
        "{name}: {}",
        response.text()
    );

    // The headers clients read.
    let recorded_headers = expected["headers"].as_object().cloned().unwrap_or_default();
    for (header, value) in &recorded_headers {
        let value = value.as_str().unwrap_or_default();
        match header.as_str() {
            "location" if name.starts_with("eve_redirect") => {
                let ours = response.header("location").expect("a location");
                assert_eq!(
                    eve_location(ours, &app.sso.url(), crate::common::EVE_CLIENT_ID),
                    eve_location(value, "https://login.eveonline.com", "reference-client-id"),
                    "{name}"
                );
            }
            "location" | "allow" | "x-inertia-location" => {
                assert_eq!(response.header(header), Some(value), "{name}: {header}");
            }
            // N8: page props come from `/api/pages/*`, under the API's
            // CORS; the pages themselves (SvelteKit) send none.
            header
                if header.starts_with("access-control-")
                    && fixture["request"]["inertia"] != true =>
            {
                assert_eq!(response.header(header), Some(value), "{name}: {header}");
            }
            "vary" if value.contains("Access-Control") => {
                assert_eq!(response.header("vary"), Some(value), "{name}: vary");
            }
            _ => {}
        }
    }
    for header in [
        "access-control-allow-origin",
        "access-control-allow-methods",
        "allow",
    ] {
        if !recorded_headers.contains_key(header) && fixture["request"]["inertia"] != true {
            assert_eq!(response.header(header), None, "{name}: {header}");
        }
    }

    // The body.
    if let Some(recorded) = expected.get("json").filter(|json| !json.is_null()) {
        let (recorded, ours) =
            if fixture["request"]["inertia"] == true && recorded.get("props").is_some() {
                let mut props = recorded["props"].as_object().cloned().expect("props");
                props.shift_remove("errors");
                let mut ours = response.json().as_object().cloned().expect("props");
                ours.shift_remove("demoLoginEnabled");
                (Value::Object(props), Value::Object(ours))
            } else {
                (recorded.clone(), response.json())
            };
        let recorded_randoms = randoms(&recorded);
        let our_randoms = randoms(&ours);
        assert_json_eq(
            &normalize(&recorded, started, finished, &recorded_randoms),
            &normalize(&ours, started, started, &our_randoms),
        );
    } else {
        let text = expected["text"].as_str().unwrap_or_default();
        if text.is_empty() {
            assert!(response.body.is_empty(), "{name}: {}", response.text());
        } else if let Some(title) = html_title(text) {
            if name == "up_page" {
                assert!(response.text().contains("Application up"), "{name}");
            }
            if name.starts_with("eve_redirect") {
                // Laravel's redirect body; the browser follows the header.
                return;
            }
            let ours = html_title(&response.text()).unwrap_or_default();
            let expected_title = if title.starts_with("An Error Occurred: ") {
                title.trim_start_matches("An Error Occurred: ").to_owned()
            } else {
                title
            };
            assert_eq!(ours, expected_title, "{name}");
        }
    }

    // The rows.
    let recorded_rows = fixture["rows"].as_object().cloned().unwrap_or_default();
    let recorded_randoms = expected.get("json").map(randoms).unwrap_or_default();
    let our_randoms = if response.body.is_empty() {
        Vec::new()
    } else {
        serde_json::from_slice::<Value>(&response.body)
            .map(|body| randoms(&body))
            .unwrap_or_default()
    };
    for (table, rows) in recorded_rows {
        let mut ours = Value::Array(app.snapshot(&table).await);
        let mut recorded = rows.clone();
        if table == "personal_access_tokens" {
            recorded = normalize_token_hashes(&recorded, 10);
            ours = normalize_token_hashes(&ours, 10);
        }
        assert_json_eq(
            &sort_keys(&normalize(&recorded, started, finished, &recorded_randoms)),
            &sort_keys(&normalize(&ours, started, started, &our_randoms)),
        );
    }
}

/// N4, N5: `/eve`'s redirect without the SSO address, the client id and
/// the random state.
fn eve_location(location: &str, sso: &str, client_id: &str) -> String {
    let rest = location.strip_prefix(sso).expect("the SSO address");
    let rest = rest.replace(&format!("client_id={client_id}"), "client_id=<client>");
    match rest.split_once("&state=") {
        Some((before, state)) => {
            assert_eq!(state.len(), 40, "{location}");
            format!("{before}&state=<state>")
        }
        None => rest,
    }
}

macro_rules! golden {
    ($($name:ident),* $(,)?) => {
        $(
            #[tokio::test]
            async fn $name() {
                replay(stringify!($name)).await;
            }
        )*

        /// Every recorded fixture has its test above.
        #[test]
        fn every_fixture_is_replayed() {
            let tested = [$(stringify!($name)),*];
            for name in fixture_names("api") {
                assert!(tested.contains(&name.as_str()), "no golden test for {name}");
            }
        }
    };
}

golden!(
    access_destroy,
    access_destroy_by_member,
    access_destroy_foreign,
    access_destroy_owner,
    access_index,
    access_index_bravo,
    access_index_by_member,
    access_store_alliance,
    access_store_bad_expiry,
    access_store_bad_values,
    access_store_by_member,
    access_store_character,
    access_store_corporation,
    access_store_corporation_without_alliance,
    access_store_expiry_with_offset,
    access_store_invalid,
    access_store_owner_row,
    access_store_past_expiry,
    access_store_unknown_entity,
    access_store_upsert,
    access_store_upsert_clears_expiry,
    access_store_with_date_expiry,
    access_store_with_expiry,
    access_store_words_expiry,
    access_update,
    access_update_by_member,
    access_update_clear_expiry,
    access_update_expiry,
    access_update_foreign,
    access_update_invalid,
    access_update_missing,
    access_update_nothing,
    access_update_null_permission,
    access_update_owner,
    access_update_same,
    bauth_ally_bravo,
    bauth_bad_socket,
    bauth_corp_mate,
    bauth_get,
    bauth_guest,
    bauth_guest_html,
    bauth_json_body,
    bauth_lapsed,
    bauth_missing_channel,
    bauth_missing_network,
    bauth_missing_socket,
    bauth_no_accept,
    bauth_non_numeric_network,
    bauth_outsider,
    bauth_owner,
    bauth_presence_channel,
    bauth_public_channel,
    bauth_unknown_channel,
    bauth_viewer,
    cors_auth_answer,
    cors_error_answer,
    cors_preflight_api,
    cors_preflight_auth,
    cors_preflight_unknown,
    cors_simple,
    csrf_cookie,
    entries_destroy,
    entries_destroy_by_viewer,
    entries_destroy_foreign,
    entries_destroy_missing,
    entries_index,
    entries_index_bravo,
    entries_index_outsider,
    entries_patch,
    entries_patch_html,
    entries_store,
    entries_store_by_outsider,
    entries_store_by_viewer,
    entries_store_empty_strings,
    entries_store_invalid,
    entries_store_missing,
    entries_store_note_only,
    entries_store_same_entity_other_type,
    entries_store_string_id,
    entries_store_unicode,
    entries_store_upsert,
    entries_update,
    entries_update_by_viewer,
    entries_update_clear_label,
    entries_update_clear_notes,
    entries_update_foreign,
    entries_update_invalid,
    entries_update_missing,
    entries_update_name_and_color,
    entries_update_same,
    entries_update_v05_client,
    eve_callback_without_state,
    eve_redirect,
    eve_redirect_desktop,
    lookup_ally,
    lookup_invalid,
    lookup_lapsed,
    lookup_member,
    lookup_missing,
    lookup_not_array,
    lookup_viewer,
    networks_destroy,
    networks_destroy_by_manager,
    networks_destroy_by_outsider,
    networks_guest,
    networks_index_ally,
    networks_index_corp_mate,
    networks_index_lapsed,
    networks_index_member,
    networks_index_outsider,
    networks_index_owner,
    networks_index_stranger,
    networks_index_viewer,
    networks_patch,
    networks_show_ally,
    networks_show_bravo,
    networks_show_corp_mate,
    networks_show_lapsed,
    networks_show_letters,
    networks_show_missing,
    networks_show_missing_html,
    networks_show_outsider,
    networks_show_owner,
    networks_show_trailing_letters,
    networks_show_trailing_slash,
    networks_show_viewer,
    networks_store,
    networks_store_empty,
    networks_store_long,
    networks_store_missing,
    networks_store_no_accept,
    networks_store_number,
    networks_store_symbols,
    networks_store_unicode,
    networks_update,
    networks_update_by_manager,
    networks_update_by_member,
    networks_update_forbidden_and_invalid,
    networks_update_invalid,
    networks_update_same_name,
    page_home,
    page_home_stale_version,
    page_share,
    page_share_lower_case,
    page_share_many,
    page_share_missing,
    page_share_trailing_slash,
    page_share_unicode,
    scans_index,
    scans_index_bravo,
    scans_index_empty,
    scans_index_outsider,
    scans_index_page_2,
    scans_index_page_99,
    scans_index_page_letters,
    scans_index_page_zero,
    scans_show,
    scans_show_foreign,
    scans_show_missing,
    scans_show_outsider,
    scans_show_outsider_foreign,
    scans_store_blank_text,
    scans_store_by_viewer,
    scans_store_dscan,
    scans_store_invalid,
    scans_store_local,
    scans_store_missing_text,
    search_all_categories,
    search_alliance_accent,
    search_alliance_name,
    search_array_query,
    search_character,
    search_character_accent,
    search_character_case,
    search_corporation_name,
    search_corporation_ticker,
    search_invalid_category,
    search_missing_query,
    search_no_local_match,
    search_short_query,
    search_wildcard_percent,
    search_wildcard_underscore,
    share_fetch,
    share_fetch_lower_case,
    share_fetch_missing,
    share_fetch_missing_html,
    share_fetch_unicode,
    share_store,
    share_store_bad_items,
    share_store_empty,
    share_store_missing,
    share_store_no_accept,
    share_store_not_array,
    share_store_too_many,
    storage_get,
    storage_put,
    tokens_create,
    tokens_create_invalid,
    tokens_create_long,
    tokens_create_number,
    tokens_revoke_current,
    tokens_revoke_foreign,
    tokens_revoke_missing,
    tokens_revoke_non_numeric,
    tokens_revoke_own,
    unknown_api_html,
    unknown_api_json,
    unknown_page,
    up_json,
    up_page,
    user_bad_token,
    user_expired_token,
    user_guest_html,
    user_guest_json,
    user_show,
    user_show_second_token,
    user_show_unicode,
    user_token_unknown_id,
    user_token_with_wrong_id,
    user_token_without_id,
    user_trailing_slash,
);
