//! The single integration-test binary: every suite lives here as a module
//! so a full `cargo test` links one executable.
//!
//! The suites assume they never run concurrently with each other: they
//! truncate the shared tables. `.cargo/config.toml` pins
//! `RUST_TEST_THREADS=1` to keep that assumption true.
//!
//! The ported PHP tests keep their file names (`entity_search`,
//! `intel_entry`, `intel_network`, `network_access`, `network_scan`,
//! `permission`, `purge_old_scans`); the `golden_*` suites compare with
//! what the Laravel reference answered.

mod common;

mod affiliations;
mod api_auth;
mod api_errors;
mod api_misc;
mod api_tokens;
mod background;
mod broadcasting_auth;
mod cors;
mod demo;
mod entity_search;
mod eve_login;
mod events;
mod github;
mod golden_api;
mod golden_php;
mod golden_pusher;
mod harness;
mod intel_entry;
mod intel_network;
mod network_access;
mod network_scan;
mod pages;
mod permission;
mod purge_old_scans;
mod pusher_protocol;
mod share;
mod web_session;
