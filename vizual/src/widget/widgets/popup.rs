use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

use super::{
    super::{Focus_provider, Shared_widget, Widget_trait},
    button::Button,
    layout::axis::Axis,
    menu::{Menu, Shared_menu_item, get_selector},
    positioning::anchor::{Anchor, Anchors},
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
    state::State,
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
        let mut text = Text::new(self.option.label());
        text.style.set(match selected {
            true => theme.load().specific.text.selected_subtitle,
            false => theme.load().specific.text.subtitle,
        });
        let text = Anchor::new(text, Anchors::top_left());

        Ok(vec![display!(text)])
    }
}

#[derive(Clone)]
struct Popup_submit_handler {
    subhandler: Box<dyn Submit_handler<bool>>,
}

#[async_trait]
impl Submit_handler<Popup_options> for Popup_submit_handler {
    async fn on_submit(&mut self, option: Option<Popup_options>) -> Result<Vizual_msg> {
        let Some(option) = option else {
            return Vizual_msg::new(Vizual_command::Layout);
        };
        let Some(should_save) = option.should_save() else {
            return Vizual_msg::new(Vizual_command::Layout);
        };

        self.subhandler.on_submit(Some(should_save)).await
    }
}

#[derive(Clone)]
struct Popup_button_handler {
    menu: Shared_widget<Menu<Popup_options>>,
    submit_handler: Box<dyn Submit_handler<Popup_options>>,
}

#[async_trait]
impl Submit_handler<String> for Popup_button_handler {
    async fn on_submit(&mut self, _label: Option<String>) -> Result<Vizual_msg> {
        let option = self.menu.on_retrieve().await?;
        self.submit_handler.on_submit(Some(option)).await
    }
}

#[derive(Clone)]
pub struct Popup {
    menu: Shared_widget<Menu<Popup_options>>,
    submit_handler: Box<dyn Submit_handler<Popup_options>>,
}

impl Popup {
    pub fn new(submit_handler: impl Submit_handler<bool>, render: crate::Render) -> Self {
        let items = Popup_options::ALL
            .into_iter()
            .map(|option| -> Shared_menu_item<Popup_options> {
                Popup_menu_item { option }.into_shared().into()
            })
            .collect::<Vec<_>>();
        let default_item = get_selector(&items[0]);
        let menu = Widget_trait::into_shared(Menu::new(items, default_item, render));
        let submit_handler: Box<dyn Submit_handler<Popup_options>> =
            Box::new(Popup_submit_handler {
                subhandler: Box::new(submit_handler),
            });

        Self {
            menu,
            submit_handler,
        }
    }
}

#[async_trait]
impl Widget_trait for Popup {
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
        let mut text = Text::new("Submit");
        text.style.set(theme.load().specific.text.selected_subtitle);
        let button = Button::new(
            text,
            Popup_button_handler {
                menu: self.menu.clone(),
                submit_handler: self.submit_handler.clone(),
            },
        );
        let button = Anchor::new(button, Anchors::top_left());
        let menu = Anchor::new(self.menu.clone(), Anchors::top_left());
        let axis = Axis::new(Direction::Vertical, vec![Box::new(menu), Box::new(button)]);
        let block = Title_block::new(axis, "Are you sure you want to quit?");
        let anchor = Anchor::new(block, Anchors::middle());
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
