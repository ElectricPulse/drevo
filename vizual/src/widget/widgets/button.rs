use async_trait::async_trait;
use color_eyre::Result;
use vizual_macros::display;

use super::{
    super::{Focus_provider, Widget_trait},
    block::Block,
    full::Full,
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
};

enum Button_content {
    Label(String),
    Child(Child),
}

pub struct Button {
    content: Button_content,
    click_handler: Option<Box<dyn Submit_handler<String>>>,
    pub active: bool,
    pub highlighted: bool,
    pub delta: Delta,
}

impl Button {
    pub fn new(label: impl Into<String>, click_handler: Box<dyn Submit_handler<String>>) -> Self {
        Self {
            content: Button_content::Label(label.into()),
            click_handler: Some(click_handler),
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
        let content = match &self.content {
            Button_content::Label(label) => {
                let active = self.active;
                let mut text = Text::new(label.clone());
                text.style.set(match active {
                    true => theme.load().specific.text.selected_subtitle,
                    false => theme.load().specific.text.subtitle,
                });
                display!(text)
            }
            Button_content::Child(content) => content.clone(),
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
        let full = Full::new(display!(block));

        Ok(vec![display!(full)])
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
