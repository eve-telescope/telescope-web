//! Test rows, written straight to the database. Times come from the app's clock. Character
//! ids count up from 90 000 001, so tests are repeatable.

use rand::RngExt;
use rand::distr::Alphanumeric;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

use super::TestApp;

pub const USER: &str = "App\\Models\\User";
pub const CORPORATION: &str = "App\\Models\\Corporation";
pub const ALLIANCE: &str = "App\\Models\\Alliance";

#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub character_id: i64,
    pub character_name: String,
    pub corporation_id: Option<i64>,
    pub alliance_id: Option<i64>,
}

impl User {
    /// The user model's JSON, for a user the factory made at `DEFAULT_NOW`.
    pub fn json(&self) -> Value {
        json!({
            "id": self.id,
            "character_id": self.character_id,
            "character_name": self.character_name,
            "corporation_id": self.corporation_id,
            "alliance_id": self.alliance_id,
            "last_active_at": null,
            "created_at": super::NOW_Z,
            "updated_at": super::NOW_Z,
        })
    }
}

/// A user to create; unset fields get factory defaults.
#[derive(Debug, Clone, Default)]
pub struct NewUser {
    pub character_id: Option<i64>,
    pub character_name: Option<String>,
    pub corporation_id: Option<i64>,
    pub alliance_id: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct NewEntry {
    pub entity_type: &'static str,
    pub entity_id: i64,
    pub entity_name: String,
    pub color: Option<String>,
    pub label: Option<String>,
    pub notes: Option<String>,
    pub added_by: i64,
}

pub fn random_string(length: usize) -> String {
    rand::rng()
        .sample_iter(Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

/// A plain text token in Sanctum's format for a row id: `<id>|<40 random><crc32b>`,
/// and the hash the table stores.
pub fn sanctum_token(id: i64) -> (String, String) {
    let entropy = random_string(40);
    let plain = format!("{entropy}{:08x}", crc32fast::hash(entropy.as_bytes()));
    let hash = hex::encode(Sha256::digest(plain.as_bytes()));
    (format!("{id}|{plain}"), hash)
}

impl TestApp {
    pub fn now(&self) -> OffsetDateTime {
        self.clock.now()
    }

    /// `Alliance::create([...])`.
    pub async fn create_alliance(&self, id: i64, name: &str, ticker: &str) {
        sqlx::query(
            "insert into alliances (id, name, ticker, created_at, updated_at) values ($1, $2, $3, $4, $4)",
        )
        .bind(id)
        .bind(name)
        .bind(ticker)
        .bind(self.now())
        .execute(&self.pool)
        .await
        .expect("alliance inserts");
    }

    /// `Corporation::create([...])`.
    pub async fn create_corporation(
        &self,
        id: i64,
        name: &str,
        ticker: &str,
        alliance_id: Option<i64>,
    ) {
        sqlx::query(
            "insert into corporations (id, name, ticker, alliance_id, created_at, updated_at)
             values ($1, $2, $3, $4, $5, $5)",
        )
        .bind(id)
        .bind(name)
        .bind(ticker)
        .bind(alliance_id)
        .bind(self.now())
        .execute(&self.pool)
        .await
        .expect("corporation inserts");
    }

    /// A user with the given fields, the rest made up.
    pub async fn create_user(&self, user: NewUser) -> User {
        let next: i64 =
            sqlx::query_scalar("select coalesce(max(character_id), 90000000) + 1 from users")
                .fetch_one(&self.pool)
                .await
                .expect("next character id");
        let character_id = user.character_id.unwrap_or(next);
        let character_name = user
            .character_name
            .unwrap_or_else(|| format!("pilot{}", character_id - 90_000_000));
        let id = sqlx::query_scalar(
            "insert into users (character_id, character_name, character_owner_hash,
                corporation_id, alliance_id, created_at, updated_at)
             values ($1, $2, $3, $4, $5, $6, $6) returning id",
        )
        .bind(character_id)
        .bind(&character_name)
        .bind(random_string(40))
        .bind(user.corporation_id)
        .bind(user.alliance_id)
        .bind(self.now())
        .fetch_one(&self.pool)
        .await
        .expect("user inserts");
        User {
            id,
            character_id,
            character_name,
            corporation_id: user.corporation_id,
            alliance_id: user.alliance_id,
        }
    }

    /// A user with defaults.
    pub async fn user(&self) -> User {
        self.create_user(NewUser::default()).await
    }

    /// A new named token of the user: the bearer token a desktop client
    /// holds.
    pub async fn token_named(&self, user: &User, name: &str) -> String {
        let id: i64 = sqlx::query_scalar(
            "select nextval(pg_get_serial_sequence('personal_access_tokens', 'id'))",
        )
        .fetch_one(&self.pool)
        .await
        .expect("a token id");
        let (plain, hash) = sanctum_token(id);
        sqlx::query(
            "insert into personal_access_tokens (id, tokenable_type, tokenable_id, name, token,
                abilities, created_at, updated_at)
             values ($1, 'App\\Models\\User', $2, $3, $4, '[\"*\"]', $5, $5)",
        )
        .bind(id)
        .bind(user.id)
        .bind(name)
        .bind(hash)
        .bind(self.now())
        .execute(&self.pool)
        .await
        .expect("token inserts");
        plain
    }

    pub async fn token(&self, user: &User) -> String {
        self.token_named(user, "test").await
    }

    /// `IntelNetwork::create(['name' => ..., 'owner_id' => ...])`: the
    /// slug is `Str::slug(name)` and six random characters.
    pub async fn create_network(&self, name: &str, owner_id: i64) -> i64 {
        let slug = format!(
            "{}-{}",
            name.to_lowercase().replace(' ', "-"),
            random_string(6)
        );
        sqlx::query_scalar(
            "insert into intel_networks (name, slug, owner_id, created_at, updated_at)
             values ($1, $2, $3, $4, $4) returning id",
        )
        .bind(name)
        .bind(slug)
        .bind(owner_id)
        .bind(self.now())
        .fetch_one(&self.pool)
        .await
        .expect("network inserts")
    }

    /// `$network->accesses()->create([...])`.
    pub async fn grant(
        &self,
        network: i64,
        accessible_type: &str,
        accessible_id: i64,
        permission: &str,
        is_owner: bool,
        expires_at: Option<OffsetDateTime>,
    ) -> i64 {
        sqlx::query_scalar(
            "insert into network_access (intel_network_id, accessible_type, accessible_id,
                permission, is_owner, expires_at, created_at, updated_at)
             values ($1, $2, $3, $4, $5, $6, $7, $7) returning id",
        )
        .bind(network)
        .bind(accessible_type)
        .bind(accessible_id)
        .bind(permission)
        .bind(is_owner)
        .bind(expires_at)
        .bind(self.now())
        .fetch_one(&self.pool)
        .await
        .expect("access inserts")
    }

    /// A character's access.
    pub async fn grant_user(&self, network: i64, user: &User, permission: &str) -> i64 {
        self.grant(network, USER, user.character_id, permission, false, None)
            .await
    }

    /// The owner's access, as `POST /api/networks` creates it.
    pub async fn grant_owner(&self, network: i64, user: &User) -> i64 {
        self.grant(network, USER, user.character_id, "manager", true, None)
            .await
    }

    /// `$network->entries()->create([...])`.
    pub async fn create_entry(&self, network: i64, entry: NewEntry) -> i64 {
        sqlx::query_scalar(
            "insert into intel_entries (intel_network_id, entity_type, entity_id, entity_name,
                color, label, notes, added_by, created_at, updated_at)
             values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9) returning id",
        )
        .bind(network)
        .bind(entry.entity_type)
        .bind(entry.entity_id)
        .bind(&entry.entity_name)
        .bind(&entry.color)
        .bind(&entry.label)
        .bind(&entry.notes)
        .bind(entry.added_by)
        .bind(self.now())
        .fetch_one(&self.pool)
        .await
        .expect("entry inserts")
    }

    /// A character entry with a label.
    pub async fn entry(&self, network: i64, entity_id: i64, label: &str, added_by: i64) -> i64 {
        self.create_entry(
            network,
            NewEntry {
                entity_type: "character",
                entity_id,
                entity_name: "Target".to_owned(),
                label: Some(label.to_owned()),
                added_by,
                ..NewEntry::default()
            },
        )
        .await
    }

    /// `$network->scans()->create([...])` at a moment.
    pub async fn create_scan_at(
        &self,
        network: i64,
        user_id: i64,
        scan_type: &str,
        raw_text: &str,
        at: OffsetDateTime,
    ) -> i64 {
        sqlx::query_scalar(
            "insert into network_scans (intel_network_id, user_id, scan_type, raw_text,
                created_at, updated_at)
             values ($1, $2, $3, $4, $5, $5) returning id",
        )
        .bind(network)
        .bind(user_id)
        .bind(scan_type)
        .bind(raw_text)
        .bind(at)
        .fetch_one(&self.pool)
        .await
        .expect("scan inserts")
    }

    pub async fn create_scan(
        &self,
        network: i64,
        user_id: i64,
        scan_type: &str,
        raw_text: &str,
    ) -> i64 {
        self.create_scan_at(network, user_id, scan_type, raw_text, self.now())
            .await
    }

    /// A share with its pilots and view count.
    pub async fn create_share(&self, code: &str, pilots: &[&str], views: i64) -> i64 {
        sqlx::query_scalar(
            "insert into shares (code, pilots, pilot_count, views, created_at, updated_at)
             values ($1, $2, $3, $4, $5, $5) returning id",
        )
        .bind(code)
        .bind(json!(pilots))
        .bind(pilots.len() as i32)
        .bind(views)
        .bind(self.now())
        .fetch_one(&self.pool)
        .await
        .expect("share inserts")
    }

    /// One row of a table as JSON, by id.
    pub async fn row(&self, table: &str, id: i64) -> Option<Value> {
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "select to_jsonb(t) from {table} t where id = $1"
        )))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .expect("row reads")
    }
}
