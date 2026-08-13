# Roadmap

## Bugs: The application panicked (crashed).

Message:  wgpu error: Validation Error

Caused by:
  In Surface::configure
    `SurfaceOutput` must be dropped before a new `Surface` is made


Location: /home/hackerman/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/wgpu-29.0.4/src/backend/wgpu_core.rs:3879

  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ BACKTRACE ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
   1: <color_eyre[b4f93a6821aeb7a1]::config::PanicHook>::panic_report<unknown>
      at /home/hackerman/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/color-eyre-0.6.5/src/config.rs:975
   2: <color_eyre[b4f93a6821aeb7a1]::config::PanicHook>::into_panic_hook::{closure#0}<unknown>
      at /home/hackerman/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/color-eyre-0.6.5/src/config.rs:954
   3: <alloc[1ffbbed2b617e870]::boxed::Box<dyn for<'a, 'b> core[ebadbae5a5a6cf5c]::ops::function::Fn<(&'a std[9018f4dc78ac7bb7]::panic::PanicHookInfo<'b>,), Output = ()> + core[ebadbae5a5a6cf5c]::marker::Sync + core[ebadbae5a5a6cf5c]::marker::Send> as core[ebadbae5a5a6cf5c]::ops::function::Fn<(&std[9018f4dc78ac7bb7]::panic::PanicHookInfo,)>>::call<unknown>
      at /rustc/ca9a134e0985765ded9cfdde4030a5df4db7e2bd/library/alloc/src/boxed.rs:2285
   4: std[9018f4dc78ac7bb7]::panicking::panic_with_hook<unknown>
      at /rustc/ca9a134e0985765ded9cfdde4030a5df4db7e2bd/library/std/src/panicking.rs:833
   5: std[9018f4dc78ac7bb7]::panicking::panic_handler::{closure#0}<unknown>
      at /rustc/ca9a134e0985765ded9cfdde4030a5df4db7e2bd/library/std/src/panicking.rs:698
   6: std[9018f4dc78ac7bb7]::sys::backtrace::__rust_end_short_backtrace::<std[9018f4dc78ac7bb7]::panicking::panic_handler::{closure#0}, !><unknown>
      at /rustc/ca9a134e0985765ded9cfdde4030a5df4db7e2bd/library/std/src/sys/backtrace.rs:182
   7: __rustc[65ef6f51ce03687d]::rust_begin_unwind<unknown>
      at /rustc/ca9a134e0985765ded9cfdde4030a5df4db7e2bd/library/std/src/panicking.rs:689
   8: core[ebadbae5a5a6cf5c]::panicking::panic_fmt<unknown>
      at /rustc/ca9a134e0985765ded9cfdde4030a5df4db7e2bd/library/core/src/panicking.rs:80

## To-Do list
- make scroll bars draggable
- switch to vello hybrid
- find one stable delta for multiple states of the Patmat configurator so the menu doesnt flicker with sizes on small devices
- generalize dialog (from settings) and make a click off logic
- Switch to microlp once it adds presolve and priorities
  microlp is already a pretty slow solver since its single threaded and a presolve step is badly needed to optimize out all the equality constraints I have
  if you want wasm compatibility just switch to microlp in its current state - it would be nice to have this library in the browser in the FUTURE
  right now I would sacrifice its desktop performance because of some browser support which isn't even needed yet
  The library should grow on the desktop and if it's momentum becomes big enough should fix microlp to become on par with highs
- disable default decorations in winit
- optimize state managment relayouting/rerendering - there is no reason to relayout if the state of a parent changed
- dont use color eyre for everything
- optimize with ArcSwaps and RwLocks
- remove the need for nightly
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
