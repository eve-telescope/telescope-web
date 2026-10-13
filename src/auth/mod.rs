//! Who is asking: desktop clients with Sanctum personal access tokens
//! (`tokens`), the website with its session cookie (`session`), the EVE
//! SSO login that hands out both (`sso`, `login`).

pub mod login;
pub mod session;
pub mod sso;
pub mod tokens;
