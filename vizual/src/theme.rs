use crate::{
    config::BORDER_SIZE,
    style::Color,
    widget::widgets::{
        block::{Block_style, Border_style},
        paper::Paper_style,
        root::Root_style,
        text::Text_style,
    },
};

// TODO: Pass the theme into `layout` and `render` through `Widget_custom_state<Theme>`
// instead of storing it on widgets; see the linked TODO by `Menu_item_trait`.
#[derive(Clone)]
pub struct Theme {
    pub units: Units,
    pub semantic: Semantic_tokens,
    pub specific: Specific_tokens,
}

#[derive(Clone, Copy)]
pub struct Units {
    pub em: f64,
}

#[derive(Clone, Copy)]
pub struct Semantic_tokens {
    pub background: Color,
    pub surface: Color,
    pub border: Color,
    pub layout: Layout_theme,
    pub text: Text_semantic,
    pub focus: Color,
}

#[derive(Clone, Copy)]
pub struct Layout_theme {
    pub gap: f64,
}

#[derive(Clone, Copy)]
pub struct Text_semantic {
    pub muted: Color,
    pub normal: Color,
}

#[derive(Clone, Copy)]
pub struct Text_styles {
    pub title: Text_style,
    pub subtitle: Text_style,
    pub selected_subtitle: Text_style,
    pub paragraph: Text_style,
}

#[derive(Clone)]
pub struct Specific_tokens {
    pub block: Block_style,
    pub paper: Paper_style,
    pub root: Root_style,
    pub text: Text_styles,
}

pub fn dark_theme() -> Theme {
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
    let root = Root_style {
        paper: Paper_style {
            padding: units.em * 3.0,
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
        },
    };
    let specific = Specific_tokens {
        block,
        paper,
        root,
        text,
    };

    Theme {
        units,
        semantic,
        specific,
    }
}

impl Default for Theme {
    fn default() -> Self {
        dark_theme()
    }
}
