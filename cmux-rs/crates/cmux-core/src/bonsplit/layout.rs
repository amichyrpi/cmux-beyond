//! Pixel-space layout snapshots that bridge Rust → Tauri → TypeScript.
//!
//! Ported from
//! [vendor/bonsplit/Sources/Bonsplit/Public/Types/LayoutSnapshot.swift](../../../../../../vendor/bonsplit/Sources/Bonsplit/Public/Types/LayoutSnapshot.swift).
//!
//! The Swift version serializes these to JSON to hand off to the app's
//! AppleScript / socket bridge. The Rust port keeps the exact same wire
//! shape so existing `tests_v2/` fixtures and the new React frontend can
//! consume them without a translation layer.

use serde::{Deserialize, Serialize};

use super::model::{PaneState, SplitNode, SplitState};
use super::types::{PaneId, SplitOrientation, UnitRect};

/// Pixel rectangle for external consumption. Mirrors Swift `PixelRect`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PixelRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl PixelRect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Project a normalised `(0..1)` rect onto a container frame.
    pub fn project(container: PixelRect, normalized: UnitRect) -> Self {
        Self {
            x: container.x + normalized.x * container.width,
            y: container.y + normalized.y * container.height,
            width: normalized.width * container.width,
            height: normalized.height * container.height,
        }
    }
}

/// Geometry + tab identifiers for a single pane. Mirrors Swift
/// `PaneGeometry`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaneGeometry {
    #[serde(rename = "paneId")]
    pub pane_id: String,
    pub frame: PixelRect,
    #[serde(rename = "selectedTabId")]
    pub selected_tab_id: Option<String>,
    #[serde(rename = "tabIds")]
    pub tab_ids: Vec<String>,
}

/// Full tree snapshot with pixel coordinates. Mirrors Swift
/// `LayoutSnapshot`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutSnapshot {
    #[serde(rename = "containerFrame")]
    pub container_frame: PixelRect,
    pub panes: Vec<PaneGeometry>,
    #[serde(rename = "focusedPaneId")]
    pub focused_pane_id: Option<String>,
    pub timestamp: f64,
}

impl LayoutSnapshot {
    /// Build a snapshot from the authoritative root node. The container
    /// frame is applied to normalised bounds to produce pixel rects.
    pub fn from_root(
        root: &SplitNode,
        container: PixelRect,
        focused: Option<PaneId>,
        timestamp: f64,
    ) -> Self {
        let bounds = root.compute_pane_bounds();
        let panes = bounds
            .into_iter()
            .filter_map(|b| {
                root.find_pane(b.pane_id).map(|pane| PaneGeometry {
                    pane_id: pane.id.uuid().to_string(),
                    frame: PixelRect::project(container, b.bounds),
                    selected_tab_id: pane.selected_tab_id.map(|t| t.uuid().to_string()),
                    tab_ids: pane.tabs.iter().map(|t| t.id.uuid().to_string()).collect(),
                })
            })
            .collect();

        Self {
            container_frame: container,
            panes,
            focused_pane_id: focused.map(|p| p.uuid().to_string()),
            timestamp,
        }
    }
}

/// External representation of a tab (id + title only). Mirrors Swift
/// `ExternalTab`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalTab {
    pub id: String,
    pub title: String,
}

/// External representation of a pane node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalPaneNode {
    pub id: String,
    pub frame: PixelRect,
    pub tabs: Vec<ExternalTab>,
    #[serde(rename = "selectedTabId")]
    pub selected_tab_id: Option<String>,
}

/// External representation of a split node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalSplitNode {
    pub id: String,
    /// `"horizontal"` or `"vertical"` — kept as a free-form string to
    /// match Swift's `String`-typed field exactly.
    pub orientation: String,
    #[serde(rename = "dividerPosition")]
    pub divider_position: f64,
    pub first: Box<ExternalTreeNode>,
    pub second: Box<ExternalTreeNode>,
}

/// External representation of a split tree. Uses the same tagged
/// encoding as Swift's `ExternalTreeNode`:
/// `{ "type": "pane", "pane": { ... } }` / `{ "type": "split", "split": { ... } }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ExternalTreeNode {
    Pane {
        pane: ExternalPaneNode,
    },
    Split {
        split: ExternalSplitNode,
    },
}

impl ExternalTreeNode {
    /// Build the external tree from the authoritative root node,
    /// projected into a container pixel frame.
    pub fn from_root(root: &SplitNode, container: PixelRect) -> Self {
        Self::from_node_rec(root, UnitRect::UNIT, container)
    }

    fn from_node_rec(node: &SplitNode, rect: UnitRect, container: PixelRect) -> Self {
        match node {
            SplitNode::Pane(pane) => Self::Pane {
                pane: external_pane(pane, PixelRect::project(container, rect)),
            },
            SplitNode::Split(split) => {
                let p = split.divider_position.clamp(0.0, 1.0);
                let (first_rect, second_rect) = match split.orientation {
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
                Self::Split {
                    split: external_split(
                        split,
                        Self::from_node_rec(&split.first, first_rect, container),
                        Self::from_node_rec(&split.second, second_rect, container),
                    ),
                }
            }
        }
    }
}

fn external_pane(pane: &PaneState, frame: PixelRect) -> ExternalPaneNode {
    ExternalPaneNode {
        id: pane.id.uuid().to_string(),
        frame,
        tabs: pane
            .tabs
            .iter()
            .map(|t| ExternalTab {
                id: t.id.uuid().to_string(),
                title: t.title.clone(),
            })
            .collect(),
        selected_tab_id: pane.selected_tab_id.map(|t| t.uuid().to_string()),
    }
}

fn external_split(
    split: &SplitState,
    first: ExternalTreeNode,
    second: ExternalTreeNode,
) -> ExternalSplitNode {
    ExternalSplitNode {
        id: split.id.to_string(),
        orientation: match split.orientation {
            SplitOrientation::Horizontal => "horizontal".to_string(),
            SplitOrientation::Vertical => "vertical".to_string(),
        },
        divider_position: split.divider_position,
        first: Box::new(first),
        second: Box::new(second),
    }
}
