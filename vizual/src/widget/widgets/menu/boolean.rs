use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

use super::{
    super::{super::Focus_provider, text::Text},
    Menu, Shared_menu_item, get_selector,
};
use crate::{
    component::{Children, context::Component_context},
    handlers::Retrieve_handler,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::{State, Store},
    theme::Theme,
    widget::custom_widget::Custom_widget_trait,
};

#[derive(Clone, Copy)]
struct Boolean_menu_item {
    value: bool,
}

impl Boolean_menu_item {
    fn label(&self) -> &'static str {
        if self.value { "Enabled" } else { "Disabled" }
    }
}

#[async_trait]
impl Retrieve_handler<bool> for Boolean_menu_item {
    async fn on_retrieve(&mut self) -> Result<bool> {
        Ok(self.value)
    }
}

#[async_trait]
impl Custom_widget_trait for Boolean_menu_item {
    type Payload = bool;

    async fn layout(
        &mut self,
        render: crate::Render,
        theme: Store<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::graphics::text::Text_context,
        slots: &mut Slots,
        selected: bool,
    ) -> Result<Children> {
        let theme = theme.affect(render).await?;
        let mut text = Text::new(self.label());
        text.style.set(match selected {
            true => theme.specific.text.selected_subtitle,
            false => theme.specific.text.subtitle,
        });

        Ok(vec![display!(text)])
    }
}

impl Menu<bool> {
    pub async fn boolean(default: bool) -> Result<Self> {
        let items = [false, true]
            .into_iter()
            .map(|value| -> Shared_menu_item<bool> {
                Boolean_menu_item { value }.into_shared().into()
            })
            .collect::<Vec<_>>();
        let default_item = get_selector(&items[usize::from(default)]);

        Self::new(items, default_item).await
    }

    pub(crate) async fn set_selected(&self, value: bool) -> Result<()> {
        self.set_index(usize::from(value)).await
    }
}
