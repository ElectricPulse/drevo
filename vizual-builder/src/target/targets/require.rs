use crate::{
    target::{Dependencies, Target},
    task::{self, Status},
};

use async_trait::async_trait;
use color_eyre::Result;
use vizual::{
    Render,
    component::{Children, context::Component_context},
    graphics::text::Text_context,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::State,
    theme::Theme,
    widget::Focus_provider,
};

#[derive(Clone, Copy)]
struct Task {}

#[async_trait]
impl vizual::widget::Widget_trait for Task {
    async fn layout(
        &mut self,
        _render: Render,
        _theme: State<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut Text_context,
        _slots: &mut Slots,
    ) -> Result<Children> {
        Ok(vec![])
    }
}

// TODO: Remove this workaround
#[async_trait]
impl task::Task_trait for Task {
    type Output = ();
    async fn run(&self, _manager: &mut task::Manager<'_>) -> task::Task_result {
        return Ok(((), Status::Built));
    }
}

pub fn new(name: impl Into<String>, dependencies: Dependencies) -> Target<()> {
    Target::new(name, Task {}, dependencies)
}
