//! cmux-core — platform-agnostic domain logic for the Rust rewrite of cmux.
//!
//! See `PLAN.md` at the repo root for the phased plan.
//!
//! This crate is intentionally UI-agnostic. The Tauri binary lives in the
//! sibling `cmux-app` crate and wires these modules to a React frontend.

pub mod bonsplit;
pub mod config;
pub mod pane;
pub mod socket;
pub mod tab;
pub mod terminal;
pub mod workspace;

/// Crate version exposed to the Tauri layer for the about dialog.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
