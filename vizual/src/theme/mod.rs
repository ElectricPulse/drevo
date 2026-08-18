use crate::{
    config::BORDER_SIZE,
    style::Color,
    widget::widgets::{
        block::{Block_style, Border_style},
        button::Button_style,
        paper::Paper_style,
        text::Text_style,
    },
};

#[cfg(test)]
mod tests;

#[derive(Clone, PartialEq)]
pub struct Theme {
    mode: System_theme,
    system: System_theme,
    follows_system: bool,
    pub units: Units,
    pub semantic: Semantic_tokens,
    pub specific: Specific_tokens,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Units {
    pub em: f64,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Semantic_tokens {
    pub background: Color,
    pub surface: Color,
    pub border: Color,
    pub axis: Axis_theme,
    pub text: Text_semantic,
    /// Accent reserved for borders whose component actually owns focus or contains it.
    pub focus: Color,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Axis_theme {
    pub gap: f64,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Text_semantic {
    pub muted: Color,
    pub normal: Color,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Text_styles {
    pub title: Text_style,
    pub subtitle: Text_style,
    pub selected_subtitle: Text_style,
    pub paragraph: Text_style,
}

#[derive(Clone, PartialEq)]
pub struct Specific_tokens {
    pub button: Button_style,
    pub paper: Paper_style,
    pub body: Paper_style,
    pub root: Paper_style,
    pub text: Text_styles,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum System_theme {
    Light,
    Dark,
}

impl Theme {
    pub fn mode(&self) -> System_theme {
        self.mode
    }

    pub fn system(&self) -> System_theme {
        self.system
    }

    pub fn follows_system(&self) -> bool {
        self.follows_system
    }

    pub fn select(&self, mode: System_theme) -> Self {
        resolve_theme(mode, self.system, false)
    }

    pub fn follow_system(&self) -> Self {
        resolve_theme(self.system, self.system, true)
    }

    pub(crate) fn set_system(&self, system: System_theme) -> Self {
        let mode = match self.follows_system {
            true => system,
            false => self.mode,
        };
        resolve_theme(mode, system, self.follows_system)
    }
}

pub fn dark_theme() -> Theme {
    resolve_theme(System_theme::Dark, System_theme::Dark, false)
}

pub fn light_theme() -> Theme {
    resolve_theme(System_theme::Light, System_theme::Light, false)
}

pub(crate) fn system_theme(system: System_theme) -> Theme {
    resolve_theme(system, system, true)
}

fn resolve_theme(mode: System_theme, system: System_theme, follows_system: bool) -> Theme {
    let mut theme = match mode {
        System_theme::Dark => dark_tokens(),
        System_theme::Light => light_tokens(),
    };
    theme.mode = mode;
    theme.system = system;
    theme.follows_system = follows_system;
    theme
}

const DARK_STEP: u8 = 11;
const LIGHT_STEP: u8 = 5;

fn dark_tokens() -> Theme {
    let units = Units { em: 16.0 };
    let background = Color::Rgb(20, 20, 20);
    let body_background = background.lighten(DARK_STEP);
    let semantic = Semantic_tokens {
        background,
        surface: body_background.lighten(DARK_STEP),
        border: Color::Rgb(102, 102, 102),
        axis: Axis_theme {
            gap: units.em * 0.625,
        },
        text: Text_semantic {
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
    let body_background = background.lighten(LIGHT_STEP);
    let semantic = Semantic_tokens {
        background,
        surface: body_background.lighten(LIGHT_STEP),
        border: Color::Rgb(209, 209, 209),
        axis: Axis_theme {
            gap: units.em * 0.625,
        },
        text: Text_semantic {
            muted: Color::Rgb(66, 66, 66),
            normal: Color::Rgb(36, 36, 36),
        },
        focus: Color::Rgb(91, 95, 199),
    };
    theme(units, semantic, body_background)
}

fn theme(units: Units, semantic: Semantic_tokens, body_background: Color) -> Theme {
    let text = Text_styles {
        title: Text_style {
            size: units.em as f32 * 1.25,
            color: semantic.text.muted,
        },
        subtitle: Text_style {
            size: units.em as f32,
            color: semantic.text.muted,
        },
        selected_subtitle: Text_style {
            size: units.em as f32,
            color: semantic.text.normal,
        },
        paragraph: Text_style {
            size: units.em as f32 * 0.575,
            color: semantic.text.normal,
        },
    };
    let block = Block_style {
        padding: units.em * 0.70,
        background: semantic.surface,
        border: Border_style {
            thickness: BORDER_SIZE,
            color: semantic.border,
            radius: units.em * 0.5,
        },
        focused_border: Border_style {
            thickness: BORDER_SIZE,
            color: semantic.focus,
            radius: units.em * 0.5,
        },
    };
    let body = Paper_style {
        block: Block_style {
            background: body_background,
            ..block
        },
    };
    let paper = Paper_style { block };
    let button_background = paper.block.background.lighten(10);
    let button = Button_style {
        block: Block_style {
            background: button_background,
            ..block
        },
        highlight: button_background.darken(15),
    };
    let root = Paper_style {
        block: Block_style {
            padding: units.em * 1.0,
            background: semantic.background,
            border: Border_style {
                thickness: 0.0,
                color: semantic.border,
                radius: 0.0,
            },
            focused_border: Border_style {
                thickness: 0.0,
                color: semantic.focus,
                radius: 0.0,
            },
        },
    };
    let specific = Specific_tokens {
        button,
        paper,
        body,
        root,
        text,
    };

    Theme {
        mode: System_theme::Dark,
        system: System_theme::Dark,
        follows_system: true,
        units,
        semantic,
        specific,
    }
}

impl Default for Theme {
    fn default() -> Self {
        system_theme(System_theme::Dark)
    }
}
