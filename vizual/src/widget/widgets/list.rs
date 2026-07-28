use async_trait::async_trait;
use color_eyre::eyre::Result;

use super::{
    super::{Control, Focus_provider, Renderable},
    text::Text_style,
};
use crate::{
    backend::graphics::Paint_context,
    event::{Key_code, Key_event},
    geometry::{Point, Rect},
    hitbox::Hitbox,
    utils::{bind_index, get_next_index, get_previous_index},
};

pub struct List {
    selected: usize,
    items: Vec<String>,
}

impl Default for List {
    fn default() -> Self {
        Self::new()
    }
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
impl Renderable for List {
    async fn render(
        &mut self,
        focus: &mut Focus_provider,
        hitbox: Rect,
        paint: &mut Paint_context<'_>,
    ) -> Result<Option<Hitbox>> {
        focus.set_active(true);
        let mut y = hitbox.origin.y;

        for (index, item) in self.items.iter().enumerate() {
            let marker = match index == self.selected {
                true => ">> ",
                false => "   ",
            };
            let line = format!("{marker}{item}");
            let size =
                paint.draw_text(&line, Point::new(hitbox.origin.x, y), Text_style::default());
            y += size.height;
        }

        Ok(None)
    }
}

#[async_trait]
impl Control for List {
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
