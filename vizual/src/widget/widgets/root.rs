use color_eyre::eyre::Result;
use vizual_macros::display;

use super::{
    super::{Control, Focus_provider, Shared_widget, Widget_trait, Widget_type},
    space::Space,
};
use crate::{
    component::context::Component_context,
    display::Display,
    event::Key_event,
    geometry::Rect,
    layouter::{constraints::Objective, hitbox::Hitbox},
    slot::manager::Slots,
    state::State,
    style::Color,
    theme::{Layout_theme, Theme},
};

#[derive(Clone)]
pub struct Root_style {
    pub background: Color,
}

pub struct Root<T: Widget_trait> {
    widget: Shared_widget<T>,
    pub theme: State<Root_style>,
    layout_theme: State<Layout_theme>,
}

impl<T: Widget_trait> Root<T> {
    pub fn new(widget: Shared_widget<T>, theme: State<Theme>) -> Self {
        Self {
            widget,
            theme: theme.project(|theme| &theme.specific.root),
            layout_theme: theme.project(|theme| &theme.semantic.layout),
        }
    }
}

#[async_trait::async_trait]
impl<T: Widget_trait> Control for Root<T> {
    async fn on_key_press(&mut self, key: &Key_event) -> Result<crate::Vizual_msg> {
        if crate::check_quit_event(key) {
            return crate::Vizual_msg::new(crate::Vizual_command::Quit);
        }

        crate::Vizual_msg::none()
    }
}

#[async_trait::async_trait]
impl<T: Widget_trait> Widget_trait for Root<T> {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Widget_type> {
        let widget = self.widget.clone();
        let gap = self.layout_theme.load().gap;
        let space = Space::uniform(display!(widget), gap, Objective::default(), 2);

        Ok(Widget_type::Virtual(Box::new(space)))
    }

    async fn render(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: Rect,
        display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        display.fill_rect(hitbox, self.theme.load().background);
        Ok(None)
    }
}
