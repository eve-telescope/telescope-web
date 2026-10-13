//! Finding characters, corporations and alliances by name or ticker, with
//! the shape the desktop client deserializes: only local rows, matched
//! without case and accents.

use serde_json::{Value, json};

use crate::common::factories::{NewUser, User};
use crate::common::{TestApp, assert_json_eq};

struct Setup {
    app: TestApp,
    user: User,
    token: String,
}

/// A pilot in Karmafleet and Goonswarm.
async fn setup() -> Setup {
    let app = TestApp::new().await;
    app.create_alliance(99_000_001, "Goonswarm Federation", "CONDI")
        .await;
    app.create_corporation(98_000_001, "Karmafleet", "KARMA", Some(99_000_001))
        .await;
    let user = app
        .create_user(NewUser {
            character_name: Some("Test Pilot".to_owned()),
            corporation_id: Some(98_000_001),
            alliance_id: Some(99_000_001),
            ..NewUser::default()
        })
        .await;
    let token = app.token(&user).await;
    Setup { app, user, token }
}

fn karmafleet() -> Value {
    json!({ "id": 98_000_001, "name": "Karmafleet", "ticker": "KARMA" })
}

fn goonswarm() -> Value {
    json!({ "id": 99_000_001, "name": "Goonswarm Federation", "ticker": "CONDI" })
}

#[tokio::test]
async fn searches_characters_by_name() {
    let s = setup().await;
    let response = s
        .app
        .get_json(&s.token, "/api/search?query=Test+Pilot&category=character")
        .await;
    response.assert_status(200);
    assert_json_eq(
        &json!([{
            "id": s.user.character_id,
            "name": "Test Pilot",
            "category": "character",
            "ticker": null,
            "corporation": karmafleet(),
            "alliance": goonswarm(),
        }]),
        &response.json(),
    );
}

#[tokio::test]
async fn searches_corporations_by_name() {
    let s = setup().await;
    let response = s
        .app
        .get_json(&s.token, "/api/search?query=Karma&category=corporation")
        .await;
    response.assert_status(200);
    assert_json_eq(
        &json!([{
            "id": 98_000_001,
            "name": "Karmafleet",
            "category": "corporation",
            "ticker": "KARMA",
            "corporation": null,
            "alliance": goonswarm(),
        }]),
        &response.json(),
    );
}

#[tokio::test]
async fn searches_corporations_by_ticker() {
    let s = setup().await;
    s.app
        .create_corporation(98_000_002, "Other Corp", "OTHER", None)
        .await;
    let response = s
        .app
        .get_json(&s.token, "/api/search?query=KARMA&category=corporation")
        .await;
    response.assert_status(200);
    let body = response.json();
    assert_eq!(body.as_array().map(Vec::len), Some(1));
    assert_eq!(body[0]["id"], 98_000_001);
}

#[tokio::test]
async fn searches_alliances_by_name() {
    let s = setup().await;
    let response = s
        .app
        .get_json(&s.token, "/api/search?query=Goon&category=alliance")
        .await;
    response.assert_status(200);
    assert_json_eq(
        &json!([{
            "id": 99_000_001,
            "name": "Goonswarm Federation",
            "category": "alliance",
            "ticker": "CONDI",
            "corporation": null,
            "alliance": null,
        }]),
        &response.json(),
    );
}

#[tokio::test]
async fn searches_all_categories_when_no_category_specified() {
    let s = setup().await;
    let response = s.app.get_json(&s.token, "/api/search?query=Test").await;
    response.assert_status(200);
    let body = response.json();
    assert_eq!(body.as_array().map(Vec::len), Some(1));
    assert_eq!(body[0]["category"], "character");
    assert_eq!(body[0]["name"], "Test Pilot");
}

#[tokio::test]
async fn returns_results_from_multiple_categories() {
    let s = setup().await;
    s.app
        .create_corporation(98_000_002, "Test Corporation", "TEST", None)
        .await;
    let response = s.app.get_json(&s.token, "/api/search?query=Test").await;
    response.assert_status(200);
    let categories: Vec<Value> = response
        .json()
        .as_array()
        .expect("results")
        .iter()
        .map(|result| result["category"].clone())
        .collect();
    assert_eq!(categories, vec![json!("character"), json!("corporation")]);
}

#[tokio::test]
async fn requires_query_parameter() {
    let s = setup().await;
    let response = s.app.get_json(&s.token, "/api/search").await;
    response.assert_status(422);
    assert_json_eq(
        &json!({
            "message": "The query field is required.",
            "errors": { "query": ["The query field is required."] },
        }),
        &response.json(),
    );
}

#[tokio::test]
async fn requires_query_to_be_at_least_2_characters() {
    let s = setup().await;
    let response = s.app.get_json(&s.token, "/api/search?query=a").await;
    response.assert_status(422);
    assert_json_eq(
        &json!({
            "message": "The query field must be at least 2 characters.",
            "errors": { "query": ["The query field must be at least 2 characters."] },
        }),
        &response.json(),
    );
}

#[tokio::test]
async fn rejects_invalid_category() {
    let s = setup().await;
    let response = s
        .app
        .get_json(&s.token, "/api/search?query=test&category=invalid")
        .await;
    response.assert_status(422);
    assert_json_eq(
        &json!({
            "message": "The selected category is invalid.",
            "errors": { "category": ["The selected category is invalid."] },
        }),
        &response.json(),
    );
}

#[tokio::test]
async fn never_returns_fields_that_would_break_rust_deserialization() {
    let s = setup().await;
    let response = s
        .app
        .get_json(&s.token, "/api/search?query=Test+Pilot&category=character")
        .await;
    response.assert_status(200);
    for result in response.json().as_array().expect("results") {
        assert!(result["id"].is_i64());
        assert!(result["name"].is_string());
        assert!(
            ["character", "corporation", "alliance"]
                .contains(&result["category"].as_str().unwrap_or(""))
        );
        let object = result.as_object().expect("an object");
        for key in ["ticker", "corporation", "alliance"] {
            assert!(object.contains_key(key), "{key}");
        }
        for key in ["corporation", "alliance"] {
            if !result[key].is_null() {
                assert!(result[key]["id"].is_i64());
                assert!(result[key]["name"].is_string());
                assert!(result[key]["ticker"].is_string());
            }
        }
    }
}

/// The search reads only local rows, so a miss is answered without
/// asking ESI.
#[tokio::test]
async fn a_query_without_local_matches_asks_nobody() {
    let s = setup().await;
    let response = s
        .app
        .get_json(&s.token, "/api/search?query=Nobody+Here")
        .await;
    response.assert_status(200);
    assert_json_eq(&json!([]), &response.json());
    assert!(s.app.esi.requests().is_empty());
}

/// Matching ignores case and accents; `%` and `_` in a query stay
/// wildcards.
#[tokio::test]
async fn matching_ignores_case_and_accents() {
    let s = setup().await;
    s.app
        .create_alliance(99_000_002, "Fédération Brûlée", "FEU")
        .await;
    s.app
        .create_corporation(98_000_002, "100% Pure_Corp", "PURE", None)
        .await;
    let accents = s
        .app
        .get_json(
            &s.token,
            "/api/search?query=FEDERATION+BRULEE&category=alliance",
        )
        .await;
    assert_eq!(accents.json()[0]["id"], 99_000_002);
    let reverse = s
        .app
        .get_json(
            &s.token,
            "/api/search?query=f%C3%A9d%C3%A9&category=alliance",
        )
        .await;
    let ids: Vec<Value> = reverse
        .json()
        .as_array()
        .expect("results")
        .iter()
        .map(|result| result["id"].clone())
        .collect();
    assert_eq!(ids, vec![json!(99_000_001), json!(99_000_002)]);
    let wildcard = s
        .app
        .get_json(&s.token, "/api/search?query=%25%25&category=corporation")
        .await;
    let ids: Vec<Value> = wildcard
        .json()
        .as_array()
        .expect("results")
        .iter()
        .map(|result| result["id"].clone())
        .collect();
    assert_eq!(ids, vec![json!(98_000_001), json!(98_000_002)]);
    let underscore = s
        .app
        .get_json(&s.token, "/api/search?query=Pure_&category=corporation")
        .await;
    assert_eq!(underscore.json()[0]["name"], "100% Pure_Corp");
}

/// At most 15 results per category, in id order.
#[tokio::test]
async fn returns_at_most_15_per_category() {
    let s = setup().await;
    for i in 0..20 {
        s.app
            .create_corporation(98_100_000 + i, &format!("Bulk Corp {i}"), "BULK", None)
            .await;
    }
    let response = s
        .app
        .get_json(&s.token, "/api/search?query=Bulk&category=corporation")
        .await;
    let body = response.json();
    let results = body.as_array().expect("results");
    assert_eq!(results.len(), 15);
    assert_eq!(results[0]["id"], 98_100_000);
    assert_eq!(results[14]["id"], 98_100_014);
}

/// Characters match without case and accents too, and carry their
/// corporation without an alliance when it has none.
#[tokio::test]
async fn characters_match_without_case_and_accents() {
    let s = setup().await;
    s.app
        .create_corporation(98_000_005, "100% Pure_Corp", "PURE", None)
        .await;
    let jose = s
        .app
        .create_user(NewUser {
            character_name: Some("José Ørsted".to_owned()),
            corporation_id: Some(98_000_005),
            ..NewUser::default()
        })
        .await;
    let accent = s
        .app
        .get_json(&s.token, "/api/search?query=jose&category=character")
        .await;
    accent.assert_status(200);
    assert_json_eq(
        &json!([{
            "id": jose.character_id,
            "name": "José Ørsted",
            "category": "character",
            "ticker": null,
            "corporation": { "id": 98_000_005, "name": "100% Pure_Corp", "ticker": "PURE" },
            "alliance": null,
        }]),
        &accent.json(),
    );
    let case = s
        .app
        .get_json(&s.token, "/api/search?query=tEST+pilot&category=character")
        .await;
    assert_eq!(case.json().as_array().map(Vec::len), Some(1));
    assert_eq!(case.json()[0]["id"], s.user.character_id);
}

/// Without a category the results come as characters, corporations, then
/// alliances.
#[tokio::test]
async fn all_categories_come_in_order() {
    let s = setup().await;
    s.app
        .create_alliance(99_000_002, "Test Alliance Please Ignore", "TEST")
        .await;
    s.app
        .create_corporation(98_000_002, "Test Corp", "TSTC", Some(99_000_002))
        .await;
    let response = s.app.get_json(&s.token, "/api/search?query=Test").await;
    response.assert_status(200);
    let test_alliance =
        json!({ "id": 99_000_002, "name": "Test Alliance Please Ignore", "ticker": "TEST" });
    assert_json_eq(
        &json!([
            {
                "id": s.user.character_id,
                "name": "Test Pilot",
                "category": "character",
                "ticker": null,
                "corporation": karmafleet(),
                "alliance": goonswarm(),
            },
            {
                "id": 98_000_002,
                "name": "Test Corp",
                "category": "corporation",
                "ticker": "TSTC",
                "corporation": null,
                "alliance": test_alliance,
            },
            {
                "id": 99_000_002,
                "name": "Test Alliance Please Ignore",
                "category": "alliance",
                "ticker": "TEST",
                "corporation": null,
                "alliance": null,
            },
        ]),
        &response.json(),
    );
}

#[tokio::test]
async fn a_query_must_be_one_string() {
    let s = setup().await;
    let response = s.app.get_json(&s.token, "/api/search?query[]=ab").await;
    response.assert_status(422);
    assert_json_eq(
        &json!({
            "message": "The query field must be a string. (and 1 more error)",
            "errors": {
                "query": [
                    "The query field must be a string.",
                    "The query field must be at least 2 characters.",
                ],
            },
        }),
        &response.json(),
    );
}
