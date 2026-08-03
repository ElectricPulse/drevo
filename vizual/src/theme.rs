use crate::{
    config::BORDER_SIZE,
    style::Color,
    widget::widgets::{
        block::{Block_style, Border_style},
        paper::Paper_style,
        text::Text_style,
    },
};

#[derive(Clone, PartialEq)]
pub struct Theme {
    choice: Theme_choice,
    system: System_theme,
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
    pub layout: Layout_theme,
    pub text: Text_semantic,
    pub focus: Color,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Layout_theme {
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
    pub block: Block_style,
    pub paper: Paper_style,
    pub root: Paper_style,
    pub text: Text_styles,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Theme_choice {
    System,
    Dark,
    Light,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum System_theme {
    Light,
    Dark,
}

impl Theme {
    pub fn choice(&self) -> Theme_choice {
        self.choice
    }

    pub fn select(&self, choice: Theme_choice) -> Self {
        resolve_theme(choice, self.system)
    }

    pub(crate) fn set_system(&self, system: System_theme) -> Self {
        resolve_theme(self.choice, system)
    }
}

pub fn dark_theme() -> Theme {
    resolve_theme(Theme_choice::Dark, System_theme::Dark)
}

pub fn light_theme() -> Theme {
    resolve_theme(Theme_choice::Light, System_theme::Light)
}

pub(crate) fn system_theme(system: System_theme) -> Theme {
    resolve_theme(Theme_choice::System, system)
}

fn resolve_theme(choice: Theme_choice, system: System_theme) -> Theme {
    let dark = match choice {
        Theme_choice::System => matches!(system, System_theme::Dark),
        Theme_choice::Dark => true,
        Theme_choice::Light => false,
    };
    let mut theme = if dark { dark_tokens() } else { light_tokens() };
    theme.choice = choice;
    theme.system = system;
    theme
}

fn dark_tokens() -> Theme {
    let units = Units { em: 16.0 };
    let semantic = Semantic_tokens {
        background: Color::Rgb(30, 31, 34),
        surface: Color::Rgb(49, 51, 56),
        border: Color::Rgb(78, 80, 88),
        layout: Layout_theme {
            gap: units.em * 0.625,
        },
        text: Text_semantic {
            muted: Color::Rgb(181, 186, 193),
            normal: Color::White,
        },
        focus: Color::Rgb(88, 101, 242),
    };
    theme(units, semantic)
}

fn light_tokens() -> Theme {
    let units = Units { em: 16.0 };
    let semantic = Semantic_tokens {
        background: Color::Rgb(245, 246, 248),
        surface: Color::White,
        border: Color::Rgb(210, 212, 218),
        layout: Layout_theme {
            gap: units.em * 0.625,
        },
        text: Text_semantic {
            muted: Color::Rgb(92, 96, 105),
            normal: Color::Rgb(31, 32, 35),
        },
        focus: Color::Rgb(88, 101, 242),
    };
    theme(units, semantic)
}

fn theme(units: Units, semantic: Semantic_tokens) -> Theme {
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
            size: units.em as f32 * 0.875,
            color: semantic.text.normal,
        },
    };
    let block = Block_style {
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
    let paper = Paper_style {
        padding: units.em * 0.75,
        block,
    };
    let root = Paper_style {
        padding: units.em * 1.2,
        block: Block_style {
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
        block,
        paper,
        root,
        text,
    };

    Theme {
        choice: Theme_choice::System,
        system: System_theme::Dark,
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
