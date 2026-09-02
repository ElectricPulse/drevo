use crate::macros::display;
use async_trait::async_trait;
use color_eyre::eyre::Result;

use super::{
    super::{Layout_input, Widget_trait},
    button::Button,
    layout::axis::Axis,
    menu::{Menu, Menu_item},
    positioning::anchor::Anchor,
    text::Text,
    title_block::Title_block,
};
use crate::{
    Vizual_command, Vizual_msg,
    component::Children,
    event::{Event, Key_event, Pointer_event},
    geometry::Direction,
    handlers::{Retrieve_handler, Submit_handler},
    state::State,
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
    async fn on_retrieve(&mut self) -> Result<State<Popup_options>> {
        Ok(self.option.into())
    }
}

#[async_trait]
impl Custom_widget_trait for Popup_menu_item {
    type Payload = bool;

    async fn layout(
        &mut self,
        Layout_input {
            render,
            theme,
            slots,
            ..
        }: Layout_input<'_>,
        selected: bool,
    ) -> Result<Children> {
        let theme = theme.affect(render).await?;
        let mut text = Text::new(self.option.label());
        let mut style = theme.specific.text.button;
        if !selected {
            style.color = theme.semantic.text.muted;
        }
        text.style.set(style);

        Ok(vec![display!(text)])
    }
}

#[derive(Clone)]
pub struct Popup {
    menu: Menu<Popup_options>,
    submit_handler: Box<dyn Submit_handler<Popup_options>>,
}

impl Popup {
    pub async fn new(submit_handler: impl Submit_handler<bool>) -> Result<Self> {
        let items: Vec<Menu_item<Popup_options>> = Popup_options::ALL
            .into_iter()
            .map(|option| -> Menu_item<Popup_options> { Box::new(Popup_menu_item { option }) })
            .collect();
        let menu = Menu::new(items, 0).await?;
        let subhandler: Box<dyn Submit_handler<bool>> = Box::new(submit_handler);
        let submit_handler: Box<dyn Submit_handler<Popup_options>> =
            Box::new(move |option: Popup_options| {
                let mut subhandler = subhandler.clone();
                async move {
                    let Some(should_save) = option.should_save() else {
                        return Vizual_msg::new(Vizual_command::Resolve);
                    };
                    subhandler.on_submit(should_save).await
                }
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
        Layout_input {
            render,
            theme,
            slots,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        let theme = theme.affect(render).await?;
        let mut text = Text::new("Submit");
        text.style.set(theme.specific.text.button);
        let menu_clone = self.menu.clone();
        let submit_handler = self.submit_handler.clone();
        let button = Button::new(text, move |_focused: bool| {
            let mut menu = menu_clone.clone();
            let mut submit_handler = submit_handler.clone();
            async move {
                let option_state = menu.on_retrieve().await?;
                let option = *option_state.read().await?;
                submit_handler.on_submit(option).await
            }
        });
        let button = Anchor::left(button);
        let menu = Anchor::left(self.menu.clone());
        let axis = Axis::new(Direction::Vertical, (menu, button));
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
