use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual::{
    Vizual_command, Vizual_msg,
    component::{Children, context::Component_context},
    event::{Key_code, Key_event},
    layouter::hitbox::Hitbox,
    render_manager::Render_manager,
    slot::manager::Slots,
    widget::{
        Focus_provider, Widget_trait,
        widgets::text::Text,
    },
};
use vizual_macros::display;

struct Counter {
    value: i64,
}

#[async_trait]
impl Widget_trait for Counter {
    async fn layout(
        &mut self,
        _render: vizual::Render,
        _theme: vizual::state::State<vizual::theme::Theme>,
        focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut vizual::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        focus.set_active(true);
        let text = Text::new(format!("Count: {} (use ↑ and ↓)", self.value));
        Ok(vec![display!(text)])
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        match key.code {
            Key_code::Arrow_up => self.value += 1,
            Key_code::Arrow_down => self.value -= 1,
            _ => return Vizual_msg::none(),
        }

        Vizual_msg::new(Vizual_command::Layout)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let render_manager = Render_manager::new();

    vizual::run(
        "Custom widget",
        Counter { value: 0 }.into_shared(),
        render_manager,
    )
}
