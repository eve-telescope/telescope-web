//! Runtime configuration from the environment, resolved once at startup.
//! Tests build a `Config` by hand instead of mutating process env vars.

use std::time::Duration;

/// Everything the app reads from the environment.
#[derive(Debug, Clone)]
pub struct Config {
    /// `APP_NAME`: the shared `name` prop.
    pub app_name: String,
    /// `APP_URL` without a trailing slash: share links, page URLs and
    /// pagination links.
    pub app_url: String,
    pub eve: EveConfig,
    pub pusher: PusherConfig,
    /// `ESI_BASE_URL`: EVE's public API.
    pub esi_base_url: String,
    /// `GITHUB_API_URL`: where the desktop app's latest release is read.
    pub github_api_url: String,
    /// `SCAN_RETENTION_DAYS`: shared scans older than this many days are
    /// purged by the schedule; 0 keeps them forever.
    pub scan_retention_days: i64,
    /// `SCHEDULER_ENABLED`: runs the hourly tasks (affiliations, expired
    /// access, old scans). Off unless asked for.
    pub scheduler_enabled: bool,
    /// `SESSION_LIFETIME_DAYS`: how long the website's sign-in lasts.
    pub session_lifetime_days: i64,
    /// `DEMO_ENABLED` and `DEMO_CHARACTER_ID`: the one-click demo login for
    /// local stacks, off unless asked for.
    pub demo: DemoConfig,
}

/// The EVE SSO application.
#[derive(Debug, Clone)]
pub struct EveConfig {
    /// `SSO_BASE_URL`.
    pub sso_base_url: String,
    /// `EVE_CLIENT_ID` and `EVE_CLIENT_SECRET`.
    pub client_id: String,
    pub client_secret: String,
    /// `EVE_CALLBACK`: the callback registered with CCP.
    pub callback_url: String,
}

/// The Pusher protocol server (the replacement of Reverb).
#[derive(Debug, Clone)]
pub struct PusherConfig {
    /// `PUSHER_APP_ID` (or `REVERB_APP_ID`).
    pub app_id: String,
    /// `PUSHER_APP_KEY` (or `REVERB_APP_KEY`): every desktop client ships
    /// with it, so it never changes.
    pub key: String,
    /// `PUSHER_APP_SECRET` (or `REVERB_APP_SECRET`): signs channel
    /// subscriptions. Only this process signs and checks them, so a
    /// random one is generated when unset.
    pub secret: String,
    /// `PUSHER_ACTIVITY_TIMEOUT`: the seconds a client may stay silent,
    /// announced on connect.
    pub activity_timeout: u64,
    /// `PUSHER_PING_INTERVAL`: a connection silent for this long is
    /// pinged, and dropped when it stays silent for as long again.
    pub ping_interval: Duration,
    /// `PUSHER_MAX_MESSAGE_SIZE`: the largest message a client may send,
    /// in bytes.
    pub max_message_size: usize,
}

#[derive(Debug, Clone)]
pub struct DemoConfig {
    pub enabled: bool,
    pub character_id: i64,
}

pub const DEFAULT_APP_NAME: &str = "Telescope";
pub const DEFAULT_APP_URL: &str = "https://eve-telescope.com";
pub const DEFAULT_SSO_BASE_URL: &str = "https://login.eveonline.com";
pub const DEFAULT_CALLBACK_URL: &str = "https://eve-telescope.com/eve/callback";
pub const DEFAULT_ESI_BASE_URL: &str = "https://esi.evetech.net";
pub const DEFAULT_GITHUB_API_URL: &str = "https://api.github.com";
/// The key every desktop client since v0.2.1 connects with.
pub const DEFAULT_PUSHER_KEY: &str = "04pcwy13bvcyjoio6mf6";
pub const DEFAULT_PUSHER_APP_ID: &str = "telescope";
pub const DEFAULT_ACTIVITY_TIMEOUT: u64 = 30;
pub const DEFAULT_PING_INTERVAL: Duration = Duration::from_secs(60);
pub const DEFAULT_MAX_MESSAGE_SIZE: usize = 10_000;
pub const DEFAULT_SCAN_RETENTION_DAYS: i64 = 7;
pub const DEFAULT_SESSION_LIFETIME_DAYS: i64 = 30;
pub const DEFAULT_DEMO_CHARACTER_ID: i64 = 2_100_000_001;

/// A variable that is set and not empty (`null`, as Laravel's .env files
/// write it, counts as unset).
fn env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .filter(|value| !value.is_empty() && value != "null")
}

fn env_or(key: &str, default: &str) -> String {
    env(key).unwrap_or_else(|| default.to_owned())
}

/// A switch: `true`, `1`, `on` or `yes`.
fn env_flag(key: &str) -> bool {
    env(key).is_some_and(|value| {
        matches!(
            value.to_ascii_lowercase().as_str(),
            "true" | "1" | "on" | "yes"
        )
    })
}

/// A number from the environment; one that does not parse stops the
/// startup instead of running with a surprise.
fn env_number<T: std::str::FromStr>(key: &str, default: T) -> T {
    env(key).map_or(default, |value| {
        value
            .parse()
            .unwrap_or_else(|_| panic!("{key} is {value:?}, expected a number"))
    })
}

/// A random secret for a process that was given none.
pub fn random_secret() -> String {
    use rand::RngExt;
    use rand::distr::Alphanumeric;
    rand::rng()
        .sample_iter(Alphanumeric)
        .take(40)
        .map(char::from)
        .collect()
}

impl Config {
    pub fn from_env() -> Self {
        let app_url = env_or("APP_URL", DEFAULT_APP_URL)
            .trim_end_matches('/')
            .to_owned();
        Self {
            app_name: env_or("APP_NAME", DEFAULT_APP_NAME),
            eve: EveConfig {
                sso_base_url: env_or("SSO_BASE_URL", DEFAULT_SSO_BASE_URL)
                    .trim_end_matches('/')
                    .to_owned(),
                client_id: env_or("EVE_CLIENT_ID", ""),
                client_secret: env_or("EVE_CLIENT_SECRET", ""),
                callback_url: env_or("EVE_CALLBACK", DEFAULT_CALLBACK_URL),
            },
            pusher: PusherConfig {
                app_id: env("PUSHER_APP_ID")
                    .or_else(|| env("REVERB_APP_ID"))
                    .unwrap_or_else(|| DEFAULT_PUSHER_APP_ID.to_owned()),
                key: env("PUSHER_APP_KEY")
                    .or_else(|| env("REVERB_APP_KEY"))
                    .unwrap_or_else(|| DEFAULT_PUSHER_KEY.to_owned()),
                secret: env("PUSHER_APP_SECRET")
                    .or_else(|| env("REVERB_APP_SECRET"))
                    .unwrap_or_else(random_secret),
                activity_timeout: env_number("PUSHER_ACTIVITY_TIMEOUT", DEFAULT_ACTIVITY_TIMEOUT),
                ping_interval: Duration::from_secs(env_number(
                    "PUSHER_PING_INTERVAL",
                    DEFAULT_PING_INTERVAL.as_secs(),
                )),
                max_message_size: env_number("PUSHER_MAX_MESSAGE_SIZE", DEFAULT_MAX_MESSAGE_SIZE),
            },
            esi_base_url: env_or("ESI_BASE_URL", DEFAULT_ESI_BASE_URL)
                .trim_end_matches('/')
                .to_owned(),
            github_api_url: env_or("GITHUB_API_URL", DEFAULT_GITHUB_API_URL)
                .trim_end_matches('/')
                .to_owned(),
            scan_retention_days: env_number("SCAN_RETENTION_DAYS", DEFAULT_SCAN_RETENTION_DAYS),
            scheduler_enabled: env_flag("SCHEDULER_ENABLED"),
            session_lifetime_days: env_number(
                "SESSION_LIFETIME_DAYS",
                DEFAULT_SESSION_LIFETIME_DAYS,
            ),
            demo: DemoConfig {
                enabled: env_flag("DEMO_ENABLED"),
                character_id: env_number("DEMO_CHARACTER_ID", DEFAULT_DEMO_CHARACTER_ID),
            },
            app_url,
        }
    }

    /// Cookies are `Secure` when the site is served over HTTPS.
    pub fn secure_cookies(&self) -> bool {
        self.app_url.starts_with("https://")
    }

    /// An absolute URL on the site.
    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.app_url)
    }
}
