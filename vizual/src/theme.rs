use crate::{
    style::Color,
    widget::widgets::{block::Block_style, paper::Paper_style, root::Root_style, text::Text_style},
};

// TODO: Pass the theme into `layout` and `render` through `Renderable_custom_state<Theme>`
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
    pub text: Text_theme,
    pub focus: Color,
}

#[derive(Clone, Copy)]
pub struct Layout_theme {
    pub gap: f64,
}

#[derive(Clone, Copy)]
pub struct Text_theme {
    pub sizes: Text_sizes,
    pub color: Text_colors,
}

impl Text_theme {
    pub fn title(self) -> Text_style {
        Text_style {
            size: self.sizes.title,
            color: self.color.muted,
        }
    }

    pub fn subtitle(self, selected: bool) -> Text_style {
        Text_style {
            size: self.sizes.subtitle,
            color: match selected {
                true => self.color.normal,
                false => self.color.muted,
            },
        }
    }

    pub fn paragraph(self) -> Text_style {
        Text_style {
            size: self.sizes.paragraph,
            color: self.color.normal,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Text_sizes {
    pub title: f32,
    pub subtitle: f32,
    pub paragraph: f32,
}

#[derive(Clone, Copy)]
pub struct Text_colors {
    pub muted: Color,
    pub normal: Color,
}

#[derive(Clone)]
pub struct Specific_tokens {
    pub block: Block_style,
    pub paper: Paper_style,
    pub root: Root_style,
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
        text: Text_theme {
            sizes: Text_sizes {
                title: units.em as f32 * 1.25,
                subtitle: units.em as f32,
                paragraph: units.em as f32 * 0.875,
            },
            color: Text_colors {
                muted: Color::Rgb(181, 186, 193),
                normal: Color::White,
            },
        },
        focus: Color::Rgb(88, 101, 242),
    };
    let specific = Specific_tokens {
        block: Block_style {
            background: semantic.surface,
            color: semantic.border,
            focused_color: semantic.focus,
            border_radius: units.em * 0.5,
        },
        paper: Paper_style {
            frame_padding: units.em * 0.75,
        },
        root: Root_style {
            background: semantic.background,
        },
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
