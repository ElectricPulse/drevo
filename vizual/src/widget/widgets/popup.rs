use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

use super::{
    super::{Focus_provider, Shared_widget, Widget_trait},
    button::Button,
    layout::axis::Axis,
    menu::{Menu, Menu_item},
    positioning::anchor::Anchor,
    text::Text,
    title_block::Title_block,
};
use crate::{
    Vizual_command, Vizual_msg,
    component::{Children, context::Component_context},
    event::{Event, Key_event, Pointer_event},
    geometry::Direction,
    handlers::{Retrieve_handler, Submit_handler},
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::{State, Store},
    theme::Theme,
    widget::custom_widget::Custom_widget_trait,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Popup_options {
    Quit,
    Save,
    Cancel,
}

impl Popup_options {
    const ALL: [Self; 3] = [Self::Quit, Self::Save, Self::Cancel];

    fn label(self) -> &'static str {
        match self {
            Self::Quit => "Quit",
            Self::Save => "Save",
            Self::Cancel => "Cancel",
        }
    }

    fn should_save(self) -> Option<bool> {
        match self {
            Self::Quit => Some(false),
            Self::Save => Some(true),
            Self::Cancel => None,
        }
    }
}

#[derive(Clone)]
struct Popup_menu_item {
    option: Popup_options,
}

#[async_trait]
impl Retrieve_handler<Popup_options> for Popup_menu_item {
    async fn on_retrieve(&mut self) -> Result<Popup_options> {
        Ok(self.option)
    }
}

#[async_trait]
impl Custom_widget_trait for Popup_menu_item {
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
        _logical: &mut bool,
        selected: bool,
    ) -> Result<Children> {
        let theme = theme.affect(render).await?;
        let mut text = Text::new(self.option.label());
        text.style.set(match selected {
            true => theme.specific.text.selected_subtitle,
            false => theme.specific.text.subtitle,
        });

        Ok(vec![display!(text)])
    }
}

#[derive(Clone)]
struct Popup_submit_handler {
    subhandler: Box<dyn Submit_handler<bool>>,
}

#[async_trait]
impl Submit_handler<Popup_options> for Popup_submit_handler {
    async fn on_submit(&mut self, option: Popup_options) -> Result<Vizual_msg> {
        let Some(should_save) = option.should_save() else {
            return Vizual_msg::new(Vizual_command::Layout);
        };

        self.subhandler.on_submit(should_save).await
    }
}

#[derive(Clone)]
struct Popup_button_handler {
    menu: Shared_widget<Menu<Popup_options>>,
    submit_handler: Box<dyn Submit_handler<Popup_options>>,
}

#[async_trait]
impl Submit_handler<bool> for Popup_button_handler {
    async fn on_submit(&mut self, _focused: bool) -> Result<Vizual_msg> {
        let option = self.menu.on_retrieve().await?;
        self.submit_handler.on_submit(option).await
    }
}

#[derive(Clone)]
pub struct Popup {
    menu: Shared_widget<Menu<Popup_options>>,
    submit_handler: Box<dyn Submit_handler<Popup_options>>,
}

impl Popup {
    pub async fn new(submit_handler: impl Submit_handler<bool>) -> Result<Self> {
        let items: Vec<Menu_item<Popup_options>> = Popup_options::ALL
            .into_iter()
            .map(|option| -> Menu_item<Popup_options> {
                Box::new(Popup_menu_item { option })
            })
            .collect();
        let menu = Menu::new(items, 0).await?.into_shared();
        let submit_handler: Box<dyn Submit_handler<Popup_options>> =
            Box::new(Popup_submit_handler {
                subhandler: Box::new(submit_handler),
            });

        Ok(Self {
            menu,
            submit_handler,
        })
    }
}

#[async_trait]
impl Widget_trait for Popup {
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
        _logical: &mut bool,
    ) -> Result<Children> {
        let theme = theme.affect(render).await?;
        let mut text = Text::new("Submit");
        text.style.set(theme.specific.text.selected_subtitle);
        let button = Button::new(
            text,
            Popup_button_handler {
                menu: self.menu.clone(),
                submit_handler: self.submit_handler.clone(),
            },
        );
        let button = Anchor::left(button);
        let menu = Anchor::left(self.menu.clone());
        let axis = Axis::new(Direction::Vertical, vec![Box::new(menu), Box::new(button)]);
        let block = Title_block::new(axis, "Are you sure you want to quit?");
        let anchor = Anchor::middle(block);
        Ok(vec![display!(anchor)])
    }

    async fn on_all_events(&mut self, event: &Event) -> Result<Vizual_msg> {
        self.menu.on_all_events(event).await
    }

    async fn on_mouse_click(&mut self, mouse: &Pointer_event) -> Result<Vizual_msg> {
        self.menu.on_mouse_click(mouse).await
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        self.menu.on_key_press(key).await
    }

    async fn on_other_event(&mut self, event: &Event) -> Result<Vizual_msg> {
        self.menu.on_other_event(event).await
    }

    async fn forward_event(&mut self, event: &Event) -> Result<Vizual_msg> {
        self.menu.forward_event(event).await
    }
}
