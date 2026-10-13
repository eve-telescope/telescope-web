//! The latest release of the desktop app, shown on every page as
//! `appVersion`: asked from GitHub with a 5 second timeout and kept for an
//! hour; a failure is `null` and is kept for 5 minutes, so a failing
//! GitHub does not slow every page.

use std::sync::Arc;
use std::time::Duration as StdDuration;

use serde::Deserialize;
use time::{Duration, OffsetDateTime};
use tokio::sync::Mutex;

use crate::clock::Clock;

const REPOSITORY: &str = "eve-telescope/telescope-app";
const KEEP: Duration = Duration::hours(1);
const KEEP_FAILURE: Duration = Duration::minutes(5);
const TIMEOUT: StdDuration = StdDuration::from_secs(5);

#[derive(Deserialize)]
struct Release {
    tag_name: Option<String>,
}

/// The answer and until when it is kept.
type Cached = Option<(Option<String>, OffsetDateTime)>;

#[derive(Clone)]
pub struct LatestRelease {
    api_url: String,
    http: reqwest::Client,
    clock: Clock,
    cached: Arc<Mutex<Cached>>,
}

impl LatestRelease {
    pub fn new(api_url: &str, clock: Clock) -> Self {
        Self {
            api_url: api_url.trim_end_matches('/').to_owned(),
            http: reqwest::Client::builder()
                .user_agent(crate::esi::USER_AGENT)
                .timeout(TIMEOUT)
                .build()
                .expect("an HTTP client"),
            clock,
            cached: Arc::new(Mutex::new(None)),
        }
    }

    /// The tag of the latest release, like `v0.5.1`.
    pub async fn version(&self) -> Option<String> {
        let mut cached = self.cached.lock().await;
        let now = self.clock.now();
        if let Some((version, until)) = cached.as_ref()
            && now < *until
        {
            return version.clone();
        }
        let version = self.fetch().await;
        let keep = if version.is_some() {
            KEEP
        } else {
            KEEP_FAILURE
        };
        *cached = Some((version.clone(), now + keep));
        version
    }

    async fn fetch(&self) -> Option<String> {
        let response = self
            .http
            .get(format!(
                "{}/repos/{REPOSITORY}/releases/latest",
                self.api_url
            ))
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|error| tracing::warn!(%error, "GitHub release not read"))
            .ok()?;
        if !response.status().is_success() {
            tracing::warn!(status = %response.status(), "GitHub release not read");
            return None;
        }
        response.json::<Release>().await.ok()?.tag_name
    }
}
