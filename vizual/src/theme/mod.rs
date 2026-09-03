use crate::{
    config::BORDER_SIZE,
    style::Color,
    widget::widgets::{
        block::{BlockStyle, BorderStyle},
        button::ButtonStyle,
        paper::PaperStyle,
        text::TextStyle,
    },
};

#[cfg(test)]
mod tests;

#[derive(Clone, PartialEq)]
pub struct Theme {
    mode: SystemTheme,
    system: SystemTheme,
    follows_system: bool,
    pub units: Units,
    pub semantic: SemanticTokens,
    pub specific: SpecificTokens,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Units {
    pub em: f64,
}

#[derive(Clone, Copy, PartialEq)]
pub struct SemanticTokens {
    pub background: Color,
    pub surface: Color,
    pub border: Color,
    pub axis: AxisTheme,
    pub text: TextSemantic,
    /// Accent reserved for borders whose component actually owns focus or contains it.
    pub focus: Color,
}

#[derive(Clone, Copy, PartialEq)]
pub struct AxisTheme {
    pub gap: f64,
}

#[derive(Clone, Copy, PartialEq)]
pub struct TextSemantic {
    pub muted: Color,
    pub normal: Color,
}

#[derive(Clone, Copy, PartialEq)]
pub struct TextStyles {
    pub title: TextStyle,
    pub subtitle: TextStyle,
    pub paragraph: TextStyle,
    pub button: TextStyle,
}

#[derive(Clone, PartialEq)]
pub struct SpecificTokens {
    pub button: ButtonStyle,
    pub paper: PaperStyle,
    pub body: PaperStyle,
    pub header: BlockStyle,
    pub text: TextStyles,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemTheme {
    Light,
    Dark,
}

impl Theme {
    pub fn mode(&self) -> SystemTheme {
        self.mode
    }

    pub fn system(&self) -> SystemTheme {
        self.system
    }

    pub fn follows_system(&self) -> bool {
        self.follows_system
    }

    pub fn select(&self, mode: SystemTheme) -> Self {
        resolve_theme(mode, self.system, false)
    }

    pub fn follow_system(&self) -> Self {
        resolve_theme(self.system, self.system, true)
    }

    pub(crate) fn set_system(&self, system: SystemTheme) -> Self {
        let mode = match self.follows_system {
            true => system,
            false => self.mode,
        };
        resolve_theme(mode, system, self.follows_system)
    }
}

pub fn dark_theme() -> Theme {
    resolve_theme(SystemTheme::Dark, SystemTheme::Dark, false)
}

pub fn light_theme() -> Theme {
    resolve_theme(SystemTheme::Light, SystemTheme::Light, false)
}

pub(crate) fn system_theme(system: SystemTheme) -> Theme {
    resolve_theme(system, system, true)
}

fn resolve_theme(mode: SystemTheme, system: SystemTheme, follows_system: bool) -> Theme {
    let mut theme = match mode {
        SystemTheme::Dark => dark_tokens(),
        SystemTheme::Light => light_tokens(),
    };
    theme.mode = mode;
    theme.system = system;
    theme.follows_system = follows_system;
    theme
}

const DARK_STEP: u8 = 11;
const LIGHT_STEP: u8 = 5;
const GAP: f64 = 0.625;

fn dark_tokens() -> Theme {
    let units = Units { em: 16.0 };
    let background = Color::Rgb(20, 20, 20);
    let body_background = background.lighten(DARK_STEP);
    let semantic = SemanticTokens {
        background,
        surface: body_background.lighten(DARK_STEP),
        border: Color::Rgb(102, 102, 102),
        axis: AxisTheme {
            gap: units.em * GAP,
        },
        text: TextSemantic {
            muted: Color::Rgb(214, 214, 214),
            normal: Color::Rgb(255, 255, 255),
        },
        focus: Color::Rgb(91, 95, 199),
    };
    theme(units, semantic, body_background)
}

fn light_tokens() -> Theme {
    let units = Units { em: 16.0 };
    let background = Color::Rgb(225, 225, 225);
    let body_background = background.darken(LIGHT_STEP);
    let semantic = SemanticTokens {
        background,
        surface: body_background.lighten(LIGHT_STEP),
        border: Color::Rgb(209, 209, 209),
        axis: AxisTheme {
            gap: units.em * GAP,
        },
        text: TextSemantic {
            muted: Color::Rgb(66, 66, 66),
            normal: Color::Rgb(36, 36, 36),
        },
        focus: Color::Rgb(91, 95, 199),
    };
    theme(units, semantic, body_background)
}

fn theme(units: Units, semantic: SemanticTokens, body_background: Color) -> Theme {
    let text = TextStyles {
        title: TextStyle {
            size: units.em as f32 * 1.25,
            color: semantic.text.muted,
            bold: false,
        },
        subtitle: TextStyle {
            size: units.em as f32,
            color: semantic.text.normal,
            bold: false,
        },
        paragraph: TextStyle {
            size: units.em as f32 * 0.915,
            color: semantic.text.normal,
            bold: false,
        },
        button: TextStyle {
            size: units.em as f32 * 1.05,
            color: semantic.text.normal,
            bold: false,
        },
    };
    let block = BlockStyle {
        padding: units.em * 0.8,
        background: semantic.surface,
        border: BorderStyle {
            thickness: BORDER_SIZE,
            color: semantic.border,
            radius: units.em * 0.5,
        },
        focused_border: BorderStyle {
            thickness: BORDER_SIZE,
            color: semantic.focus,
            radius: units.em * 0.5,
        },
    };
    let body = PaperStyle {
        block: BlockStyle {
            background: body_background,
            border: BorderStyle::none(),
            padding: units.em * 0.5,
            focused_border: BorderStyle::none(),
        },
    };
    let paper = PaperStyle { block };
    let button_background = paper.block.background.lighten(10);
    let button_block = BlockStyle {
        padding: block.padding * 0.6,
        background: button_background,
        border: BorderStyle {
            thickness: BORDER_SIZE,
            color: semantic.border,
            radius: units.em * 0.5,
        },
        focused_border: BorderStyle {
            thickness: BORDER_SIZE,
            color: semantic.focus,
            radius: units.em * 0.5,
        },
    };
    let button = ButtonStyle {
        block: button_block,
        highlight: button_background.darken(15),
    };
    let header = BlockStyle {
        padding: units.em * 1.0,
        background: semantic.background,
        border: BorderStyle::none(),
        focused_border: BorderStyle::none(),
    };
    let specific = SpecificTokens {
        button,
        paper,
        body,
        header,
        text,
    };

    Theme {
        mode: SystemTheme::Dark,
        system: SystemTheme::Dark,
        follows_system: true,
        units,
        semantic,
        specific,
    }
}

impl Default for Theme {
    fn default() -> Self {
        system_theme(SystemTheme::Dark)
    }
}
