use async_trait::async_trait;
use color_eyre::eyre::Result;

use super::{
    super::{RenderInput, WidgetTrait},
    text::TextStyle,
};
use crate::{
    event::KeyCode,
    geometry::Point,
    graphics::text::StyledText,
    utils::{bind_index, get_next_index, get_previous_index},
};

#[derive(Clone, Default)]
pub struct List {
    selected: usize,
    items: Vec<String>,
}

impl List {
    pub fn new() -> Self {
        Self {
            selected: 0,
            items: Vec::new(),
        }
    }

    pub fn set(&mut self, items: Vec<String>) {
        self.items = items;
        if !self.items.is_empty() {
            self.set_index(bind_index(self.items.len(), self.get_index()));
        }
    }

    pub fn set_index(&mut self, index: usize) {
        self.selected = index;
    }

    pub fn get_index(&self) -> usize {
        self.selected
    }
}

#[async_trait]
impl WidgetTrait for List {
    async fn render(
        &mut self,
        RenderInput {
            focus,
            hitbox,
            scene,
            text_context,
            ..
        }: RenderInput<'_, '_>,
    ) -> Result<()> {
        focus.set_interactive(true);
        let mut y = hitbox.origin.y;

        for (index, item) in self.items.iter().enumerate() {
            let marker = match index == self.selected {
                true => ">> ",
                false => "   ",
            };
            let line = format!("{marker}{item}");
            let styled = StyledText::styled(&line, TextStyle::default());
            let size = text_context
                .draw_text(scene, &styled, Point::new(hitbox.origin.x, y))
                .await?;
            y += size.height;
        }

        Ok(())
    }

    async fn on_key_press(
        &mut self,
        input: crate::widget::KeyPress<'_>,
    ) -> Result<crate::VizualMsg> {
        let key = input.key;
        if self.items.is_empty() {
            return crate::VizualMsg::none();
        }

        match key.code {
            KeyCode::ArrowDown => {
                self.set_index(get_next_index(self.items.len(), self.get_index()));
                crate::VizualMsg::new(crate::VizualCommand::Render)
            }
            KeyCode::ArrowUp => {
                self.set_index(get_previous_index(self.items.len(), self.get_index()));
                crate::VizualMsg::new(crate::VizualCommand::Render)
            }
            _ => crate::VizualMsg::none(),
        }
    }
}
