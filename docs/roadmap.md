# Roadmap

## Optimization
If you encounter performance issues, it's probably gonna get solved with one of these features,
which I haven't implemented because they are complex and make the code even less flexible for architectural changes

- render signal only relayouts current component
- scroll should render only visible components
- constraints should take in `State` and only relayout if state changed (things like a button changing color or a button changing size inside a scroll should not cause a full global relayout)
- make the render system parallel (render subtrees / components concurrently)
- cache Parley text layout constructs across renders instead of recreating them on every frame

## To-Do list
- add a global popup - save & exit, exit, cancel that will work for configurator as of now
- most of the Submit_handlers could be converted into closures
- think about if scroll couldnt alter hitboxes after layout - to improve performance
- choose if putting everything in a widget in a store really is the way
- using parley implement text selection copying cursor &c
- add an example patmat build
- finish text input with parley cursor positioning
- tab refocuses without clicking
- in apps like react the same lifetime used to track widget is the same as the component lifetime ie. a stable id used in slots.set() means you dont have to store menu under `Shared_widget<Menu>`. Since Menu needs to keep state of the selected item
- make scroll bars draggable
- fix the occasional ~500 ms stutter during scrollin
- figure out a better Paragraph sizing and constraint-negotiation model
- switch to vello hybrid
- Switch to microlp once it adds presolve and priorities
  microlp is already a pretty slow solver since its single threaded and a presolve step is badly needed to optimize out all the equality constraints I have
  if you want wasm compatibility just switch to microlp in its current state - it would be nice to have this library in the browser in the FUTURE
  right now I would sacrifice its desktop performance because of some browser support which isn't even needed yet
  The library should grow on the desktop and if it's momentum becomes big enough should fix microlp to become on par with highs
- disable default decorations in winit
- dont use color eyre for everything
- remove the need for nightly
- Solve text wrapping by representing the different possible text widths and
  their resulting heights as layout branches for the solver to choose between.
- Implement scaling fonts down for constrained layouts, which is useful on
  mobile screens.
- a widget shouldn't have to recoincile itself between system theme or override
- When Winit exposes cross-platform window-size negotiation, let the window
  manager propose a size and have Vizual respond with a supported size instead
  of calculating and publishing a minimum window size in advance.
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
- Distinguish between graphical parent (how to mask/clip, how to resolve render hierarchy and hit-testing) and logical parent (where to forward events and preserve logical widget relationships, e.g. floating menus / portals).
- add a follow feature to terminal.rs (auto-scroll to bottom as new output arrives)
