# Getting started

Vizual requires a Rust 2024 toolchain, an active multi-thread Tokio runtime,
and a desktop environment supported by Winit and Vello. Vello also requires a
GPU adapter with its compute-rendering requirements.

Add the workspace crate and runtime:

```toml
[dependencies]
color-eyre = "0.6"
tokio = { version = "1", features = ["full"] }
vizual = { path = "../vizual" }
```

Create a shared root widget and a rerender channel:

```rust
use color_eyre::eyre::Result;
use vizual::{
    Rerender,
    renderable::Renderable as _,
    state::State,
    theme::dark_theme,
};
use vizual::renderable::paragraph::Paragraph;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let (rerender, render_signal) = Rerender::new();
    let theme = State::new_with(rerender, dark_theme());
    let mut paragraph = Paragraph::new();
    paragraph.set_content("Hello from Vizual".to_owned());

    vizual::run(
        "Vizual example",
        paragraph.into_shared(),
        theme,
        render_signal,
    )
}
```

`vizual::run` is synchronous because Winit owns the calling thread. Existing
widget logic and background work remain async on the active Tokio runtime.
The window uses platform-default initial dimensions and is resizable.

## Background rerenders

Clone `Rerender` into background work and call `send` after changing visible
state. Pending requests are coalesced before a relayout:

```rust
let (rerender, render_signal) = vizual::Rerender::new();
tokio::spawn(async move {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    rerender.send();
});
```

## Runtime commands

Event handlers return `Vizual_msg`. `None` propagates without work, `Layout`
rebuilds constraints, `Resolve` resolves existing constraints for a new
window size, `Render` repaints, `Focus` changes focus, and `Quit` closes the
window. `Vizual_msg::new` consumes the event;
`Vizual_msg::new_propagated` lets ancestors continue handling it.

The default runner handles `Tab`, `Shift+Tab`, `Esc`, and `Ctrl+C`. Window
close requests are normalized into the framework event path and then exit the
event loop.

`Screen` runs commands through `/bin/bash -c` on Unix. Its `run*` methods
return an unsupported-platform error elsewhere.
