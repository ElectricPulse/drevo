use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual::{
    Rerender, Vizual_command, Vizual_msg,
    event::{Key_code, Key_event},
    layouter::hitbox::Hitbox,
    layouter::Problem_context,
    slot::manager::Slots,
    state::State,
    theme::dark_theme,
    widget::{Control, Focus_provider, Widget_trait, Widget_type, widgets::text::Text},
};

struct Counter(i64);

#[async_trait]
impl Control for Counter {
    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        match key.code {
            Key_code::Arrow_up => self.0 += 1,
            Key_code::Arrow_down => self.0 -= 1,
            _ => return Vizual_msg::none(),
        }

        Vizual_msg::new(Vizual_command::Layout)
    }
}

#[async_trait]
impl Widget_trait for Counter {
    async fn layout(
        &mut self,
        focus: &mut Focus_provider,
        _hitbox: Hitbox,
        _problem: Problem_context,
        _slots: &mut Slots,
    ) -> Result<Widget_type> {
        focus.set_active(true);
        Ok(Widget_type::Virtual(Box::new(Text::new(format!(
            "Count: {} (use ↑ and ↓)",
            self.0
        )))))
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
