# ![banner](assets/banner.png)

# Vizual

Vizual ( ˈvizuaːl) is a reactive all Rust UI framework

## Features
- State system heavily inspired by [React](https://react.dev/)
- MILP powered layouting system inspired by [iOS Auto Layout](https://developer.apple.com/library/archive/documentation/UserExperience/Conceptual/AutolayoutPG/index.html)
- Structural navigation via ```Tab``` and ```Shift + Tab``` for accessibility
## Demo
![demo](assets/demo.gif)

## Quick start

Vizual currently requires a nightly Rust toolchain because it uses
`async_fn_track_caller`.

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
runtime remains active for asynchronous widget and background work.

## Documentation

- [Getting started](docs/getting-started.md)
- [Core concepts](docs/core-concepts.md)
- [Components](docs/components.md)
- [Creating custom components](docs/custom-components.md)
- [Current limitations](docs/limitations.md)

## Pre-release notes
- Currently cargo nightly is required because of ```#[track_caller]``` usage in the database

## To-Do list
- remove the need for nightly
- `display!()` can be called infinitely on a value that is
  already `Renderable`
- fix structural navigation
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
- Make the Vizual Configurator save popup work.
- Reconcile the different behavior of `Align` and `Space`. `Space` tries to
  push its child and can enlarge the surrounding area, while `Align` only
  positions its child inside an area that already exists.
- Crystallize the relational-delta layout system. It should probably support
  weights for adjusting how relationships scale, while an absolute-difference
  system likely has a place alongside it.
- Crystalize the mess of Child, Shared_renderable, Shared_compoent &c

## Technologies used
- [winit](https://github.com/rust-windowing/winit) for window managment
- [vello](https://github.com/linebender/vello) for graphics
