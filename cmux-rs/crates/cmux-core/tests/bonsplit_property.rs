//! Property-style fuzz test for the bonsplit model. The Swift version
//! has no equivalent: SwiftUI tests go through the real AppKit tree,
//! which is not available cross-platform. This test drives the Rust
//! controller through a deterministic sequence of pseudo-random
//! operations and verifies the structural invariants after each step.
//!
//! Required by the Phase 4 checklist in `PLAN.md`.

use cmux_core::bonsplit::{
    NavigationDirection, PaneId, SplitNode, SplitOrientation, SplitViewController, TabItem,
};

/// Tiny deterministic LCG — we do **not** want `rand` as a test dep.
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(1))
    }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
    fn below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            0
        } else {
            (self.next() as usize) % bound
        }
    }
}

/// Run a structural check over the controller. Panics on violation so
/// the calling test pinpoints the failing seed/step.
fn assert_invariants(controller: &SplitViewController, step: usize, seed: u64) {
    let root = controller.root();

    // Every pane in the tree has a unique id.
    let pane_ids = root.all_pane_ids();
    let mut sorted = pane_ids.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        pane_ids.len(),
        sorted.len(),
        "seed={seed} step={step}: duplicate pane id in tree"
    );

    // At least one pane must always exist.
    assert!(
        !pane_ids.is_empty(),
        "seed={seed} step={step}: tree collapsed to zero panes"
    );

    // Every normalised pane rect must be inside the unit square and
    // together the areas must sum to ~1.0 (they partition the unit
    // square). We allow slack for floating-point drift.
    let bounds = root.compute_pane_bounds();
    let mut total_area = 0.0f64;
    for b in &bounds {
        assert!(
            b.bounds.x >= -1e-9 && b.bounds.y >= -1e-9,
            "seed={seed} step={step}: pane rect escaped top-left"
        );
        assert!(
            b.bounds.max_x() <= 1.0 + 1e-9 && b.bounds.max_y() <= 1.0 + 1e-9,
            "seed={seed} step={step}: pane rect escaped bottom-right ({:?})",
            b.bounds
        );
        assert!(
            b.bounds.width >= -1e-9 && b.bounds.height >= -1e-9,
            "seed={seed} step={step}: negative rect dims"
        );
        total_area += b.bounds.width * b.bounds.height;
    }
    assert!(
        (total_area - 1.0).abs() < 1e-6,
        "seed={seed} step={step}: pane areas don't cover unit square (got {total_area})"
    );

    // Every pane's selection is a tab that exists in the pane.
    for pane in root.all_panes() {
        if let Some(selected) = pane.selected_tab_id {
            assert!(
                pane.tabs.iter().any(|t| t.id == selected),
                "seed={seed} step={step}: selected tab not found in pane"
            );
        } else {
            assert!(
                pane.tabs.is_empty(),
                "seed={seed} step={step}: non-empty pane with no selection"
            );
        }
    }

    // Focused pane id is always a valid pane (or None in the degenerate
    // seed=0 edge case, which we preclude by constructing with a tab).
    if let Some(focused) = controller.focused_pane_id() {
        assert!(
            root.find_pane(focused).is_some(),
            "seed={seed} step={step}: focused_pane_id points outside the tree"
        );
    }
}

fn random_pane(controller: &SplitViewController, rng: &mut Lcg) -> Option<PaneId> {
    let panes = controller.root().all_pane_ids();
    if panes.is_empty() {
        return None;
    }
    Some(panes[rng.below(panes.len())])
}

#[test]
fn fuzz_controller_operations() {
    // 40 seeds × 200 ops = 8k operations exercised. Cheap and
    // deterministic — rerun a failure by copying the seed into a
    // one-off test.
    for seed in 0..40u64 {
        let mut controller = SplitViewController::new_welcome();
        let mut rng = Lcg::new(seed);

        for step in 0..200usize {
            let Some(pane) = random_pane(&controller, &mut rng) else {
                break;
            };
            match rng.below(10) {
                0 | 1 => {
                    let orientation = if rng.below(2) == 0 {
                        SplitOrientation::Horizontal
                    } else {
                        SplitOrientation::Vertical
                    };
                    let tab = if rng.below(2) == 0 {
                        Some(TabItem::new(format!("t{step}")))
                    } else {
                        None
                    };
                    controller.split_pane(pane, orientation, tab);
                }
                2 => {
                    controller.close_pane(pane);
                }
                3 | 4 => {
                    controller.add_tab(
                        TabItem::new(format!("added-{step}")),
                        Some(pane),
                        None,
                    );
                }
                5 => {
                    // Close the first tab in this pane if it has any.
                    if let Some(tab_id) = controller
                        .root()
                        .find_pane(pane)
                        .and_then(|p| p.tabs.first().map(|t| t.id))
                    {
                        controller.close_tab(tab_id, pane);
                    }
                }
                6 => {
                    let direction = match rng.below(4) {
                        0 => NavigationDirection::Left,
                        1 => NavigationDirection::Right,
                        2 => NavigationDirection::Up,
                        _ => NavigationDirection::Down,
                    };
                    controller.navigate_focus(direction);
                }
                7 => {
                    controller.focus_pane(pane);
                    controller.toggle_pane_zoom(pane);
                }
                8 => {
                    // Random divider drag for a random split.
                    let splits = controller.root().all_splits().iter().map(|s| s.id).collect::<Vec<_>>();
                    if !splits.is_empty() {
                        let split_id = splits[rng.below(splits.len())];
                        let target = (rng.below(9) as f64 + 1.0) / 10.0;
                        controller.set_divider_position(split_id, target);
                    }
                }
                _ => {
                    if rng.below(2) == 0 {
                        controller.select_next_tab();
                    } else {
                        controller.select_previous_tab();
                    }
                }
            }

            assert_invariants(&controller, step, seed);
        }
    }
}

#[test]
fn split_produces_correct_geometry() {
    let mut controller = SplitViewController::new_welcome();
    let root_pane = controller.root().all_pane_ids()[0];
    let new_pane = controller
        .split_pane(root_pane, SplitOrientation::Horizontal, None)
        .expect("split should produce a new pane");

    let bounds = controller.pane_bounds();
    assert_eq!(bounds.len(), 2);
    let first = bounds.iter().find(|b| b.pane_id == root_pane).unwrap();
    let second = bounds.iter().find(|b| b.pane_id == new_pane).unwrap();
    assert!((first.bounds.width - 0.5).abs() < 1e-6);
    assert!((second.bounds.width - 0.5).abs() < 1e-6);
    assert!((first.bounds.x - 0.0).abs() < 1e-6);
    assert!((second.bounds.x - 0.5).abs() < 1e-6);
}

#[test]
fn close_pane_collapses_parent_split() {
    let mut controller = SplitViewController::new_welcome();
    let root_pane = controller.root().all_pane_ids()[0];
    let sibling = controller
        .split_pane(root_pane, SplitOrientation::Vertical, Some(TabItem::new("x")))
        .unwrap();

    assert_eq!(controller.root().all_pane_ids().len(), 2);
    controller.close_pane(sibling);
    assert_eq!(controller.root().all_pane_ids().len(), 1);
    // After collapse the root must be a leaf.
    assert!(matches!(controller.root(), SplitNode::Pane(_)));
}

#[test]
fn move_tab_between_panes_closes_empty_source() {
    let mut controller = SplitViewController::new_empty();
    let source_pane = controller.root().all_pane_ids()[0];

    // Put one tab in the source pane.
    let tab = TabItem::new("only");
    let tab_id = tab.id;
    controller.add_tab(tab, Some(source_pane), None);

    // Split so we have a target pane.
    let target_pane = controller
        .split_pane(source_pane, SplitOrientation::Horizontal, None)
        .unwrap();

    controller.move_tab(tab_id, source_pane, target_pane, None);

    // The source pane should have been collapsed since it became empty.
    assert_eq!(controller.root().all_pane_ids(), vec![target_pane]);
    // And the target pane now owns that tab.
    let pane = controller.root().find_pane(target_pane).unwrap();
    assert_eq!(pane.tabs.len(), 1);
    assert_eq!(pane.tabs[0].id, tab_id);
}

#[test]
fn navigate_focus_moves_spatially() {
    let mut controller = SplitViewController::new_welcome();
    let left = controller.root().all_pane_ids()[0];
    let right = controller
        .split_pane(left, SplitOrientation::Horizontal, Some(TabItem::new("r")))
        .unwrap();

    controller.focus_pane(left);
    assert!(controller.navigate_focus(NavigationDirection::Right));
    assert_eq!(controller.focused_pane_id(), Some(right));
    assert!(controller.navigate_focus(NavigationDirection::Left));
    assert_eq!(controller.focused_pane_id(), Some(left));
}

#[test]
fn zoom_requires_multiple_panes() {
    let mut controller = SplitViewController::new_welcome();
    let only = controller.root().all_pane_ids()[0];
    assert!(!controller.toggle_pane_zoom(only));

    let other = controller
        .split_pane(only, SplitOrientation::Vertical, Some(TabItem::new("o")))
        .unwrap();
    assert!(controller.toggle_pane_zoom(only));
    assert_eq!(controller.zoomed_pane_id(), Some(only));

    // Closing the zoomed pane clears the zoom state.
    controller.close_pane(only);
    assert!(controller.zoomed_pane_id().is_none());
    assert!(controller.root().find_pane(other).is_some());
}
