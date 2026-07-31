use std::sync::Arc;

use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

use super::{
    super::{Focus_provider, Shared_widget, Widget_trait},
    anchor::{Anchor, Anchors},
    button::Button,
    full::Full,
    layout::{Layout, Style as Layout_style},
    menu::{Menu, Menu_item_trait, Shared_menu_item, get_selector},
    text::Text,
    title_block::Title_block,
};
use crate::{
    Vizual_command, Vizual_msg,
    component::{Child, Children, context::Component_context},
    event::{Event, Key_event, Pointer_event},
    geometry::Direction,
    handlers::{Retrieve_handler, Submit_handler},
    layouter::{hitbox::Hitbox, objective::Objective},
    slot::manager::Slots,
    state::State,
    sync::Mutex,
    theme::Theme,
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

struct Popup_menu_item {
    option: Popup_options,
    theme: State<Theme>,
}

#[async_trait]
impl Retrieve_handler<Popup_options> for Popup_menu_item {
    async fn on_retrieve(&mut self) -> Result<Popup_options> {
        Ok(self.option)
    }
}

#[async_trait]
impl Menu_item_trait<Popup_options> for Popup_menu_item {
    async fn layout(
        &mut self,
        selected: bool,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Child> {
        let text = Text::new(self.option.label())
            .set_style(self.theme.load().semantic.text.subtitle(selected));

        Ok(display!(text))
    }
}

struct Popup_submit_handler<Subhandler> {
    subhandler: Subhandler,
}

#[async_trait]
impl<Subhandler: Submit_handler<bool>> Submit_handler<Popup_options>
    for Popup_submit_handler<Subhandler>
{
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

type Shared_popup_submit_handler = Arc<Mutex<dyn Submit_handler<Popup_options>>>;

struct Popup_button_handler {
    menu: Shared_widget<Menu<Popup_options>>,
    submit_handler: Shared_popup_submit_handler,
}

#[async_trait]
impl Submit_handler<String> for Popup_button_handler {
    async fn on_submit(&mut self, _label: Option<String>) -> Result<Vizual_msg> {
        let option = self.menu.on_retrieve().await?;
        self.submit_handler
            .lock()
            .await?
            .on_submit(Some(option))
            .await
    }
}

pub struct Popup {
    menu: Shared_widget<Menu<Popup_options>>,
    submit_handler: Shared_popup_submit_handler,
    theme: State<Theme>,
}

impl Popup {
    pub fn new(submit_handler: impl Submit_handler<bool>, theme: State<Theme>) -> Self {
        let items = Popup_options::ALL
            .into_iter()
            .map(|option| {
                Arc::new(Mutex::new(Popup_menu_item {
                    option,
                    theme: theme.clone(),
                })) as Shared_menu_item<Popup_options>
            })
            .collect::<Vec<_>>();
        let default_item = get_selector(&items[0]);
        let menu = Menu::new(items, default_item, theme.clone()).into_shared();
        let submit_handler: Shared_popup_submit_handler =
            Arc::new(Mutex::new(Popup_submit_handler {
                subhandler: submit_handler,
            }));

        Self {
            menu,
            submit_handler,
            theme,
        }
    }
}

#[async_trait]
impl Widget_trait for Popup {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let button = Button::new(
            "Submit",
            Box::new(Popup_button_handler {
                menu: self.menu.clone(),
                submit_handler: self.submit_handler.clone(),
            }),
            self.theme.clone(),
        );
        let menu = self.menu.clone();
        let layout = Layout::new(
            Direction::Vertical,
            vec![display!(menu), display!(button)],
            Layout_style::default(self.theme.clone()),
            Objective::default(),
            2,
        );
        let block = Title_block::new(
            display!(layout),
            "Are you sure you want to quit?",
            self.theme.clone(),
        );
        let anchor = Anchor::new(display!(block), Anchors::middle());
        let full = Full::new(display!(anchor));

        Ok(vec![display!(full)])
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
