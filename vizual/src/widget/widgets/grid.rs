use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

use crate::{
    component::{Children, context::Component_context},
    layouter::{constraints::prohibit_overlap, hitbox::Hitbox},
    slot::manager::Slots,
    widget::{Control, Focus_provider, Widget_trait, Widget_type},
};

pub struct Grid {
    children: Vec<Widget_type>,
    gap: f64,
}

impl Grid {
    pub fn new(children: Vec<Widget_type>, gap: f64) -> Self {
        Self { children, gap }
    }
}

impl Control for Grid {}

#[async_trait]
impl Widget_trait for Grid {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Widget_type> {
        let mut visual_children = Children::new();
        for child in std::mem::take(&mut self.children) {
            match child {
                Widget_type::None => {}
                Widget_type::Virtual(widget) => visual_children.push(display!(widget)),
                Widget_type::Visual {
                    children: mut item_children,
                } => visual_children.append(&mut item_children),
            }
        }

        for (index, first) in visual_children.iter().enumerate() {
            for second in visual_children.iter().skip(index + 1) {
                let first_hitbox = first.get_hitbox().await?;
                let second_hitbox = second.get_hitbox().await?;

                prohibit_overlap(&problem, first_hitbox, second_hitbox, self.gap).await?;
            }
        }

        Ok(Widget_type::Visual {
            children: visual_children,
        })
    }
}
