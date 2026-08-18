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

## [Architecture](docs/architecture.md)

See the full [Architecture guide](docs/architecture.md) for details on the layouter, granular state management, and component model.

- **Granular State Management**: Localized, fine-grained reactivity via `Store<T>` and `State<T>`. Components subscribe directly through `store.affect(render).await`, ensuring that state mutations only re-layout and re-render affected subtrees.
- **MILP Constraint Layouter**: Multi-objective layout optimization powered by Mixed-Integer Linear Programming (MILP) with lexicographical priority levels, inspired by Cassowary and iOS Auto Layout.
- **Heterogeneous Tuple Layout**: Multi-child layout containers (`Axis`, `Grid`) accept tuples of arbitrary concrete widget types directly via `Into_widgets`.
- **Event Routing & Focus**: Pointer clicks and keyboard events are routed hierarchically through active focus chains (to get clicked, a component has to be able to get focused for now).

## Performance

Currently the performance — mainly visible when scrolling — is inadequate, somewhere around 5 FPS.

The bottleneck at a first glance is obviously the obscene MILP layouter, and it truly does add a pretty big delay to a rerender as there is currently no way to decouple or destructure the MILP problem: it always requires a resolve of the entire system.
Even so, re-solving the entire system puts the app in the 25-50 FPS range (~20-30ms solve). The difference in FPS is made up of a very immature render system in reality. It is not parallel and it is slow — mainly because of Parley constructs being recreated on every render.

Here is proof:

```text
app problem layout took 79.471867ms
17:52:25 [INFO]   lexicographic model: 7805 variables, 3593 constraints, 3 priorities
17:52:25 [INFO]     lexicographic model recreation took 6.354568ms
17:52:25 [INFO]   lexicographic solve took 24.950131ms
17:52:25 [INFO] layout full solve took 40.687032ms
17:52:25 [INFO] app problem render took 45.587774ms
```

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

## [Roadmap](docs/roadmap.md)

See [docs/roadmap.md](docs/roadmap.md) for known bugs and planned work.

## Technologies used
- [winit](https://github.com/rust-windowing/winit) for window managment
- [vello](https://github.com/linebender/vello) for graphics
