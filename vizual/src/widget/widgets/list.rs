use async_trait::async_trait;
use color_eyre::eyre::Result;

use super::{
    super::{Focus_provider, Render_input, Widget_trait},
    text::Text_style,
};
use crate::{
    event::{Key_code, Key_event},
    geometry::{Point, Rect},
    graphics::scene::Scene,
    layouter::hitbox::Hitbox,
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
impl Widget_trait for List {
    async fn render(
        &mut self,
        Render_input {
            focus,
            hitbox,
            scene,
            text_context,
            ..
        }: Render_input<'_, '_>,
    ) -> Result<()> {
        focus.set_interactive(true);
        let mut y = hitbox.origin.y;

        for (index, item) in self.items.iter().enumerate() {
            let marker = match index == self.selected {
                true => ">> ",
                false => "   ",
            };
            let line = format!("{marker}{item}");
            let size = text_context.draw_text(
                scene,
                &line,
                Point::new(hitbox.origin.x, y),
                Text_style::default(),
            );
            y += size.height;
        }

        Ok(())
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<crate::Vizual_msg> {
        if self.items.is_empty() {
            return crate::Vizual_msg::none();
        }

        match key.code {
            Key_code::Arrow_down => {
                self.set_index(get_next_index(self.items.len(), self.get_index()));
                crate::Vizual_msg::new(crate::Vizual_command::Layout)
            }
            Key_code::Arrow_up => {
                self.set_index(get_previous_index(self.items.len(), self.get_index()));
                crate::Vizual_msg::new(crate::Vizual_command::Layout)
            }
            _ => crate::Vizual_msg::none(),
        }
    }
}
