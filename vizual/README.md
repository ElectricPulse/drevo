# Vizual

It's time to make simple UI even more computationally expensive so that even basic programs require hardware a million times more capable than of the landing computer of Apollo 11.

Vizual is an asynchronous GUI framework rendered by Vello, with Winit window
and input handling and Parley text layout. It provides a solver-backed
component tree, focus traversal, event propagation, shared state, and reusable
desktop widgets.

It should be possible to use Vizual to provide chatbots with a nice UI.

All layout and pointer geometry uses floating-point logical pixels. Winit's
display scale factor is applied only when the completed logical Vello scene is
presented to the physical surface. Text uses one private system sans-serif
font configuration throughout the framework.

## Features

- A single resizable desktop window
- Continuous `good_lp` layout with explicit spacing and fill objectives
- Async widgets, handlers, command output, and shared state on Tokio
- Keyboard focus traversal and reverse-order pointer hit testing
- Vello borders, colors, focus, validation, disabled, and selection states
- ANSI-aware Parley text for static and streamed command output
- Explicit scrolling in `Paragraph` and `Screen`; other widgets do not scroll

## Quick start

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

    // Winit owns the calling thread, so run is synchronous.
    vizual::run(
        "Vizual example",
        paragraph.into_shared(),
        theme,
        render_signal,
    )
}
```

The Tokio runtime must remain active while `run` owns the main thread. The
default global controls are `Tab`, `Shift+Tab`, `Esc`, and `Ctrl+C`.

## TODO

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

## Documentation

- [Getting started](docs/getting-started.md)
- [Core concepts](docs/core-concepts.md)
- [Components](docs/components.md)
- [Creating custom components](docs/custom-components.md)
- [Current limitations](docs/limitations.md)
