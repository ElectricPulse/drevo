//! Priority levels for layout objectives.
//!
//! Vizual uses weighted priorities rather than HiGHS' lexicographic priorities.
//! Because variables represent screen coordinates or dimensions within known bounds,
//! priorities use a geometric scale: priority `p` uses `BLENDED_GOAL_WEIGHT.powi(p)`.
//!
//! Priorities are ordered such that lower numbers have higher precedence (higher weight in the objective):
//! - Priority 0: Tightest precedence (highest objective weight).
//! - Priority 1: Intermediate precedence.
//! - Priority 2: Flexible precedence (lowest objective weight).
//!
//! ### Layout Objective Strategy
//! When configuring layout objectives across nested widgets, it is best to:
//! 1. Put a **minimize goal on axis cross at 0** (`CROSS_AXIS_LIMIT` / `SHRINK_WRAP`). This ensures
//!    parent containers shrink-wrap tightly around child elements on their cross axis.
//! 2. Set **maximize on content at 1** (`CONTENT` / `FILL`). Content and scrollable
//!    viewports expand to fill available parent space after cross limits and shrink-wrapping are established.
//! 3. Set **maximize on padding and margin at 2** (`PADDING` / `MARGIN` / `GAP` / `SPACING`).
//!    Flexible spacing and gaps absorb remaining space or flex gracefully without overriding content
//!    expansion or cross-axis shrink-wrapping.

pub const PRIORITY_LEVELS: usize = 3;

/// Priority 0: Tightest constraints; minimize goal on axis cross to shrink-wrap parents around
/// their children, main-axis hugging, and edge alignment.
pub const CROSS_AXIS_LIMIT: usize = 0;
pub const CROSS_AXIS: usize = 0;
pub const SHRINK_WRAP: usize = 0;
pub const ALIGNMENT: usize = 0;
pub const FIT: usize = 0;
pub const MAIN_AXIS: usize = 0;

/// Priority 1: Content maximization; expanding content and widgets (such as scroll containers)
/// to fill available layout space.
pub const CONTENT: usize = 1;
pub const FILL: usize = 1;

/// Priority 2: Flexible spacing; maximize on padding and margin, gaps, deltas, and window/root minimization.
pub const PADDING: usize = 2;
pub const MARGIN: usize = 2;
pub const GAP: usize = 2;
pub const GAP_PRIORITY: usize = 2;
pub const SPACING: usize = 2;
pub const ROOT_MINIMIZATION: usize = 2;
