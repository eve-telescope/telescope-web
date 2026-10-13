//! The single integration-test binary: every suite lives here as a module
//! so a full `cargo test` links one executable.
//!
//! The suites assume they never run concurrently with each other: they
//! truncate the shared tables. `.cargo/config.toml` pins
//! `RUST_TEST_THREADS=1` to keep that assumption true.

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
