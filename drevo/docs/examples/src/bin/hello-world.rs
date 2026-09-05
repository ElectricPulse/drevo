use async_trait::async_trait;
use color_eyre::eyre::Result;
use drevo::{
    VizualMsg,
    component::Children,
    event::KeyCode,
    macros::display,
    state::Store,
    widget::{KeyPress, LayoutInput, WidgetTrait, widgets::text::Text},
};

#[derive(Clone)]
struct Counter {
    value: Store<i64>,
}

#[async_trait]
impl WidgetTrait for Counter {
    async fn layout(
        &mut self,
        LayoutInput {
            focus,
            relayout,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        focus.set_interactive(true);
        let value = self.value.affect(relayout).await?;
        let text = Text::new(format!("Count: {} (use ↑ and ↓)", *value));
        Ok(vec![display!(text)])
    }

    async fn on_key_press(&mut self, input: KeyPress<'_>) -> Result<VizualMsg> {
        let current = *self.value.read().await?;
        let value = match input.key.code {
            KeyCode::ArrowUp => current + 1,
            KeyCode::ArrowDown => current - 1,
            _ => return VizualMsg::none(),
        };

        self.value.set(value).await?;
        VizualMsg::none()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    drevo::run(
        "Hello from Vizual",
        Counter {
            value: Store::new(0),
        },
    )
}
