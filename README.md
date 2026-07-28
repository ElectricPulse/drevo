# Vizual

Vizual is a reactive all Rust UI framework.

## Features
- State system heavily inspired by React
- MILP powered layouting system inspired by iOS Auto Layout

## Quick start

Vizual currently requires a nightly Rust toolchain because it uses
`async_fn_track_caller` for caller locations on asynchronous layout methods.
Stable Rust rejects the crate's feature declaration before compiling an
application. `cargo +nightly` selects the nightly Cargo and compiler together;
Vizual does not otherwise require a special version of Cargo.

Create a new binary crate and add these dependencies:

```toml
[dependencies]
color-eyre = "0.6"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
vizual = { git = "https://github.com/ElectricPulse/vizual" }
```

Then replace `src/main.rs` with:

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

Run it with:

```sh
cargo +nightly run
```

`vizual::run` is synchronous because Winit owns the calling thread. The Tokio
runtime remains active for asynchronous widget and background work. The
default global controls are `Tab`, `Shift+Tab`, `Esc`, and `Ctrl+C`.

## TODO
- It should be possible to use Vizual to provide chatbots with a nice UI.
- Film the cool grid component as a GIF for GitHub.
- Investigate why `display!()` can be called infinitely on a value that is
  already `Renderable`, which is weird.
- Solve text wrapping by representing the different possible text widths and
  their resulting heights as layout branches for the solver to choose between.
- Implement scaling fonts down for constrained layouts, which is useful on
  mobile screens.
- Remove the extra minimum-screen calculation pass. Allow the window to use any
  size and handle content overflow with scrollbars or equivalent behavior.
- When Winit exposes cross-platform window-size negotiation, let the window
  manager propose a size and have Vizual respond with a supported size instead
  of calculating and publishing a minimum window size in advance.
- Implement a reusable `Scroll` component for arbitrary child widgets with
  viewport clipping, scroll-offset input, and translated rendering and hit
  testing. Decide between cached child scenes and culled rerendering during
  implementation.
- Make the Vizual Configurator popup work.
- Reconcile the different behavior of `Align` and `Space`. `Space` tries to
  push its child and can enlarge the surrounding area, while `Align` only
  positions its child inside an area that already exists.
- Crystallize the relational-delta layout system. It should probably support
  weights for adjusting how relationships scale, while an absolute-difference
  system likely has a place alongside it.
  
## Technologies used
- winit for window managment
- vello for graphics

## Documentation

- [Getting started](vizual/docs/getting-started.md)
- [Core concepts](vizual/docs/core-concepts.md)
- [Components](vizual/docs/components.md)
- [Creating custom components](vizual/docs/custom-components.md)
- [Current limitations](vizual/docs/limitations.md)
