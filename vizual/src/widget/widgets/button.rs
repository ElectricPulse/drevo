use async_trait::async_trait;
use color_eyre::Result;
use vizual_macros::display;

use super::{
    super::{Focus_provider, Widget_trait},
    block::Block,
    positioning::anchor::{Anchor, Anchors},
    space::Space,
    text::Text,
};
use crate::{
    Vizual_msg,
    component::{Child, Children, context::Component_context},
    event::Pointer_event,
    handlers::Submit_handler,
    layouter::{
        hitbox::Hitbox,
        objective::{Delta, Objective},
    },
    slot::manager::Slots,
    state::State,
    theme::Theme,
    widget::General_widget,
};

#[derive(Clone)]
enum Button_content {
    Label(String),
    Child(Child),
}

trait Button_handler: Submit_handler<String> + dyn_clone::DynClone {}

impl<Handler> Button_handler for Handler where Handler: Submit_handler<String> + Clone {}

dyn_clone::clone_trait_object!(Button_handler);

#[derive(Clone)]
pub struct Button {
    content: Button_content,
    click_handler: Option<Box<dyn Button_handler>>,
    pub active: bool,
    pub highlighted: bool,
    pub delta: Delta,
}

impl Button {
    pub fn new(
        label: impl Into<String>,
        click_handler: impl Submit_handler<String> + Clone,
    ) -> Self {
        Self {
            content: Button_content::Label(label.into()),
            click_handler: Some(Box::new(click_handler)),
            active: true,
            highlighted: false,
            delta: Delta::default(),
        }
    }

    pub fn around(content: Child) -> Self {
        Self {
            content: Button_content::Child(content),
            click_handler: None,
            active: true,
            highlighted: false,
            delta: Delta::default(),
        }
    }
}

#[async_trait]
impl Widget_trait for Button {
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
        let content: General_widget = match &self.content {
            Button_content::Label(label) => {
                let active = self.active;
                let mut text = Text::new(label.clone());
                text.style.set(match active {
                    true => theme.load().specific.text.selected_subtitle,
                    false => theme.load().specific.text.subtitle,
                });
                let text = Anchor::new(text, Anchors::top_left());
                Box::new(text)
            }
            Button_content::Child(content) => Box::new(content.clone()),
        };

        let mut space = Space::uniform(
            content,
            theme.load().units.em * 0.75,
            Objective::default(),
            2,
        );
        space.delta = self.delta;

        let mut block = Block::new(display!(space));
        block.highlighted = self.highlighted;

        Ok(vec![display!(block)])
    }

    async fn on_mouse_click(&mut self, _mouse: &Pointer_event) -> Result<Vizual_msg> {
        match (&mut self.click_handler, &self.content) {
            (Some(click_handler), Button_content::Label(label)) => {
                click_handler.on_submit(Some(label.clone())).await
            }
            _ => Vizual_msg::none(),
        }
    }
}
