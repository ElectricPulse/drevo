use crate::macros::display;
use async_trait::async_trait;
use color_eyre::eyre::Result;

use super::{
    super::{LayoutInput, WidgetTrait},
    button::Button,
    layout::axis::Axis,
    menu::{Menu, MenuItem},
    positioning::anchor::Anchor,
    text::Text,
    title_block::TitleBlock,
};
use crate::{
    VizualCommand, VizualMsg,
    component::Children,
    event::Event,
    geometry::Direction,
    handlers::{RetrieveHandler, SubmitHandler},
    state::State,
    widget::custom_widget::CustomWidgetTrait,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PopupOptions {
    Quit,
    Save,
    Cancel,
}

impl PopupOptions {
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
struct PopupMenuItem {
    option: PopupOptions,
}

#[async_trait]
impl RetrieveHandler<PopupOptions> for PopupMenuItem {
    async fn on_retrieve(&mut self) -> Result<State<PopupOptions>> {
        Ok(self.option.into())
    }
}

#[async_trait]
impl CustomWidgetTrait for PopupMenuItem {
    type Payload = bool;

    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            theme,
            slots,
            ..
        }: LayoutInput<'_>,
        selected: bool,
    ) -> Result<Children> {
        let theme = theme.affect(relayout).await?;
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
    menu: Menu<PopupOptions>,
    submit_handler: Box<dyn SubmitHandler<PopupOptions>>,
}

impl Popup {
    pub async fn new(submit_handler: impl SubmitHandler<bool>) -> Result<Self> {
        let items: Vec<MenuItem<PopupOptions>> = PopupOptions::ALL
            .into_iter()
            .map(|option| -> MenuItem<PopupOptions> { Box::new(PopupMenuItem { option }) })
            .collect();
        let menu = Menu::new(items, 0).await?;
        let subhandler: Box<dyn SubmitHandler<bool>> = Box::new(submit_handler);
        let submit_handler: Box<dyn SubmitHandler<PopupOptions>> =
            Box::new(move |option: PopupOptions| {
                let mut subhandler = subhandler.clone();
                async move {
                    let Some(should_save) = option.should_save() else {
                        return VizualMsg::new(VizualCommand::Resolve);
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
impl WidgetTrait for Popup {
    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            theme,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let theme = theme.affect(relayout).await?;
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
        let block = TitleBlock::new(axis, "Are you sure you want to quit?");
        let anchor = Anchor::middle(block);
        Ok(vec![display!(anchor)])
    }

    async fn on_all_events(&mut self, input: crate::widget::AllEvents<'_>) -> Result<VizualMsg> {
        self.menu.on_all_events(input).await
    }

    async fn on_mouse_click(
        &mut self,
        input: crate::widget::MouseEvent<'_>,
    ) -> Result<VizualMsg> {
        self.menu.on_mouse_click(input).await
    }

    async fn on_key_press(&mut self, input: crate::widget::KeyPress<'_>) -> Result<VizualMsg> {
        self.menu.on_key_press(input).await
    }

    async fn on_other_event(
        &mut self,
        input: crate::widget::OtherEvent<'_>,
    ) -> Result<VizualMsg> {
        self.menu.on_other_event(input).await
    }

    async fn forward_event(
        &mut self,
        event: &Event,
        relayout: crate::Signal,
    ) -> Result<VizualMsg> {
        self.menu.forward_event(event, relayout).await
    }
}
