use crate::{state::State, theme::Theme};

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

    pub fn get(&self, theme: &State<Theme>) -> ConcreteStyle {
        self.concrete
            .clone()
            .unwrap_or_else(|| ConcreteStyle::from((*theme.load()).clone()))
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
    Dark_gray,
    Gray,
    Light_red,
    Light_green,
    Light_yellow,
    Light_blue,
    Light_magenta,
    Light_cyan,
    Rgb(u8, u8, u8),
    Indexed(u8),
}

impl Color {
    pub(crate) fn to_peniko(self) -> vello::peniko::Color {
        let (red, green, blue) = match self {
            Self::Black => (0, 0, 0),
            Self::Red => (205, 49, 49),
            Self::Green => (13, 188, 121),
            Self::Yellow => (229, 229, 16),
            Self::Blue => (36, 114, 200),
            Self::Magenta => (188, 63, 188),
            Self::Cyan => (17, 168, 205),
            Self::White => (229, 229, 229),
            Self::Dark_gray => (102, 102, 102),
            Self::Gray => (128, 128, 128),
            Self::Light_red => (241, 76, 76),
            Self::Light_green => (35, 209, 139),
            Self::Light_yellow => (245, 245, 67),
            Self::Light_blue => (59, 142, 234),
            Self::Light_magenta => (214, 112, 214),
            Self::Light_cyan => (41, 184, 219),
            Self::Rgb(red, green, blue) => (red, green, blue),
            Self::Indexed(index) => indexed_color(index),
        };

        vello::peniko::Color::from_rgb8(red, green, blue)
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
