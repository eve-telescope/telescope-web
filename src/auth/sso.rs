//! EVE SSO (OAuth2), as the Socialite EVE Online provider used it: the
//! authorize redirect, the code traded for a token with basic auth, and
//! the token's JWT verified against EVE's published keys (signature,
//! expiry, issuer) to read the character.

use std::fmt;
use std::time::Duration;

use serde::Deserialize;

use crate::config::EveConfig;

/// The issuers EVE's tokens carry.
const ACCEPTED_ISSUERS: [&str; 2] = ["login.eveonline.com", "https://login.eveonline.com"];

/// The prefix of the `sub` claim in front of the character id.
const SUBJECT_PREFIX: &str = "CHARACTER:EVE:";

#[derive(Debug)]
pub enum SsoError {
    Http(reqwest::Error),
    Status(reqwest::StatusCode),
    Token(jsonwebtoken::errors::Error),
    Claims(&'static str),
}

impl fmt::Display for SsoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SsoError::Http(error) => write!(f, "SSO request failed: {error}"),
            SsoError::Status(status) => write!(f, "unexpected SSO status: {status}"),
            SsoError::Token(error) => write!(f, "invalid SSO token: {error}"),
            SsoError::Claims(what) => write!(f, "malformed SSO token: {what}"),
        }
    }
}

impl std::error::Error for SsoError {}

impl From<reqwest::Error> for SsoError {
    fn from(error: reqwest::Error) -> Self {
        SsoError::Http(error)
    }
}

impl From<jsonwebtoken::errors::Error> for SsoError {
    fn from(error: jsonwebtoken::errors::Error) -> Self {
        SsoError::Token(error)
    }
}

/// The character behind a verified token.
#[derive(Debug, Clone)]
pub struct Character {
    pub id: i64,
    pub name: String,
    pub owner_hash: String,
}

#[derive(Deserialize)]
struct TokenAnswer {
    access_token: String,
}

#[derive(Deserialize)]
struct Claims {
    sub: String,
    name: String,
    owner: String,
}

#[derive(Clone)]
pub struct SsoClient {
    config: EveConfig,
    http: reqwest::Client,
}

/// `http_build_query`'s encoding: RFC 1738, a space is `+`.
pub fn form_encode(value: &str) -> String {
    form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

impl SsoClient {
    pub fn new(config: EveConfig) -> Self {
        Self {
            config,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("an HTTP client"),
        }
    }

    /// The authorize URL with Socialite's parameters in its order: no
    /// scopes, the callback registered with CCP and the state.
    pub fn authorize_url(&self, state: &str) -> String {
        format!(
            "{}/v2/oauth/authorize/?client_id={}&redirect_uri={}&scope=&response_type=code&state={}",
            self.config.sso_base_url,
            form_encode(&self.config.client_id),
            form_encode(&self.config.callback_url),
            form_encode(state),
        )
    }

    /// Trades the authorization code for an access token and reads the
    /// character from it.
    pub async fn character(&self, code: &str) -> Result<Character, SsoError> {
        let response = self
            .http
            .post(format!("{}/v2/oauth/token", self.config.sso_base_url))
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .form(&[("grant_type", "authorization_code"), ("code", code)])
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(SsoError::Status(response.status()));
        }
        let answer: TokenAnswer = response.json().await?;
        self.verify(&answer.access_token).await
    }

    /// Verifies a token against the JWKS: the key named in its header, the
    /// algorithm that key carries (RS256 or ES256, never the token's own
    /// claim), its expiry and issuer.
    pub async fn verify(&self, token: &str) -> Result<Character, SsoError> {
        let response = self
            .http
            .get(format!("{}/oauth/jwks", self.config.sso_base_url))
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(SsoError::Status(response.status()));
        }
        let jwks: jsonwebtoken::jwk::JwkSet = response.json().await?;
        let header = jsonwebtoken::decode_header(token)?;
        let key = match &header.kid {
            Some(kid) => jwks.find(kid),
            None => jwks.keys.first(),
        }
        .ok_or(SsoError::Claims("no matching key"))?;
        let algorithm = match &key.algorithm {
            jsonwebtoken::jwk::AlgorithmParameters::RSA(_) => jsonwebtoken::Algorithm::RS256,
            jsonwebtoken::jwk::AlgorithmParameters::EllipticCurve(_) => {
                jsonwebtoken::Algorithm::ES256
            }
            _ => return Err(SsoError::Claims("unsupported key type")),
        };
        let mut validation = jsonwebtoken::Validation::new(algorithm);
        validation.set_issuer(&ACCEPTED_ISSUERS);
        // The audience is the app's client id; the provider did not check
        // it either.
        validation.validate_aud = false;
        let claims = jsonwebtoken::decode::<Claims>(
            token,
            &jsonwebtoken::DecodingKey::from_jwk(key)?,
            &validation,
        )?
        .claims;
        let id = claims
            .sub
            .strip_prefix(SUBJECT_PREFIX)
            .and_then(|id| id.parse().ok())
            .ok_or(SsoError::Claims("the subject is no character"))?;
        Ok(Character {
            id,
            name: claims.name,
            owner_hash: claims.owner,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_socialites_authorize_url() {
        let client = SsoClient::new(EveConfig {
            sso_base_url: "https://login.eveonline.com".to_owned(),
            client_id: "client id".to_owned(),
            client_secret: "secret".to_owned(),
            callback_url: "https://eve-telescope.com/eve/callback".to_owned(),
        });
        assert_eq!(
            client.authorize_url("abc"),
            "https://login.eveonline.com/v2/oauth/authorize/?client_id=client+id&redirect_uri=https%3A%2F%2Feve-telescope.com%2Feve%2Fcallback&scope=&response_type=code&state=abc"
        );
    }
}
