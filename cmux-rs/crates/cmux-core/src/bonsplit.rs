//! Bonsplit model port. The original lives in
//! [vendor/bonsplit/Sources/Bonsplit/] and is **not** modified per the locked
//! decision in `PLAN.md` (the one documented exception to the "same folder,
//! same name" rule).
//!
//! This module mirrors the public and internal types from the Swift package:
//!
//! - Public types in [vendor/bonsplit/Sources/Bonsplit/Public/Types/]:
//!   [`TabId`](types::TabId), [`PaneId`](types::PaneId),
//!   [`SplitOrientation`](types::SplitOrientation),
//!   [`NavigationDirection`](types::NavigationDirection),
//!   [`LayoutSnapshot`](layout::LayoutSnapshot),
//!   [`ExternalTreeNode`](layout::ExternalTreeNode).
//! - Internal model types in
//!   [vendor/bonsplit/Sources/Bonsplit/Internal/Models/]:
//!   [`TabItem`](model::TabItem),
//!   [`PaneState`](model::PaneState),
//!   [`SplitState`](model::SplitState),
//!   [`SplitNode`](model::SplitNode).
//! - The central
//!   [`SplitViewController`](controller::SplitViewController) ported from
//!   [vendor/bonsplit/Sources/Bonsplit/Internal/Controllers/SplitViewController.swift].
//!
//! Phase 4 of `PLAN.md`. The SwiftUI views are replaced by the TS
//! implementation in [cmux-rs/ui/src/bonsplit/](../../../../../ui/src/bonsplit/).
//! The AppKit drag/drop layer is handled Tauri-side in Phase 5.

pub mod controller;
pub mod layout;
pub mod model;
pub mod types;

pub use controller::SplitViewController;
pub use layout::{
    ExternalPaneNode, ExternalSplitNode, ExternalTab, ExternalTreeNode, LayoutSnapshot,
    PaneGeometry, PixelRect,
};
pub use model::{PaneState, SplitNode, SplitState, TabItem};
pub use types::{NavigationDirection, PaneId, SplitOrientation, TabId, UnitRect};

/// Kept for the Phase 2 sibling-stub scheme so `sources::link_all()` in
/// [cmux-rs/crates/cmux-app/src/sources.rs](../../../cmux-app/src/sources.rs)
/// continues to link while the real types land.
pub fn __link() {}
