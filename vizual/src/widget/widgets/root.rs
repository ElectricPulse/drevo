use color_eyre::eyre::Result;
use vizual_macros::display;

use super::{
    super::{Focus_provider, Shared_widget, Widget_trait},
    full::Full,
};
use crate::{
    component::{Children, context::Component_context},
    event::Key_event,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::State,
    theme::Theme,
    widget::widgets::paper::{Paper, Paper_style},
};

#[derive(Clone, Copy, PartialEq)]
pub struct Root_style {
    pub paper: Paper_style,
}

pub struct Root<T: Widget_trait> {
    widget: Shared_widget<T>,
    pub style: State<Root_style>,
}

impl<T: Widget_trait> Root<T> {
    pub fn new(widget: Shared_widget<T>, style: State<Root_style>) -> Self {
        Self { widget, style }
    }
}

impl From<&State<Theme>> for State<Root_style> {
    fn from(theme: &State<Theme>) -> Self {
        theme.project(|theme| &theme.specific.root)
    }
}

#[async_trait::async_trait]
impl<T: Widget_trait> Widget_trait for Root<T> {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let widget = self.widget.clone();
        let paper_style = self.style.project(|style| &style.paper);
        let paper = Paper::new(display!(widget), paper_style);
        let full = Full::new(display!(paper));

        Ok(vec![display!(full)])
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<crate::Vizual_msg> {
        if crate::check_quit_event(key) {
            return crate::Vizual_msg::new(crate::Vizual_command::Quit);
        }

        crate::Vizual_msg::none()
    }
}
