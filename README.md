# ![banner](assets/banner.png)

# Vizual

Vizual (ˈvizuaːl) is a component-based Rust UI framework with built in state managment and MILP layouter

## Features
- A robust component system with prebuilts for alignment, vertical/horizontal layout and grid.
This avoids the ad-hoc method for every alignment, style option that other rust libraries add like in tui library [Ratatui](https://ratatui.rs/) or gui library [Iced](https://iced.rs/)
- State system heavily inspired by [React](https://react.dev/)
This avoids the sometimes insanely verbose and repetitive ELM architecture of a message for everything
- MILP powered layouting system inspired by [iOS Auto Layout](https://developer.apple.com/library/archive/documentation/UserExperience/Conceptual/AutolayoutPG/index.html) which is built on [Cassowary](https://constraints.cs.washington.edu/solvers/cassowary-tochi.pdf)
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

## To-Do list
- disable default decorations in winit
- optimize state managment relayouting/rerendering - there is no reason to relayout if the state of a parent changed
- dont use color eyre for everything
- optimize with ArcSwaps and RwLocks
- remove the need for nightly
- `display!()` can be called infinitely on a value that is
  already implements `Widget_trait`
- fix structural navigation
- Solve text wrapping by representing the different possible text widths and
  their resulting heights as layout branches for the solver to choose between.
- Implement scaling fonts down for constrained layouts, which is useful on
  mobile screens.
- Remove the extra minimum-screen calculation pass. Allow the window to use any
  size and handle content overflow with scrollbars or equivalent behavior.
- a widget shouldn't have to recoincile itself between system theme or override
- When Winit exposes cross-platform window-size negotiation, let the window
  manager propose a size and have Vizual respond with a supported size instead
  of calculating and publishing a minimum window size in advance.
- Implement a reusable `Scroll` component for arbitrary child widgets with
  viewport clipping, scroll-offset input, and translated rendering and hit
  testing. Decide between cached child scenes and culled rerendering during
  implementation.
- Make the Vizual Configurator save popup work.
- Create a dedicated demo of `Alignments` capabilities; the current Vizual
  Configurator does not fit the concept well enough to demonstrate them.
- Reconcile the different behavior of `Align` and `Space`. `Space` tries to
  push its child and can enlarge the surrounding area, while `Align` only
  positions its child inside an area that already exists.
- Crystallize the relational-delta layout system. It should probably support
  weights for adjusting how relationships scale, while an absolute-difference
  system likely has a place alongside it.
- Crystalize the relationships between `Widget`, `Shared_widget`, and `Shared_component`
- Look into removing the `Shared_widget<Concrete>` to `Shared_menu_item<Choice>`
  conversion shim. It only exists because the `Shared_widget` newtype does not
  automatically unsize to a trait object; consider making `Shared_widget` a
  type alias over `Arc<Mutex<T>>` or constructing menu items directly as their
  erased shared type.
- Reconsider using `General_shared_widget` as a generic layout-composition
  escape hatch. Erasing and remounting widgets this way makes it impossible to
  reliably track which widget owns focus.
- Reconcile the extra state-preserving behavior of `Child_slot` with the rest
  of the widget API. Reusing a child slot can preserve component state such as
  focus when replacing one widget with another, but many widgets do not expose
  child-slot support even though that transition is possible.
    You can pass display!() widget into a impl Widget_trait - how does that work? - the first child is just never used probably 
  Right now estabilishing a component lifetime also sets its hitbox (from the current parent) - these two would have to be seperated - one is lifetime creation,
  second is mounting and hitbox creation
- add some cool animations to elements to showcase the real time capabilities

- for no focus components I don't think they need to have stable known lifetime in between
 layout() calls

## Technologies used
- [winit](https://github.com/rust-windowing/winit) for window managment
- [vello](https://github.com/linebender/vello) for graphics
