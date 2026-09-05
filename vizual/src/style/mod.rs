use crate::theme::Theme;
pub use drevo_macros::Style;

#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct Style<ConcreteStyle: From<Theme>> {
    concrete: Option<ConcreteStyle>,
}

impl<ConcreteStyle: From<Theme>> Default for Style<ConcreteStyle> {
    fn default() -> Self {
        Self { concrete: None }
    }
}

impl<ConcreteStyle: From<Theme> + Clone> Style<ConcreteStyle> {
    pub fn set(&mut self, concrete: ConcreteStyle) {
        self.concrete = Some(concrete);
    }

    pub fn get(&self, theme: &Theme) -> ConcreteStyle {
        self.concrete
            .clone()
            .unwrap_or_else(|| ConcreteStyle::from(theme.clone()))
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    #[default]
    White,
    DarkGray,
    Gray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    Rgb(u8, u8, u8),
    Rgba(u8, u8, u8, u8),
    Indexed(u8),
}

impl Color {
    pub fn lighten(self, amount: u8) -> Self {
        self.map_channels(|channel| channel.saturating_add(amount))
    }

    pub fn darken(self, amount: u8) -> Self {
        self.map_channels(|channel| channel.saturating_sub(amount))
    }

    fn map_channels(self, map: impl Fn(u8) -> u8) -> Self {
        let preserves_alpha = matches!(self, Self::Rgba(_, _, _, _));
        let (red, green, blue, alpha) = self.components();
        let (red, green, blue) = (map(red), map(green), map(blue));

        match preserves_alpha {
            true => Self::Rgba(red, green, blue, alpha),
            false => Self::Rgb(red, green, blue),
        }
    }

    pub(crate) fn to_peniko(self) -> vello::peniko::Color {
        let (red, green, blue, alpha) = self.components();
        vello::peniko::Color::from_rgba8(red, green, blue, alpha)
    }

    fn components(self) -> (u8, u8, u8, u8) {
        match self {
            Self::Black => (0, 0, 0, 255),
            Self::Red => (205, 49, 49, 255),
            Self::Green => (13, 188, 121, 255),
            Self::Yellow => (229, 229, 16, 255),
            Self::Blue => (36, 114, 200, 255),
            Self::Magenta => (188, 63, 188, 255),
            Self::Cyan => (17, 168, 205, 255),
            Self::White => (229, 229, 229, 255),
            Self::DarkGray => (102, 102, 102, 255),
            Self::Gray => (128, 128, 128, 255),
            Self::LightRed => (241, 76, 76, 255),
            Self::LightGreen => (35, 209, 139, 255),
            Self::LightYellow => (245, 245, 67, 255),
            Self::LightBlue => (59, 142, 234, 255),
            Self::LightMagenta => (214, 112, 214, 255),
            Self::LightCyan => (41, 184, 219, 255),
            Self::Rgb(red, green, blue) => (red, green, blue, 255),
            Self::Rgba(red, green, blue, alpha) => (red, green, blue, alpha),
            Self::Indexed(index) => {
                let (red, green, blue) = indexed_color(index);
                (red, green, blue, 255)
            }
        }
    }
}

fn indexed_color(index: u8) -> (u8, u8, u8) {
    const ANSI: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (205, 49, 49),
        (13, 188, 121),
        (229, 229, 16),
        (36, 114, 200),
        (188, 63, 188),
        (17, 168, 205),
        (229, 229, 229),
        (102, 102, 102),
        (241, 76, 76),
        (35, 209, 139),
        (245, 245, 67),
        (59, 142, 234),
        (214, 112, 214),
        (41, 184, 219),
        (255, 255, 255),
    ];

    match index {
        0..=15 => ANSI[index as usize],
        16..=231 => {
            let value = index - 16;
            let red = value / 36;
            let green = value % 36 / 6;
            let blue = value % 6;
            let channel = |component: u8| match component {
                0 => 0,
                value => 55 + 40 * value,
            };
            (channel(red), channel(green), channel(blue))
        }
        232..=255 => {
            let gray = 8 + 10 * (index - 232);
            (gray, gray, gray)
        }
    }
}
