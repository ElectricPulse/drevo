pub mod header;
mod theme_picker;

use color_eyre::eyre::Result;
use vizual_macros::display;

use self::header::Header;
use super::{
    layout::Layout,
    paper::Paper,
    positioning::anchor::{Anchor, Anchors},
};
use crate::{
    component::{Children, context::Component_context},
    geometry::Direction,
    layouter::{hitbox::Hitbox, objective::Objective},
    slot::manager::Slots,
    state::State,
    theme::{Theme, Theme_choice},
    widget::{Focus_provider, Widget, Widget_trait},
};

#[derive(Clone)]
pub struct Default_root {
    title: String,
    widget: Widget,
    header_open: State<bool>,
    theme_choice: State<Theme_choice>,
}

impl Default_root {
    pub fn new(title: impl Into<String>, widget: impl Widget_trait, render: crate::Render) -> Self {
        Self {
            title: title.into(),
            widget: Box::new(widget),
            header_open: render.new_state(false),
            theme_choice: render.new_state(Theme_choice::System),
        }
    }
}

#[async_trait::async_trait]
impl Widget_trait for Default_root {
    async fn layout(
        &mut self,
        _render: crate::Render,
        theme: State<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let widget = Anchor::new(self.widget.clone(), Anchors::top_left());

        let body = Paper::new(display!(widget));

        let header = display!(Header::new(
            self.title.clone(),
            self.header_open.clone(),
            self.theme_choice.clone(),
        ));

        let layout = Layout::new(
            Direction::Vertical,
            vec![header, display!(body)],
            Objective::default(),
            2,
        );

        let mut root = Paper::new(display!(layout));
        root.style.set(theme.load().specific.root);

        Ok(vec![display!(root)])
    }
}
