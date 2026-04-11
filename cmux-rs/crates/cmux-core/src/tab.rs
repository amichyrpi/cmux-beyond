//! Tab model — ported from [Sources/TabManager.swift] and the Bonsplit
//! `TabItem` / `TabID` public types.
//!
//! Phase 4 of `PLAN.md`. The authoritative types live in
//! [`crate::bonsplit`]; this module re-exports them under the
//! short names used by the rest of the codebase so the sibling-file
//! layout stays clean.

pub use crate::bonsplit::{TabId, TabItem};
pub use crate::workspace::TabManager;

/// Kept so `sources::link_all()` in
/// [cmux-rs/crates/cmux-app/src/sources.rs](../../../cmux-app/src/sources.rs)
/// continues to link.
pub fn __link() {}
