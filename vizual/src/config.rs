use std::time::Duration;

use crate::geometry::Size;

pub(crate) const DEFAULT_FONT_SIZE: f32 = 16.0;

// TODO: It might be better to use the platform's default window size instead.
pub(crate) const DEFAULT_SCREEN_SIZE: Size = Size::new(800.0, 600.0);

// Since there is not yet a universal mechanism for window size negotiation where the OS
// asks "can you do this size?" and the window answers, we set the minimum window size to a static
// default. We are not letting content get clipped; instead the whole root is rendered in a scroll.
// 100x100 is chosen as the minimum size so that the scroll bars fit.
// That is because calculating a minimum window size needs to be done for each layout—which
// basically doubles the layout time—not to mention the fact that minimum window height depends
// on the current window width and vice versa. It is not a single minimum window size but viable window sizes.
// Besides allowing the window to be scrollable on really small devices while really annoying provides a last resort fallback instead of crashing
pub(crate) const MINIMUM_WINDOW_SIZE: Size = Size::new(100.0, 100.0);

// TODO: solve this some other way
pub(crate) const MAXIMUM_LAYOUT_VALUE: f64 = 21_000.0;

pub(crate) const MAX_ZOOM: f64 = 50.0;
/// At 1× scale, one logical pixel is one physical pixel; a full-HD display is 1920 pixels wide.
/// This weight is [`MAX_ZOOM`] times that so weights should survive that max zoom.
/// I am 99% sure that this calculation is partly bogus but I don't want to think about it ¯\_(ツ)_/¯
/// weights are then calculated as BLENDED_GOAL_WEIGHT to the power of priority (starting from lowest priority at 0)
pub(crate) const BLENDED_GOAL_WEIGHT: f64 = 1920.0 * MAX_ZOOM;

pub(crate) const BORDER_SIZE: f64 = 1.0;

pub(crate) const COMMAND_WAIT_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) const LAYOUT_TIMEOUT: Duration = Duration::from_millis(2);

/// Retains the last solved values and duals on formulas so the next rebuilt layout model can use
/// them as a HiGHS warm start. Disable this to compare cold layout solves.
/// Disabled as of now because if offers no performance benefit
pub(crate) const COPY_SOLUTION_TO_FORMULA: bool = false;
