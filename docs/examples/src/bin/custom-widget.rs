use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual::{
    Rerender, Vizual_command, Vizual_msg,
    component::{Children, context::Component_context},
    event::{Key_code, Key_event},
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::State,
    theme::dark_theme,
    widget::{
        Focus_provider, Widget_trait,
        widgets::{full::Full, text::Text},
    },
};
use vizual_macros::display;

struct Counter(i64);

#[async_trait]
impl Widget_trait for Counter {
    async fn layout(
        &mut self,
        focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut vizual::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        focus.set_active(true);
        let text = Text::new(format!(
            "Count: {} (use ↑ and ↓)",
            self.0
        ));
        let full = Full::new(display!(text));
        Ok(vec![display!(full)])
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        match key.code {
            Key_code::Arrow_up => self.0 += 1,
            Key_code::Arrow_down => self.0 -= 1,
            _ => return Vizual_msg::none(),
        }

        Vizual_msg::new(Vizual_command::Layout)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let (rerender, render_signal) = Rerender::new();
    let theme = State::new_with(rerender, dark_theme());

    vizual::run(
        "Custom widget",
        Counter(0).into_shared(),
        theme,
        render_signal,
    )
}
