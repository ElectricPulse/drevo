//! Priority levels for layout objectives.
//!
//! Vizual uses weighted priorities rather than HiGHS' lexicographic priorities.
//! Because variables represent screen coordinates or dimensions within known bounds,
//! priorities use a geometric scale: priority `p` uses `BLENDED_GOAL_WEIGHT.powi(p)`.
//!
//! Priorities are ordered such that lower numbers have higher precedence (higher weight in the objective):
//! - Priority 0 (`POSITIONING`): Anchor minimization of space and edge alignment.
//! - Priority 1 (`EXTRA_CONTENT`): Minimizing child content overflow beyond the parent container in scroll viewports.
//! - Priority 2 (`CONTENT`): Minimizing content growth slack so parent and child fit content dimensions.
//! - Priority 3 (`SPACING`): Flexible spacing, margins, padding, gaps, deltas, and window/root minimization.

pub const PRIORITY_LEVELS: usize = 4;

/// Priority 0: Positioning; anchor minimization of space, edge alignment, and shrink-wrapping.
pub const POSITIONING: usize = 0;
pub const ALIGNMENT: usize = 0;
pub const SHRINK_WRAP: usize = 0;

/// Priority 1: Extra content; minimizing child content overflow beyond the parent container in scroll viewports.
pub const EXTRA_CONTENT: usize = 1;

/// Priority 2: Content sizing; minimizing content growth slack so parent and child match content dimensions.
pub const CONTENT: usize = 2;
pub const FILL: usize = 2;

/// Priority 3: Flexible spacing; padding, margins, gaps, deltas, and window/root minimization.
pub const SPACING: usize = 3;
pub const PADDING: usize = 3;
pub const MARGIN: usize = 3;
pub const GAP: usize = 3;
pub const GAP_PRIORITY: usize = 3;
pub const ROOT_MINIMIZATION: usize = 3;
