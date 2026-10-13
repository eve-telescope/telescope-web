//! EVE's public API: character affiliations, alliances and corporations.
//! Every request carries the app's user agent. A failure (network or
//! status) is `None`; callers carry on without the data.

use std::time::Duration;

use serde::Deserialize;
use serde_json::Value;

/// The user agent every ESI request sends.
pub const USER_AGENT: &str = "Telescope | https://eve-telescope.com";

#[derive(Clone)]
pub struct EsiClient {
    base_url: String,
    http: reqwest::Client,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Affiliation {
    pub character_id: i64,
    pub corporation_id: Option<i64>,
    pub alliance_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Alliance {
    pub name: String,
    pub ticker: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Corporation {
    pub name: String,
    pub ticker: String,
    pub alliance_id: Option<i64>,
    pub member_count: Option<i64>,
}

impl EsiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            http: reqwest::Client::builder()
                .user_agent(USER_AGENT)
                .timeout(Duration::from_secs(30))
                .build()
                .expect("an HTTP client"),
        }
    }

    async fn json<T: serde::de::DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Option<T> {
        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                tracing::warn!(%error, "ESI request failed");
                return None;
            }
        };
        if !response.status().is_success() {
            tracing::warn!(status = %response.status(), "ESI answered an error");
            return None;
        }
        response.json().await.ok()
    }

    /// `POST /latest/characters/affiliation/` for up to 1000 characters.
    pub async fn affiliations(&self, character_ids: &[i64]) -> Option<Vec<Affiliation>> {
        let answer: Value = self
            .json(
                self.http
                    .post(format!("{}/latest/characters/affiliation/", self.base_url))
                    .json(character_ids),
            )
            .await?;
        serde_json::from_value(answer).ok()
    }

    pub async fn alliance(&self, id: i64) -> Option<Alliance> {
        self.json(
            self.http
                .get(format!("{}/latest/alliances/{id}/", self.base_url)),
        )
        .await
    }

    pub async fn corporation(&self, id: i64) -> Option<Corporation> {
        self.json(
            self.http
                .get(format!("{}/latest/corporations/{id}/", self.base_url)),
        )
        .await
    }
}
