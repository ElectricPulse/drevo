pub mod header;
mod theme_picker;

use color_eyre::eyre::Result;
use vizual_macros::display;

use self::header::Header;
use super::{
    anchor::{Anchor, Anchors, Position},
    full::Full,
    layout::Layout,
    paper::Paper,
};
use crate::{
    component::{Children, context::Component_context},
    geometry::Direction,
    layouter::{hitbox::Hitbox, objective::Objective},
    slot::manager::Slots,
    state::State,
    theme::{Theme, Theme_choice},
    widget::{Focus_provider, Shared_widget, Widget_trait},
};

pub struct Default_root<T: Widget_trait> {
    title: String,
    widget: Shared_widget<T>,
    header_open: State<bool>,
    theme_choice: State<Theme_choice>,
}

impl<T: Widget_trait> Default_root<T> {
    pub fn new(title: impl Into<String>, widget: Shared_widget<T>, render: crate::Render) -> Self {
        Self {
            title: title.into(),
            widget,
            header_open: render.new_state(false),
            theme_choice: render.new_state(Theme_choice::System),
        }
    }
}

#[async_trait::async_trait]
impl<T: Widget_trait> Widget_trait for Default_root<T> {
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
        let body = Paper::new(display!(self.widget.clone()));
        let body = Full::new(display!(body));

        let header = Full::width(display!(Header::new(
            self.title.clone(),
            self.header_open.clone(),
            self.theme_choice.clone(),
        )));

        let layout = Layout::new(
            Direction::Vertical,
            vec![display!(header), display!(body)],
            Objective::default(),
            2,
        );

        let layout = Full::new(display!(layout));
        let mut root = Paper::new(display!(layout));
        root.style.set(theme.load().specific.root);

        let root = Anchor::new(
            display!(root),
            Anchors {
                horizontal: Some(Position::Start),
                vertical: Some(Position::Start),
            },
        );

        Ok(vec![display!(root)])
    }
}
