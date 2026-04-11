//! Pane model — Bonsplit `PaneState` / `PaneID`. See
//! [crate::bonsplit].
//!
//! Phase 4 of `PLAN.md`. This module re-exports the authoritative types
//! so `use cmux_core::pane::PaneId` still works for downstream code.

pub use crate::bonsplit::{PaneId, PaneState};

pub fn __link() {}
