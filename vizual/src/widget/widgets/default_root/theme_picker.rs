use async_trait::async_trait;
use color_eyre::eyre::Result;
use lucide_icons::Icon as Lucide_icon;
use vizual_macros::display;

use super::super::{
    button::Button,
    full::Full,
    icon::Icon,
    menu::{Menu, Shared_menu_item, get_selector},
    text::Text,
};
use crate::{
    Render, Vizual_command, Vizual_msg,
    component::{Children, context::Component_context},
    event::Pointer_event,
    handlers::Retrieve_handler,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::State,
    theme::{Theme, Theme_choice},
    widget::{Focus_provider, Widget_trait, custom_widget::Custom_widget_trait},
};

fn label(choice: Theme_choice) -> &'static str {
    match choice {
        Theme_choice::System => "System",
        Theme_choice::Dark => "Dark",
        Theme_choice::Light => "Light",
    }
}

struct Theme_menu_item {
    choice: Theme_choice,
}

#[async_trait]
impl Retrieve_handler<Theme_choice> for Theme_menu_item {
    async fn on_retrieve(&mut self) -> Result<Theme_choice> {
        Ok(self.choice)
    }
}

#[async_trait]
impl Custom_widget_trait for Theme_menu_item {
    type Payload = bool;

    async fn layout(
        &mut self,
        _render: Render,
        theme: State<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
        selected: bool,
    ) -> Result<Children> {
        let mut text = Text::new(label(self.choice));
        text.style.set(match selected {
            true => theme.load().specific.text.selected_subtitle,
            false => theme.load().specific.text.subtitle,
        });

        Ok(vec![display!(text)])
    }
}

pub(super) struct Theme_picker {
    open: State<bool>,
    choice: State<Theme_choice>,
}

impl Theme_picker {
    pub(super) fn new(open: State<bool>, choice: State<Theme_choice>) -> Self {
        Self { open, choice }
    }
}

#[async_trait]
impl Widget_trait for Theme_picker {
    async fn layout(
        &mut self,
        render: Render,
        theme: State<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        if *self.choice.load() != theme.load().choice() {
            theme.set(theme.load().select(*self.choice.load()));
        }

        let icon = Icon::new(Lucide_icon::Settings);
        let button = Button::around(display!(icon));
        let button = display!(button);
        let button = display!(Full::new(button));

        if !*self.open.load() {
            return Ok(vec![button]);
        }

        let choices = [
            Theme_choice::System,
            Theme_choice::Dark,
            Theme_choice::Light,
        ];
        let selected_index = choices
            .iter()
            .position(|candidate| *candidate == *self.choice.load())
            .expect("selected theme choice must be present in the theme menu");
        let items = choices
            .into_iter()
            .map(|choice| -> Shared_menu_item<Theme_choice> {
                Theme_menu_item { choice }.into_shared()
            })
            .collect::<Vec<_>>();
        let default_item = get_selector(&items[selected_index]);
        let mut menu = Menu::new(items, default_item, render);
        menu.set_submit_state(self.choice.clone());

        Ok(vec![button, display!(menu)])
    }

    async fn on_mouse_click(&mut self, _mouse: &Pointer_event) -> Result<Vizual_msg> {
        self.open.set(!*self.open.load());
        Vizual_msg::new(Vizual_command::Layout)
    }
}
