# Core concepts

Vizual represents a UI as a tree of `Renderable` components. A layout pass
adds solver constraints and returns stable child nodes. A paint pass receives
the resolved logical `Rect`, a `Focus_provider`, and a `Paint_context`
containing the Vello scene and shared Parley resources.

## Logical geometry and layout

`Point`, `Size`, and `Rect` in `vizual::geometry` use `f64` logical pixels.
Hit testing uses the fractional solved rectangles directly. Layout variables
are non-negative continuous `good_lp` variables.

`Layout::new(Direction, Vec<Option<Child>>)` arranges existing child nodes.
`Minimize`, `Child::fill`, adjacency objectives, alignment, and `Space`
determine size. Space values are logical units: for example,
`Space::inline(child, 2.0, Space_goal::Maximize, priority)` caps each inline
gap at two logical units and maximizes it at the given priority. Priorities
range from `0` through `9`; higher numbers are optimized before lower
priorities, and each achieved priority objective is preserved exactly while
the remaining priorities are solved. Objectives with the same priority are
summed and optimized together.
There are no fixed widget-size or overflow constraints. If the window cannot
satisfy the constraints, solving returns the overconstrained-layout
diagnostic.

## Rendering

Parents paint before their children, and later children paint above earlier
ones. Vizual does not add Vello clip layers around components. Widgets rely on
solved geometry and the window surface boundary. Only `Paragraph` and
`Screen` perform visible text-range selection for scrolling/truncation.

Parley supplies glyph advances and line metrics. All widgets, including
command output, use the same private system sans-serif font size.

## Events and focus

`vizual::event::Event` contains normalized key presses, pointer presses with
logical coordinates, wheel deltas, committed text, and close requests.
Focused key and other events propagate from the focused node toward the root.
Pointer presses use reverse child order so the last-painted matching child is
offered the event first.

A component participates in focus traversal when it calls `Focus_provider::get`
or `set_active(true)`. `Tab` and `Shift+Tab` traverse focusable nodes in tree
order. `Vizual_command::Focus` targets a retained `Child_reference`.

## Shared components and state

`Renderable::into_shared` stores a component behind the framework's async
mutex. `State<T>` stores an atomically replaceable value paired with a
`Rerender`; calling `store` also requests a relayout. Slots preserve child
identity between layout passes, which keeps focus and parent links stable.

Async rendering and event handlers must avoid holding component locks across
unrelated work. Long-running I/O belongs in Tokio tasks and should request a
rerender when visible state changes.
