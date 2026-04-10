//! Bonsplit model port. The original lives in
//! [vendor/bonsplit/Sources/Bonsplit/] and is **not** modified per the locked
//! decision in `PLAN.md` (the one documented exception to the "same folder,
//! same name" rule).
//!
//! This module mirrors the public types from
//! `vendor/bonsplit/Sources/Bonsplit/Public/Types/`:
//! - `TabID`, `PaneID`, `Tab`
//! - `SplitOrientation`, `NavigationDirection`
//! - `LayoutSnapshot`
//! - `TabContextAction`
//!
//! and the internal model files under `Internal/Models/`:
//! - `SplitNode`, `SplitState`, `PaneState`, `TabItem`
//!
//! Phase 4 of `PLAN.md`.

// TODO(rewrite): port Bonsplit model layer. SwiftUI views are replaced by
// the TS implementation in cmux-rs/ui/src/bonsplit/.
pub fn __link() {}
