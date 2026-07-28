use async_trait::async_trait;
use color_eyre::Result;
use vizual_macros::display;

use super::{
    super::{Control, Focus_provider, Renderable, Widget_type},
    block::Block,
    space::Space,
    text::Text,
};
use crate::{
    Vizual_msg,
    component::Child,
    event::Pointer_event,
    handlers::Submit_handler,
    hitbox::Hitbox,
    layouter::{Problem_context, constraints::Objective},
    slot_manager::Slots,
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
    pub theme: State<Theme>,
}

impl Button {
    pub fn new(
        label: impl Into<String>,
        click_handler: Box<dyn Submit_handler<String>>,
        theme: State<Theme>,
    ) -> Self {
        Self {
            content: Button_content::Label(label.into()),
            click_handler: Some(click_handler),
            active: true,
            highlighted: false,
            theme,
        }
    }

    pub fn around(content: Child, theme: State<Theme>) -> Self {
        Self {
            content: Button_content::Child(content),
            click_handler: None,
            active: true,
            highlighted: false,
            theme,
        }
    }
}

#[async_trait]
impl Renderable for Button {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: Hitbox,
        _problem: Problem_context,
        slots: &mut Slots,
    ) -> Result<Widget_type> {
        let content = match &self.content {
            Button_content::Label(label) => {
                let text = Text::new(label.clone())
                    .set_style(self.theme.load().semantic.text.subtitle(self.active));
                display!(text)
            }
            Button_content::Child(content) => content.clone(),
        };

        let space = Space::uniform(
            content,
            self.theme.load().units.em * 0.75,
            Objective::default(),
            2,
        );

        let mut block = Block::new(display!(space), self.theme.clone());
        block.highlighted = self.highlighted;

        Ok(Widget_type::Virtual(Box::new(block)))
    }
}

#[async_trait]
impl Control for Button {
    async fn on_mouse_click(&mut self, _mouse: &Pointer_event) -> Result<Vizual_msg> {
        match (&mut self.click_handler, &self.content) {
            (Some(click_handler), Button_content::Label(label)) => {
                click_handler.on_submit(Some(label.clone())).await
            }
            _ => Vizual_msg::none(),
        }
    }
}
