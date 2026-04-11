//! Internal value-type model for the split tree.
//!
//! Ported from:
//! - [vendor/bonsplit/Sources/Bonsplit/Internal/Models/TabItem.swift](../../../../../../vendor/bonsplit/Sources/Bonsplit/Internal/Models/TabItem.swift)
//! - [vendor/bonsplit/Sources/Bonsplit/Internal/Models/PaneState.swift](../../../../../../vendor/bonsplit/Sources/Bonsplit/Internal/Models/PaneState.swift)
//! - [vendor/bonsplit/Sources/Bonsplit/Internal/Models/SplitState.swift](../../../../../../vendor/bonsplit/Sources/Bonsplit/Internal/Models/SplitState.swift)
//! - [vendor/bonsplit/Sources/Bonsplit/Internal/Models/SplitNode.swift](../../../../../../vendor/bonsplit/Sources/Bonsplit/Internal/Models/SplitNode.swift)
//!
//! Swift uses `@Observable final class` reference semantics so that
//! SwiftUI views can mutate nested panes via shared references. The Rust
//! port intentionally switches to **owned value types**: mutation goes
//! through [`SplitNode`] recursive methods, and the React frontend in
//! Phase 5 receives snapshots via `serde_json`. This removes the need
//! for `Arc<Mutex<_>>` sprinkled throughout the tree and keeps the
//! property tests cheap.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::types::{PaneId, SplitOrientation, TabId, UnitRect};

/// Single tab inside a pane's tab bar. Mirrors Swift `TabItem`.
///
/// `iconImageData` is deliberately omitted from Phase 4 — the terminal
/// renderer in Phase 6 supplies its own icons, and persisting raw PNG
/// bytes through the workspace model was a pure macOS UI convenience.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TabItem {
    pub id: TabId,
    pub title: String,
    #[serde(default)]
    pub has_custom_title: bool,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub is_dirty: bool,
    #[serde(default)]
    pub shows_notification_badge: bool,
    #[serde(default)]
    pub is_loading: bool,
    #[serde(default)]
    pub is_pinned: bool,
}

impl TabItem {
    /// Create a new tab with a freshly generated id and the default icon.
    pub fn new<S: Into<String>>(title: S) -> Self {
        Self {
            id: TabId::new(),
            title: title.into(),
            has_custom_title: false,
            icon: Some("doc.text".to_string()),
            kind: None,
            is_dirty: false,
            shows_notification_badge: false,
            is_loading: false,
            is_pinned: false,
        }
    }

    /// Convenience for tests / session restore that need a specific id.
    pub fn with_id<S: Into<String>>(id: Uuid, title: S) -> Self {
        Self {
            id: TabId::from_uuid(id),
            title: title.into(),
            has_custom_title: false,
            icon: Some("doc.text".to_string()),
            kind: None,
            is_dirty: false,
            shows_notification_badge: false,
            is_loading: false,
            is_pinned: false,
        }
    }
}

/// Value-typed pane state. Mirrors Swift `PaneState` but owned by value.
///
/// The `selected_tab_id` invariant is: either `None` (empty pane) or a
/// tab id that is present in `tabs`. All mutators uphold this.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PaneState {
    pub id: PaneId,
    pub tabs: Vec<TabItem>,
    pub selected_tab_id: Option<TabId>,
}

impl PaneState {
    /// New pane with a fresh id and the first tab selected.
    pub fn new(tabs: Vec<TabItem>) -> Self {
        let selected = tabs.first().map(|t| t.id);
        Self {
            id: PaneId::new(),
            tabs,
            selected_tab_id: selected,
        }
    }

    /// Empty pane (no tabs, no selection).
    pub fn empty() -> Self {
        Self {
            id: PaneId::new(),
            tabs: Vec::new(),
            selected_tab_id: None,
        }
    }

    /// Currently selected tab, if any.
    pub fn selected_tab(&self) -> Option<&TabItem> {
        let id = self.selected_tab_id?;
        self.tabs.iter().find(|t| t.id == id)
    }

    /// Change selection. Noop if the id isn't in this pane.
    pub fn select_tab(&mut self, tab_id: TabId) {
        if self.tabs.iter().any(|t| t.id == tab_id) {
            self.selected_tab_id = Some(tab_id);
        }
    }

    fn pinned_count(&self) -> usize {
        self.tabs.iter().filter(|t| t.is_pinned).count()
    }

    /// Append a tab (respecting the pinned prefix invariant), and
    /// select it unless `select` is false. Mirrors Swift `addTab`.
    pub fn add_tab(&mut self, tab: TabItem, select: bool) {
        let pinned = self.pinned_count();
        let insert_index = if tab.is_pinned { pinned } else { self.tabs.len() };
        let id = tab.id;
        self.tabs.insert(insert_index, tab);
        if select {
            self.selected_tab_id = Some(id);
        }
    }

    /// Insert a tab at a specific index (clamped). Mirrors Swift
    /// `insertTab(_:at:select:)` including the pinned-prefix clamp.
    pub fn insert_tab(&mut self, tab: TabItem, index: usize, select: bool) {
        let pinned = self.pinned_count();
        let requested = index.min(self.tabs.len());
        let safe_index = if tab.is_pinned {
            requested.min(pinned)
        } else {
            requested.max(pinned)
        };
        let id = tab.id;
        self.tabs.insert(safe_index, tab);
        if select {
            self.selected_tab_id = Some(id);
        }
    }

    /// Remove a tab by id and return it. The selection follows the
    /// same "prefer the next tab, else the previous" rule as Swift.
    pub fn remove_tab(&mut self, tab_id: TabId) -> Option<TabItem> {
        let index = self.tabs.iter().position(|t| t.id == tab_id)?;
        let tab = self.tabs.remove(index);
        if self.selected_tab_id == Some(tab_id) {
            self.selected_tab_id = if self.tabs.is_empty() {
                None
            } else {
                let new_index = index.min(self.tabs.len().saturating_sub(1));
                Some(self.tabs[new_index].id)
            };
        }
        Some(tab)
    }

    /// Reorder a tab within this pane. Mirrors Swift `moveTab`.
    pub fn move_tab(&mut self, source_index: usize, destination_index: usize) {
        if source_index >= self.tabs.len() || destination_index > self.tabs.len() {
            return;
        }
        // Dropping onto itself / right-after-itself is a no-op (Swift
        // comment calls this out to avoid visual churn during drag/drop).
        if destination_index == source_index || destination_index == source_index + 1 {
            return;
        }
        let tab = self.tabs.remove(source_index);
        let requested = if destination_index > source_index {
            destination_index - 1
        } else {
            destination_index
        };
        let pinned = self.pinned_count();
        let adjusted = if tab.is_pinned {
            requested.min(pinned)
        } else {
            requested.max(pinned)
        };
        let safe_index = adjusted.min(self.tabs.len());
        self.tabs.insert(safe_index, tab);
    }
}

/// Interior split node state. Mirrors Swift `SplitState`, minus the
/// SwiftUI animation origin (handled frontend-side).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SplitState {
    pub id: Uuid,
    pub orientation: SplitOrientation,
    pub first: Box<SplitNode>,
    pub second: Box<SplitNode>,
    pub divider_position: f64,
}

impl SplitState {
    pub fn new(orientation: SplitOrientation, first: SplitNode, second: SplitNode) -> Self {
        Self {
            id: Uuid::new_v4(),
            orientation,
            first: Box::new(first),
            second: Box::new(second),
            divider_position: 0.5,
        }
    }
}

/// Recursive split tree. Mirrors Swift `SplitNode`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SplitNode {
    Pane(PaneState),
    Split(SplitState),
}

/// Computed bounds for one pane in normalised `(0..1)` space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaneBounds {
    pub pane_id: PaneId,
    pub bounds: UnitRect,
}

impl SplitNode {
    pub fn leaf(pane: PaneState) -> Self {
        Self::Pane(pane)
    }

    /// Find a pane by id (immutable).
    pub fn find_pane(&self, pane_id: PaneId) -> Option<&PaneState> {
        match self {
            Self::Pane(state) => {
                if state.id == pane_id {
                    Some(state)
                } else {
                    None
                }
            }
            Self::Split(state) => state
                .first
                .find_pane(pane_id)
                .or_else(|| state.second.find_pane(pane_id)),
        }
    }

    /// Find a pane by id (mutable).
    pub fn find_pane_mut(&mut self, pane_id: PaneId) -> Option<&mut PaneState> {
        match self {
            Self::Pane(state) => {
                if state.id == pane_id {
                    Some(state)
                } else {
                    None
                }
            }
            Self::Split(state) => {
                if let Some(p) = state.first.find_pane_mut(pane_id) {
                    return Some(p);
                }
                state.second.find_pane_mut(pane_id)
            }
        }
    }

    /// Every pane id in the tree, in tree order.
    pub fn all_pane_ids(&self) -> Vec<PaneId> {
        let mut out = Vec::new();
        self.collect_pane_ids(&mut out);
        out
    }

    fn collect_pane_ids(&self, out: &mut Vec<PaneId>) {
        match self {
            Self::Pane(state) => out.push(state.id),
            Self::Split(state) => {
                state.first.collect_pane_ids(out);
                state.second.collect_pane_ids(out);
            }
        }
    }

    /// Every pane by reference.
    pub fn all_panes(&self) -> Vec<&PaneState> {
        let mut out = Vec::new();
        self.collect_panes(&mut out);
        out
    }

    fn collect_panes<'a>(&'a self, out: &mut Vec<&'a PaneState>) {
        match self {
            Self::Pane(state) => out.push(state),
            Self::Split(state) => {
                state.first.collect_panes(out);
                state.second.collect_panes(out);
            }
        }
    }

    /// Compute normalised 0..1 bounds for every pane. Mirrors Swift
    /// `computePaneBounds(in:)`.
    pub fn compute_pane_bounds(&self) -> Vec<PaneBounds> {
        let mut out = Vec::new();
        self.compute_pane_bounds_rec(UnitRect::UNIT, &mut out);
        out
    }

    fn compute_pane_bounds_rec(&self, rect: UnitRect, out: &mut Vec<PaneBounds>) {
        match self {
            Self::Pane(state) => out.push(PaneBounds {
                pane_id: state.id,
                bounds: rect,
            }),
            Self::Split(state) => {
                let p = state.divider_position.clamp(0.0, 1.0);
                let (first_rect, second_rect) = match state.orientation {
                    SplitOrientation::Horizontal => (
                        UnitRect::new(rect.x, rect.y, rect.width * p, rect.height),
                        UnitRect::new(
                            rect.x + rect.width * p,
                            rect.y,
                            rect.width * (1.0 - p),
                            rect.height,
                        ),
                    ),
                    SplitOrientation::Vertical => (
                        UnitRect::new(rect.x, rect.y, rect.width, rect.height * p),
                        UnitRect::new(
                            rect.x,
                            rect.y + rect.height * p,
                            rect.width,
                            rect.height * (1.0 - p),
                        ),
                    ),
                };
                state.first.compute_pane_bounds_rec(first_rect, out);
                state.second.compute_pane_bounds_rec(second_rect, out);
            }
        }
    }

    /// Borrow every split state in the tree.
    pub fn all_splits(&self) -> Vec<&SplitState> {
        let mut out = Vec::new();
        self.collect_splits(&mut out);
        out
    }

    fn collect_splits<'a>(&'a self, out: &mut Vec<&'a SplitState>) {
        if let Self::Split(s) = self {
            out.push(s);
            s.first.collect_splits(out);
            s.second.collect_splits(out);
        }
    }

    /// Mutable access to a split state by id (used by the frontend's
    /// divider-drag command path in Phase 5).
    pub fn find_split_mut(&mut self, split_id: Uuid) -> Option<&mut SplitState> {
        match self {
            Self::Pane(_) => None,
            Self::Split(state) => {
                if state.id == split_id {
                    return Some(state);
                }
                if let Some(s) = state.first.find_split_mut(split_id) {
                    return Some(s);
                }
                state.second.find_split_mut(split_id)
            }
        }
    }
}
