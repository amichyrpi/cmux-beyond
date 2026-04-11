//! Opaque ID types and small enums shared by the rest of the bonsplit port.
//!
//! Ported from:
//! - [vendor/bonsplit/Sources/Bonsplit/Public/Types/TabID.swift](../../../../../../vendor/bonsplit/Sources/Bonsplit/Public/Types/TabID.swift)
//! - [vendor/bonsplit/Sources/Bonsplit/Public/Types/PaneID.swift](../../../../../../vendor/bonsplit/Sources/Bonsplit/Public/Types/PaneID.swift)
//! - [vendor/bonsplit/Sources/Bonsplit/Public/Types/SplitOrientation.swift](../../../../../../vendor/bonsplit/Sources/Bonsplit/Public/Types/SplitOrientation.swift)
//! - [vendor/bonsplit/Sources/Bonsplit/Public/Types/NavigationDirection.swift](../../../../../../vendor/bonsplit/Sources/Bonsplit/Public/Types/NavigationDirection.swift)

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Opaque identifier for tabs. Mirrors Swift `TabID`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct TabId(pub Uuid);

impl TabId {
    /// Allocate a fresh random id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Wrap an existing uuid (used by session restore).
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Return the inner uuid.
    pub fn uuid(self) -> Uuid {
        self.0
    }
}

impl Default for TabId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TabId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Opaque identifier for panes. Mirrors Swift `PaneID`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct PaneId(pub Uuid);

impl PaneId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn uuid(self) -> Uuid {
        self.0
    }
}

impl Default for PaneId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PaneId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Orientation for splitting panes. Mirrors Swift `SplitOrientation`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SplitOrientation {
    /// Side-by-side split (left | right).
    Horizontal,
    /// Stacked split (top / bottom).
    Vertical,
}

/// Keyboard / programmatic navigation between panes.
/// Mirrors Swift `NavigationDirection`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavigationDirection {
    Left,
    Right,
    Up,
    Down,
}

/// Normalised 0..1 rectangle used for in-tree bounds computation.
///
/// Swift's version uses `CGRect`; here we keep an f64 rect that is
/// platform-free so the same math runs on macOS, Linux and Windows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl UnitRect {
    /// The whole unit square `(0,0)..(1,1)`.
    pub const UNIT: UnitRect = UnitRect {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    };

    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn min_x(&self) -> f64 {
        self.x
    }
    pub fn min_y(&self) -> f64 {
        self.y
    }
    pub fn max_x(&self) -> f64 {
        self.x + self.width
    }
    pub fn max_y(&self) -> f64 {
        self.y + self.height
    }
}
