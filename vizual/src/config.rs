use std::time::Duration;

use crate::geometry::Size;

pub(crate) const DEFAULT_FONT_SIZE: f32 = 16.0;

// TODO: It might be better to use the platform's default window size instead.
pub(crate) const DEFAULT_SCREEN_SIZE: Size = Size::new(800.0, 600.0);

// TODO: solve this some other way
pub(crate) const MAXIMUM_LAYOUT_VALUE: f64 = 21_000.0;

pub(crate) const BORDER_SIZE: f64 = 1.0;
pub(crate) const SCROLLBAR_SIZE: f64 = 1.0;

pub(crate) const COMMAND_WAIT_TIMEOUT: Duration = Duration::from_secs(5);
