//! - (`ALIGNMENT`, `SHRINK_WRAP`): Edge alignment and shrink-wrapping.
//! - (`EXCESS_SPACE`): Priority at which excess space tries to maximize.
//! - (`CROSS_AXIS_LIMIT`): Axis cross-axis limiting.
//! - (`INTRINSIC_CONTENT`): Intrinsic content sizing.
//! - (`INTRINSIC_SPACING`): Flexible spacing, margins, padding, gaps, and deltas.
//! - (`ROOT_DIMENSIONS`): Minimizing extra root size beyond the actual window size.

// Higher values have higher precedence:
pub const PRIORITY_LEVELS: usize = 5;

// TODO: Reconcile whether this can stay at 0.
pub const ALIGNMENT: usize = 0;
pub const SHRINK_WRAP: usize = 0;

pub const CROSS_AXIS_LIMIT: usize = 0;

pub const EXCESS_SPACE: usize = 1;

pub const INTRINSIC_CONTENT: usize = 2;

pub const INTRINSIC_SPACING: usize = 3;

pub const ROOT_DIMENSIONS: usize = 4;
