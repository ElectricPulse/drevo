# Getting started

Vizual requires the nightly Rust toolchain with Rust 2024 edition support, an
active multi-thread Tokio runtime, and a desktop environment supported by Winit
and Vello. Vello also requires a GPU adapter with its compute-rendering
requirements.

Install the toolchain if necessary:

```sh
rustup toolchain install nightly
```

Add Vizual and its runtime dependencies:

```toml
[dependencies]
color-eyre = "0.6"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
vizual = { git = "https://github.com/ElectricPulse/vizual" }
```

Create a shared root widget and a rerender channel:

```rust
use color_eyre::eyre::Result;
use vizual::{
    Rerender,
    state::State,
    theme::dark_theme,
    widget::{Renderable as _, widgets::paragraph::Paragraph},
};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let (rerender, render_signal) = Rerender::new();
    let theme = State::new_with(rerender, dark_theme());
    let mut paragraph = Paragraph::new();
    paragraph.set_content("Hello from Vizual".into());

    vizual::run(
        "Vizual example",
        paragraph.into_shared(),
        theme,
        render_signal,
    )
}
```

Run the application with the nightly toolchain:

```sh
cargo +nightly run
```

`cargo +nightly` tells Rustup to use nightly Cargo and `rustc` for the command.
Cargo itself does not need unstable functionality. Running with stable Rust
currently stops before building the application:

```text
error[E0554]: #![feature] may not be used on the stable release channel
```

Vizual uses `#![feature(async_fn_track_caller)]` because several asynchronous
layout methods preserve their caller locations for diagnostics.

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
