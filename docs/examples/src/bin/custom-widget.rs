use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual::{
    Vizual_command, Vizual_msg,
    component::Children,
    event::{Key_code, Key_event},
    render_manager::Render_manager,
    widget::{Layout_input, Widget_trait, widgets::text::Text},
};
use vizual::macros::display;

#[derive(Clone)]
struct Counter {
    value: i64,
}

#[async_trait]
impl Widget_trait for Counter {
    async fn layout(
        &mut self,
        Layout_input {
            focus,
            slots,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        focus.set_interactive(true);
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
