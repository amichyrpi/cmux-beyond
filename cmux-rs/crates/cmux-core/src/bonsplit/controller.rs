//! [`SplitViewController`] — central mutation surface for the split tree.
//!
//! Ported from
//! [vendor/bonsplit/Sources/Bonsplit/Internal/Controllers/SplitViewController.swift](../../../../../../vendor/bonsplit/Sources/Bonsplit/Internal/Controllers/SplitViewController.swift).
//!
//! Unlike the Swift version this is a plain `struct` with `&mut self`
//! methods. The Tauri backend wraps it in a [`parking_lot::Mutex`]
//! inside [cmux-rs/crates/cmux-app/src/state.rs](../../../../cmux-app/src/state.rs)
//! and exposes it to the frontend via Tauri commands. Drag state,
//! animation origin, and the AppKit-specific fields from the Swift
//! version are intentionally dropped — they're frontend concerns and
//! the React layer in [cmux-rs/ui/src/bonsplit/](../../../../../ui/src/bonsplit/)
//! owns them.

use super::layout::{ExternalTreeNode, LayoutSnapshot, PixelRect};
use super::model::{PaneBounds, PaneState, SplitNode, SplitState, TabItem};
use super::types::{NavigationDirection, PaneId, SplitOrientation, TabId};

/// Central controller managing the entire split tree.
#[derive(Debug, Clone)]
pub struct SplitViewController {
    root_node: SplitNode,
    focused_pane_id: Option<PaneId>,
    zoomed_pane_id: Option<PaneId>,
}

impl Default for SplitViewController {
    fn default() -> Self {
        Self::new_empty()
    }
}

impl SplitViewController {
    /// New controller with a single welcome pane, matching the Swift
    /// default initializer.
    pub fn new_welcome() -> Self {
        let welcome = TabItem::new("Welcome");
        let pane = PaneState::new(vec![welcome]);
        let focused = Some(pane.id);
        Self {
            root_node: SplitNode::leaf(pane),
            focused_pane_id: focused,
            zoomed_pane_id: None,
        }
    }

    /// New controller with a single empty pane. Used by tests and the
    /// Tauri backend when a fresh workspace is requested without an
    /// initial tab.
    pub fn new_empty() -> Self {
        let pane = PaneState::empty();
        let focused = Some(pane.id);
        Self {
            root_node: SplitNode::leaf(pane),
            focused_pane_id: focused,
            zoomed_pane_id: None,
        }
    }

    /// Wrap an existing tree (used by session restore).
    pub fn from_root(root: SplitNode) -> Self {
        let focused = root.all_pane_ids().first().copied();
        Self {
            root_node: root,
            focused_pane_id: focused,
            zoomed_pane_id: None,
        }
    }

    // --- immutable accessors ---------------------------------------------

    pub fn root(&self) -> &SplitNode {
        &self.root_node
    }

    pub fn focused_pane_id(&self) -> Option<PaneId> {
        self.focused_pane_id
    }

    pub fn focused_pane(&self) -> Option<&PaneState> {
        let id = self.focused_pane_id?;
        self.root_node.find_pane(id)
    }

    pub fn zoomed_pane_id(&self) -> Option<PaneId> {
        self.zoomed_pane_id
    }

    pub fn pane_bounds(&self) -> Vec<PaneBounds> {
        self.root_node.compute_pane_bounds()
    }

    /// Snapshot the tree in a UI-friendly recursive shape. The
    /// container is normalized to `1x1` because the web frontend uses
    /// the fields as flex ratios rather than absolute pixels.
    pub fn tree_snapshot(&self) -> ExternalTreeNode {
        ExternalTreeNode::from_root(&self.root_node, PixelRect::new(0.0, 0.0, 1.0, 1.0))
    }

    /// Snapshot the tree as a flattened geometry list.
    pub fn layout_snapshot(&self, container: PixelRect, timestamp: f64) -> LayoutSnapshot {
        LayoutSnapshot::from_root(&self.root_node, container, self.focused_pane_id, timestamp)
    }

    // --- focus ------------------------------------------------------------

    /// Focus a specific pane. Noop if `pane_id` is not in the tree.
    pub fn focus_pane(&mut self, pane_id: PaneId) {
        if self.root_node.find_pane(pane_id).is_some() {
            self.focused_pane_id = Some(pane_id);
        }
    }

    /// Clear the zoom flag. Returns `true` if anything changed.
    pub fn clear_pane_zoom(&mut self) -> bool {
        if self.zoomed_pane_id.is_some() {
            self.zoomed_pane_id = None;
            true
        } else {
            false
        }
    }

    /// Toggle zoom for a pane. Mirrors Swift `togglePaneZoom(_:)`.
    /// A single-pane layout can never be zoomed.
    pub fn toggle_pane_zoom(&mut self, pane_id: PaneId) -> bool {
        if self.root_node.find_pane(pane_id).is_none() {
            return false;
        }
        if self.zoomed_pane_id == Some(pane_id) {
            self.zoomed_pane_id = None;
            return true;
        }
        if self.root_node.all_pane_ids().len() <= 1 {
            return false;
        }
        self.zoomed_pane_id = Some(pane_id);
        self.focused_pane_id = Some(pane_id);
        true
    }

    // --- split / close ----------------------------------------------------

    /// Split a pane. New pane is empty unless `new_tab` is supplied.
    /// Mirrors Swift `splitPane(_:orientation:with:)`.
    pub fn split_pane(
        &mut self,
        pane_id: PaneId,
        orientation: SplitOrientation,
        new_tab: Option<TabItem>,
    ) -> Option<PaneId> {
        self.clear_pane_zoom();
        let new_pane_id = split_rec(&mut self.root_node, pane_id, orientation, new_tab);
        if let Some(id) = new_pane_id {
            self.focused_pane_id = Some(id);
        }
        new_pane_id
    }

    /// Split a pane and place a specific tab in the new side. If
    /// `insert_first` is true the new pane is the `first` child
    /// (left / top), matching Swift's `splitPaneWithTab`.
    pub fn split_pane_with_tab(
        &mut self,
        pane_id: PaneId,
        orientation: SplitOrientation,
        tab: TabItem,
        insert_first: bool,
    ) -> Option<PaneId> {
        self.clear_pane_zoom();
        let new_pane_id =
            split_with_tab_rec(&mut self.root_node, pane_id, orientation, tab, insert_first);
        if let Some(id) = new_pane_id {
            self.focused_pane_id = Some(id);
        }
        new_pane_id
    }

    /// Close a pane and collapse its split parent. Mirrors Swift
    /// `closePane(_:)`. Refuses to close the last pane.
    pub fn close_pane(&mut self, pane_id: PaneId) -> bool {
        if self.root_node.all_pane_ids().len() <= 1 {
            return false;
        }
        let mut focus_target: Option<PaneId> = None;
        if let Some(new_root) =
            close_pane_rec(self.root_node.clone(), pane_id, &mut focus_target)
        {
            self.root_node = new_root;
        } else {
            return false;
        }
        self.focused_pane_id = focus_target.or_else(|| self.root_node.all_pane_ids().first().copied());
        if let Some(z) = self.zoomed_pane_id {
            if self.root_node.find_pane(z).is_none() {
                self.zoomed_pane_id = None;
            }
        }
        true
    }

    // --- tab operations ---------------------------------------------------

    /// Add a tab to the focused pane (or an explicit pane). Mirrors
    /// Swift `addTab(_:toPane:atIndex:)`.
    pub fn add_tab(&mut self, tab: TabItem, pane_id: Option<PaneId>, index: Option<usize>) {
        let target = pane_id.or(self.focused_pane_id);
        let Some(target) = target else { return };
        let Some(pane) = self.root_node.find_pane_mut(target) else {
            return;
        };
        match index {
            Some(i) => pane.insert_tab(tab, i, true),
            None => pane.add_tab(tab, true),
        }
    }

    /// Select an existing tab inside a pane.
    pub fn select_tab(&mut self, pane_id: PaneId, tab_id: TabId) -> bool {
        let Some(pane) = self.root_node.find_pane_mut(pane_id) else {
            return false;
        };
        if pane.tabs.iter().any(|tab| tab.id == tab_id) {
            pane.select_tab(tab_id);
            self.focused_pane_id = Some(pane_id);
            true
        } else {
            false
        }
    }

    /// Move a tab from one pane to another. Mirrors Swift
    /// `moveTab(_:from:to:atIndex:)`.
    pub fn move_tab(
        &mut self,
        tab_id: TabId,
        source_pane: PaneId,
        target_pane: PaneId,
        index: Option<usize>,
    ) -> bool {
        if self.root_node.find_pane(source_pane).is_none()
            || self.root_node.find_pane(target_pane).is_none()
        {
            return false;
        }
        let tab = {
            let Some(pane) = self.root_node.find_pane_mut(source_pane) else {
                return false;
            };
            match pane.remove_tab(tab_id) {
                Some(t) => t,
                None => return false,
            }
        };
        {
            let Some(pane) = self.root_node.find_pane_mut(target_pane) else {
                return false;
            };
            match index {
                Some(i) => pane.insert_tab(tab, i, true),
                None => pane.add_tab(tab, true),
            }
        }
        self.focus_pane(target_pane);
        // Close the source pane if it's now empty and isn't the only
        // pane in the tree.
        let source_empty = self
            .root_node
            .find_pane(source_pane)
            .map(|p| p.tabs.is_empty())
            .unwrap_or(false);
        if source_empty && self.root_node.all_pane_ids().len() > 1 {
            self.close_pane(source_pane);
        }
        true
    }

    /// Close a tab. Mirrors Swift `closeTab(_:inPane:)`. Closes the
    /// containing pane if it becomes empty and is not the only pane.
    pub fn close_tab(&mut self, tab_id: TabId, pane_id: PaneId) -> bool {
        let removed = {
            let Some(pane) = self.root_node.find_pane_mut(pane_id) else {
                return false;
            };
            pane.remove_tab(tab_id).is_some()
        };
        if !removed {
            return false;
        }
        let empty = self
            .root_node
            .find_pane(pane_id)
            .map(|p| p.tabs.is_empty())
            .unwrap_or(false);
        if empty && self.root_node.all_pane_ids().len() > 1 {
            self.close_pane(pane_id);
        }
        true
    }

    /// Create a new "Untitled N" tab in the focused pane.
    pub fn create_new_tab(&mut self) -> Option<TabId> {
        let pane_id = self.focused_pane_id?;
        let Some(pane) = self.root_node.find_pane_mut(pane_id) else {
            return None;
        };
        let tab = TabItem::new(format!("Untitled {}", pane.tabs.len() + 1));
        let id = tab.id;
        pane.add_tab(tab, true);
        Some(id)
    }

    /// Close the currently selected tab in the focused pane.
    pub fn close_selected_tab(&mut self) -> bool {
        let Some(pane_id) = self.focused_pane_id else {
            return false;
        };
        let Some(selected) = self
            .root_node
            .find_pane(pane_id)
            .and_then(|p| p.selected_tab_id)
        else {
            return false;
        };
        self.close_tab(selected, pane_id)
    }

    /// Cycle to the previous tab in the focused pane (wraps around).
    pub fn select_previous_tab(&mut self) {
        let Some(pane_id) = self.focused_pane_id else {
            return;
        };
        let Some(pane) = self.root_node.find_pane_mut(pane_id) else {
            return;
        };
        if pane.tabs.is_empty() {
            return;
        }
        let Some(current) = pane.selected_tab_id else {
            return;
        };
        let Some(idx) = pane.tabs.iter().position(|t| t.id == current) else {
            return;
        };
        let new_idx = if idx == 0 { pane.tabs.len() - 1 } else { idx - 1 };
        pane.selected_tab_id = Some(pane.tabs[new_idx].id);
    }

    /// Cycle to the next tab in the focused pane (wraps around).
    pub fn select_next_tab(&mut self) {
        let Some(pane_id) = self.focused_pane_id else {
            return;
        };
        let Some(pane) = self.root_node.find_pane_mut(pane_id) else {
            return;
        };
        if pane.tabs.is_empty() {
            return;
        }
        let Some(current) = pane.selected_tab_id else {
            return;
        };
        let Some(idx) = pane.tabs.iter().position(|t| t.id == current) else {
            return;
        };
        let new_idx = if idx + 1 >= pane.tabs.len() { 0 } else { idx + 1 };
        pane.selected_tab_id = Some(pane.tabs[new_idx].id);
    }

    // --- navigation -------------------------------------------------------

    /// Spatial neighbour of the focused pane. Mirrors Swift
    /// `navigateFocus(direction:)`.
    pub fn navigate_focus(&mut self, direction: NavigationDirection) -> bool {
        let Some(current) = self.focused_pane_id else {
            return false;
        };
        let Some(target) = self.adjacent_pane(current, direction) else {
            return false;
        };
        self.focus_pane(target);
        true
    }

    /// Find the closest pane in `direction` from the given pane, using
    /// the same "highest overlap, then shortest distance" metric as the
    /// Swift implementation.
    pub fn adjacent_pane(
        &self,
        pane_id: PaneId,
        direction: NavigationDirection,
    ) -> Option<PaneId> {
        let bounds = self.root_node.compute_pane_bounds();
        let current = bounds.iter().find(|b| b.pane_id == pane_id)?.bounds;
        find_best_neighbor(current, pane_id, direction, &bounds)
    }

    // --- split state accessors --------------------------------------------

    /// Find a split by id. Returns `None` for a leaf root.
    pub fn find_split_mut(&mut self, split_id: uuid::Uuid) -> Option<&mut SplitState> {
        self.root_node.find_split_mut(split_id)
    }

    /// Update a split's divider position, clamped into `[0.0, 1.0]`.
    pub fn set_divider_position(&mut self, split_id: uuid::Uuid, position: f64) -> bool {
        let Some(split) = self.root_node.find_split_mut(split_id) else {
            return false;
        };
        let clamped = position.clamp(0.0, 1.0);
        if (split.divider_position - clamped).abs() < f64::EPSILON {
            return false;
        }
        split.divider_position = clamped;
        true
    }
}

// --- recursive helpers ----------------------------------------------------

fn split_rec(
    node: &mut SplitNode,
    target_pane_id: PaneId,
    orientation: SplitOrientation,
    new_tab: Option<TabItem>,
) -> Option<PaneId> {
    match node {
        SplitNode::Pane(pane_state) => {
            if pane_state.id != target_pane_id {
                return None;
            }
            let existing = std::mem::replace(pane_state, PaneState::empty());
            let new_pane = match new_tab {
                Some(tab) => PaneState::new(vec![tab]),
                None => PaneState::empty(),
            };
            let new_id = new_pane.id;
            let split = SplitState::new(
                orientation,
                SplitNode::leaf(existing),
                SplitNode::leaf(new_pane),
            );
            *node = SplitNode::Split(split);
            Some(new_id)
        }
        SplitNode::Split(split_state) => split_rec(&mut split_state.first, target_pane_id, orientation, new_tab.clone())
            .or_else(|| split_rec(&mut split_state.second, target_pane_id, orientation, new_tab)),
    }
}

fn split_with_tab_rec(
    node: &mut SplitNode,
    target_pane_id: PaneId,
    orientation: SplitOrientation,
    tab: TabItem,
    insert_first: bool,
) -> Option<PaneId> {
    match node {
        SplitNode::Pane(pane_state) => {
            if pane_state.id != target_pane_id {
                return None;
            }
            let existing = std::mem::replace(pane_state, PaneState::empty());
            let new_pane = PaneState::new(vec![tab]);
            let new_id = new_pane.id;
            let split = if insert_first {
                SplitState::new(
                    orientation,
                    SplitNode::leaf(new_pane),
                    SplitNode::leaf(existing),
                )
            } else {
                SplitState::new(
                    orientation,
                    SplitNode::leaf(existing),
                    SplitNode::leaf(new_pane),
                )
            };
            *node = SplitNode::Split(split);
            Some(new_id)
        }
        SplitNode::Split(split_state) => {
            if let Some(id) = split_with_tab_rec(
                &mut split_state.first,
                target_pane_id,
                orientation,
                tab.clone(),
                insert_first,
            ) {
                return Some(id);
            }
            split_with_tab_rec(
                &mut split_state.second,
                target_pane_id,
                orientation,
                tab,
                insert_first,
            )
        }
    }
}

/// Remove `target_pane_id` and collapse its parent split. Returns the
/// new root (or the same node if the target wasn't found). Mirrors
/// Swift's `closePaneRecursively`, which returns `(newNode, siblingFocusId)`.
fn close_pane_rec(
    node: SplitNode,
    target_pane_id: PaneId,
    focus_target: &mut Option<PaneId>,
) -> Option<SplitNode> {
    match node {
        SplitNode::Pane(state) => {
            if state.id == target_pane_id {
                None
            } else {
                Some(SplitNode::Pane(state))
            }
        }
        SplitNode::Split(split) => {
            // Unbox children for easier matching.
            let SplitState {
                id,
                orientation,
                first,
                second,
                divider_position,
            } = split;
            let first = *first;
            let second = *second;

            // Direct child pane hits collapse the split.
            if let SplitNode::Pane(ref first_pane) = first {
                if first_pane.id == target_pane_id {
                    *focus_target = second.all_pane_ids().first().copied();
                    return Some(second);
                }
            }
            if let SplitNode::Pane(ref second_pane) = second {
                if second_pane.id == target_pane_id {
                    *focus_target = first.all_pane_ids().first().copied();
                    return Some(first);
                }
            }

            // Recurse on the first child.
            let new_first = close_pane_rec(first, target_pane_id, focus_target);
            let new_first = match new_first {
                Some(n) => n,
                None => {
                    // first collapsed into nothing → second becomes the new subtree.
                    *focus_target = second.all_pane_ids().first().copied();
                    return Some(second);
                }
            };

            // Recurse on the second child.
            let new_second = close_pane_rec(second, target_pane_id, focus_target);
            let new_second = match new_second {
                Some(n) => n,
                None => {
                    // second collapsed into nothing → first becomes the new subtree.
                    *focus_target = new_first.all_pane_ids().first().copied();
                    return Some(new_first);
                }
            };

            Some(SplitNode::Split(SplitState {
                id,
                orientation,
                first: Box::new(new_first),
                second: Box::new(new_second),
                divider_position,
            }))
        }
    }
}

fn find_best_neighbor(
    current_bounds: super::types::UnitRect,
    current_pane_id: PaneId,
    direction: NavigationDirection,
    all_pane_bounds: &[PaneBounds],
) -> Option<PaneId> {
    const EPSILON: f64 = 0.001;

    let candidates: Vec<&PaneBounds> = all_pane_bounds
        .iter()
        .filter(|pb| {
            if pb.pane_id == current_pane_id {
                return false;
            }
            let b = pb.bounds;
            match direction {
                NavigationDirection::Left => b.max_x() <= current_bounds.min_x() + EPSILON,
                NavigationDirection::Right => b.min_x() >= current_bounds.max_x() - EPSILON,
                NavigationDirection::Up => b.max_y() <= current_bounds.min_y() + EPSILON,
                NavigationDirection::Down => b.min_y() >= current_bounds.max_y() - EPSILON,
            }
        })
        .collect();

    if candidates.is_empty() {
        return None;
    }

    let mut scored: Vec<(PaneId, f64, f64)> = candidates
        .iter()
        .map(|pb| {
            let b = pb.bounds;
            let (overlap, distance) = match direction {
                NavigationDirection::Left | NavigationDirection::Right => {
                    let overlap = (current_bounds.max_y().min(b.max_y())
                        - current_bounds.min_y().max(b.min_y()))
                    .max(0.0);
                    let distance = match direction {
                        NavigationDirection::Left => current_bounds.min_x() - b.max_x(),
                        NavigationDirection::Right => b.min_x() - current_bounds.max_x(),
                        _ => unreachable!(),
                    };
                    (overlap, distance)
                }
                NavigationDirection::Up | NavigationDirection::Down => {
                    let overlap = (current_bounds.max_x().min(b.max_x())
                        - current_bounds.min_x().max(b.min_x()))
                    .max(0.0);
                    let distance = match direction {
                        NavigationDirection::Up => current_bounds.min_y() - b.max_y(),
                        NavigationDirection::Down => b.min_y() - current_bounds.max_y(),
                        _ => unreachable!(),
                    };
                    (overlap, distance)
                }
            };
            (pb.pane_id, overlap, distance)
        })
        .collect();

    scored.sort_by(|a, b| {
        if (a.1 - b.1).abs() > EPSILON {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal)
        }
    });

    scored.first().map(|(id, _, _)| *id)
}
