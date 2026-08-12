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
    state::State,
    theme::Theme,
    widget::custom_widget::Custom_widget_trait,
};

#[derive(Clone)]
struct String_menu_item {
    value: String,
}

#[async_trait]
impl Retrieve_handler<String> for String_menu_item {
    async fn on_retrieve(&mut self) -> Result<String> {
        Ok(self.value.clone())
    }
}

#[async_trait]
impl Custom_widget_trait for String_menu_item {
    type Payload = bool;

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
        selected: bool,
    ) -> Result<Children> {
        let mut text = Text::new(self.value.clone());
        text.style.set(match selected {
            true => theme.load().specific.text.selected_subtitle,
            false => theme.load().specific.text.subtitle,
        });

        Ok(vec![display!(text)])
    }
}

impl Menu<String> {
    pub fn text(items: Vec<String>, default_item: usize, render: crate::Render) -> Self {
        let items = items
            .into_iter()
            .map(|value| -> Shared_menu_item<String> {
                String_menu_item { value }.into_shared().into()
            })
            .collect::<Vec<_>>();
        let default_item = get_selector(
            items
                .get(default_item)
                .expect("Default menu item index must be in range"),
        );

        Self::new(items, default_item, render)
    }
}
