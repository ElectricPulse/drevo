//! - (`ALIGNMENT`): Edge alignment.
//! - (`EXCESS_SPACE`): Priority at which excess space tries to maximize.
//! - (`ABOVE_EXCESS_SPACE`): Objectives that must outrank ordinary excess-space growth.
//! - (`CROSS_AXIS_LIMIT`): Axis cross-axis limiting.
//! - (`INTRINSIC_CONTENT`): Intrinsic content sizing.
//! - (`INTRINSIC_SPACING`): Flexible spacing, margins, padding, gaps, and deltas.
//! - (`ROOT_DIMENSIONS`): Minimizing extra root size beyond the actual window size.

// Higher values have higher precedence:
pub const PRIORITY_LEVELS: usize = 7;

pub const ALIGNMENT: usize = 0;

pub const CROSS_AXIS_LIMIT: usize = 1;

pub const EXCESS_SPACE: usize = 2;

pub const ABOVE_EXCESS_SPACE: usize = 3;

pub const INTRINSIC_CONTENT: usize = 4;

pub const INTRINSIC_SPACING: usize = 5;

pub const ROOT_DIMENSIONS: usize = 6;
