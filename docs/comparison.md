# Comparison with Iced

This is a small source-level comparison between [Iced](https://iced.rs/) and
Vizual. It focuses on one layout that is deceptively awkward with a normal
horizontal row:

- `Hello, world!` must remain at the exact center of the window;
- a differently sized `Goodbye!` button must remain on the right;
- changing the button width must not move the title; and
- the button must have 5 pixels of left padding.

The button closes the application. Both examples implement the same behavior.
The Iced example targets [Iced 0.14.0](https://docs.rs/iced/0.14.0/iced/), while
the Vizual example targets the current repository checkout.

## Iced

Add Iced to `Cargo.toml`:

```toml
[dependencies]
iced = "0.14"
```

Then use this `src/main.rs`:

```rust
use iced::widget::{button, container, stack, text};
use iced::{Element, Fill, Task, padding};

#[derive(Debug, Clone)]
enum Message {
    Goodbye,
}

fn main() -> iced::Result {
    iced::application(|| (), update, view)
        .title("Hello, world!")
        .run()
}

fn update(_state: &mut (), message: Message) -> Task<Message> {
    match message {
        Message::Goodbye => iced::exit(),
    }
}

fn view(_state: &()) -> Element<'_, Message> {
    let title = container(text("Hello, world!").size(32))
        .center_x(Fill)
        .center_y(Fill);

    let goodbye = container(button("Goodbye!").on_press(Message::Goodbye))
        .padding(padding::left(5))
        .align_right(Fill)
        .center_y(Fill);

    stack![title, goodbye].width(Fill).height(Fill).into()
}
```

A regular row would center the title in the space left over by the button, not
in the window. This Iced version therefore overlays two fill-sized containers
with `stack!`: one centers the title and the other right-aligns the button.

## Vizual

Vizual currently requires nightly Rust. Add these dependencies to
`Cargo.toml`:

```toml
[dependencies]
async-trait = "0.1"
color-eyre = "0.6"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
uniqify = "0.1"
vizual = { git = "https://github.com/ElectricPulse/vizual" }
vizual-macros = { git = "https://github.com/ElectricPulse/vizual" }
```

Then use this `src/main.rs`:

```rust
use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual::{
    Vizual_command,
    component::{Children, context::Component_context},
    handlers::Command_submit_handler,
    layouter::{hitbox::Hitbox, objective::Objective},
    render_manager::Render_manager,
    slot::manager::Slots,
    state::State,
    widget::{
        Focus_provider, Widget_trait,
        widgets::{
            anchor::{Anchor, Anchors, Position},
            button::Button,
            space::Space,
            text::Text,
        },
    },
};
use vizual_macros::display;

struct Hello;

#[async_trait]
impl Widget_trait for Hello {
    async fn layout(
        &mut self,
        render: vizual::Render,
        theme: vizual::state::Store<vizual::theme::Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut vizual::graphics::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let theme = theme.affect(render).await?;
        let mut title = Text::new("Hello, world!");
        title.style.set(theme.specific.text.title);
        let title = Anchor::center(display!(title));

        let goodbye = Button::new(
            Text::new("Goodbye!"),
            Box::new(Command_submit_handler::new(Vizual_command::Quit)),
        );
        let goodbye = Space::left(display!(goodbye), 5.0, Objective::default(), 1);
        let goodbye = Anchor::new(
            display!(goodbye),
            Anchors {
                horizontal: Some(Position::End),
                vertical: Some(Position::Middle),
            },
        );

        Ok(vec![display!(title), display!(goodbye)])
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let render_manager = Render_manager::new();
    let hello = Hello;

    vizual::run("Hello, world!", hello.into_shared(), render_manager)
}
```

Vizual does not need a row, an overlay widget, a dummy left-side spacer, or a
manual width calculation. The two children independently describe their
relationship to the same parent: the title uses `Anchor::center`, while the
button uses an end/middle `Anchor`. The solver keeps both relationships true
regardless of either child's size. `Space::left` adds the requested 5-pixel
padding without changing those relationships.

## What the example highlights

| Concern | Iced | Vizual |
| --- | --- | --- |
| Exact title position | A centered fill container | `Anchor::center` |
| Independent right action | A second fill container | An end/middle `Anchor` |
| Combining both | An overlaying `stack!` | Return both constrained children |
| Left padding | `padding::left(5)` | `Space::left(..., 5.0, ...)` |
| Button action | Emit a message and return an exit task | Return `Vizual_command::Quit` from a submit handler |

Iced is shorter overall because its application API hides more runtime
machinery. Vizual currently exposes its asynchronous widget and render-signal
interfaces, but its relational layout model makes the unusual placement rule
direct: describe the relationships and let the solver satisfy them.
