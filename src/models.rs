//! Rows the API answers with, serialized as the Eloquent models were:
//! columns in table order, hidden ones left out, timestamps in the model
//! form (`dates::model`).

use serde_json::{Value, json};
use time::OffsetDateTime;

use crate::dates;

/// The PHP class names Eloquent stored as morph types, and the plain
/// types the API speaks.
pub const USER_CLASS: &str = "App\\Models\\User";
pub const CORPORATION_CLASS: &str = "App\\Models\\Corporation";
pub const ALLIANCE_CLASS: &str = "App\\Models\\Alliance";

/// The plain type of a morph class.
pub fn plain_type(class: &str) -> &str {
    match class {
        USER_CLASS => "character",
        CORPORATION_CLASS => "corporation",
        ALLIANCE_CLASS => "alliance",
        other => other,
    }
}

/// The morph class of a plain type.
pub fn morph_class(plain: &str) -> Option<&'static str> {
    match plain {
        "character" => Some(USER_CLASS),
        "corporation" => Some(CORPORATION_CLASS),
        "alliance" => Some(ALLIANCE_CLASS),
        _ => None,
    }
}

/// A user: one EVE character. The owner hash stays hidden.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub character_id: i64,
    pub character_name: String,
    pub corporation_id: Option<i64>,
    pub alliance_id: Option<i64>,
    pub last_active_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// The columns `User` reads.
pub const USER_COLUMNS: &str = "id, character_id, character_name, corporation_id, alliance_id, last_active_at, created_at, updated_at";

impl User {
    pub fn json(&self) -> Value {
        json!({
            "id": self.id,
            "character_id": self.character_id,
            "character_name": self.character_name,
            "corporation_id": self.corporation_id,
            "alliance_id": self.alliance_id,
            "last_active_at": self.last_active_at.map(dates::model),
            "created_at": dates::model(self.created_at),
            "updated_at": dates::model(self.updated_at),
        })
    }

    /// The ids a network access row may name for this user: the
    /// character, the corporation and the alliance.
    pub fn accessible_ids(&self) -> Vec<i64> {
        let mut ids = vec![self.character_id];
        ids.extend(self.corporation_id);
        ids.extend(self.alliance_id);
        ids.dedup();
        ids
    }

    pub async fn find(pool: &sqlx::PgPool, id: i64) -> sqlx::Result<Option<User>> {
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "select {USER_COLUMNS} from users where id = $1"
        )))
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_character(
        pool: &sqlx::PgPool,
        character_id: i64,
    ) -> sqlx::Result<Option<User>> {
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "select {USER_COLUMNS} from users where character_id = $1"
        )))
        .bind(character_id)
        .fetch_optional(pool)
        .await
    }
}

/// `{id, name, ticker}` of a corporation or alliance (AffiliationData).
pub fn affiliation(id: Option<i64>, name: Option<String>, ticker: Option<String>) -> Value {
    match (id, name, ticker) {
        (Some(id), Some(name), Some(ticker)) => json!({ "id": id, "name": name, "ticker": ticker }),
        _ => Value::Null,
    }
}
