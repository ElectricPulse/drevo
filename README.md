# ![banner](assets/banner.png)

# Vizual

Vizual (ˈvizuaːl) is a component-based Rust UI framework with built in state managment and MILP layouter

## Features
- A robust modular component system with prebuilts for alignment, vertical/horizontal layout and grid.
This avoids the ad-hoc method for every alignment, layout written specificly for each component
- State system heavily inspired by [React](https://react.dev/)
This avoids the sometimes insanely verbose and repetitive ELM architecture of a message for everything
- MILP powered layouting system inspired by [iOS Auto Layout](https://developer.apple.com/library/archive/documentation/UserExperience/Conceptual/AutolayoutPG/index.html) which is built on [Cassowary](https://constraints.cs.washington.edu/solvers/cassowary-tochi.pdf)
- Structural navigation via ```Tab``` and ```Shift + Tab``` for accessibility

## Architecture

### Granular State Management
- **Targeted Reactivity**: State is managed through `Store<T>` and `State<T>`. Instead of monolithic ELM-style message loops or top-down virtual DOM diffs, components subscribe directly to the specific stores they read via `store.affect(render).await`.
- **Deduplicated Updates**: When a store value is modified via `write().await`, only the registered components subscribing to that state are scheduled for re-layout and re-render through the `Render_manager`.

### MILP Constraint Layouter
- **Multi-Objective Constraint Solving**: Layout geometry is solved using Mixed-Integer Linear Programming (MILP) with lexicographical priority levels, inspired by Cassowary and iOS Auto Layout.
- **Declarative Composition**: Widgets declare layout relationships (Hitbox dimensions, Axis alignments, Grids, Anchors, and Spacing) as mathematical constraints rather than hardcoded pixel calculations.
- **Content-Driven Resolution**: Higher-priority constraints define strict structural requirements, while lower-priority objectives handle flexible spacing and content wrapping (e.g. paragraphs deriving wrapped height from resolved width).

### Component Model & Event Routing
- **Tuple-Driven Composition**: Multi-child containers like `Axis` and `Grid` accept heterogeneous tuples of widgets directly (e.g. `(Anchor::left(title), body)`) via the `Into_widgets` trait.
- **Focus-Driven Interaction**: Pointer clicks and keyboard events are routed hierarchically through active focus chains (to get clicked, a component has to be able to get focused for now).

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
    render_manager::Render_manager,
    widget::{Widget_trait as _, widgets::paragraph::Paragraph},
};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let render_manager = Render_manager::new();
    let mut paragraph = Paragraph::new();
    paragraph.set_content("Hello from Vizual".into());

    vizual::run("Vizual example", paragraph.into_shared(), render_manager)
}
```

Run it with:

```sh
cargo +nightly run
```

`vizual::run` is synchronous because Winit owns the calling thread. The Tokio
runtime remains active for asynchronous widget and background work.

## [Documentation](docs/index.md)

## [Comparison with Iced](docs/comparison.md)

## Pre-release notes
- Currently cargo nightly is required because of ```#[track_caller]``` usage in the database

## Roadmap

See [ROADMAP.md](ROADMAP.md) for known bugs and planned work.

## Technologies used
- [winit](https://github.com/rust-windowing/winit) for window managment
- [vello](https://github.com/linebender/vello) for graphics
