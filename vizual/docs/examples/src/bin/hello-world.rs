use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual::{
    Vizual_msg,
    component::Children,
    event::{Key_code, Key_event},
    state::Store,
    widget::{Layout_input, Widget_trait, widgets::text::Text},
};
use vizual::macros::display;

#[derive(Clone)]
struct Counter {
    value: Store<i64>,
}

#[async_trait]
impl Widget_trait for Counter {
    async fn layout(
        &mut self,
        Layout_input {
            focus,
            render,
            slots,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        focus.set_interactive(true);
        let value = self.value.affect(render).await?;
        let text = Text::new(format!("Count: {} (use ↑ and ↓)", *value));
        Ok(vec![display!(text)])
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        let value = match key.code {
            Key_code::Arrow_up => *self.value.get().await? + 1,
            Key_code::Arrow_down => *self.value.get().await? - 1,
            _ => return Vizual_msg::none(),
        };

        self.value.set(value).await?;
        Vizual_msg::none()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    vizual::run(
        "Hello from Vizual",
        Counter {
            value: Store::new(0),
        }
    )
}
