use async_trait::async_trait;
use color_eyre::eyre::Result;

use super::{
    super::{Focus_provider, Widget_trait},
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
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: crate::state::Store<crate::theme::Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: crate::component::context::Component_context,
        _text_context: &mut crate::graphics::text::Text_context,
        _slots: &mut crate::slot::manager::Slots,
        _root: &crate::component::Shared_component,
    ) -> Result<crate::component::Children> {
        Ok(vec![])
    }

    async fn render(
        &mut self,
        _render: crate::Render,
        _theme: crate::state::Store<crate::theme::Theme>,
        focus: &mut Focus_provider,
        hitbox: Rect,
        scene: &mut Scene<'_>,
        text_context: &mut crate::graphics::text::Text_context,
        _context: &crate::component::Render_context<'_>,
    ) -> Result<()> {
        focus.set_active(true);
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
