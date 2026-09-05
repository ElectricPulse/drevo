# Widget hitbox

Each widget has its own hitbox, which by default is shared with its parent.
Calling `hitbox.make_independent()` unlinks all four hitbox variables from the parent. Individual edges can be unlinked with `hitbox.make_start_independent(direction)` or `hitbox.make_end_independent(direction)`.

By default, elements have the exact same width and height as their parents. To resize or position an element, use layout widgets such as `Anchor`, `Space`, or `Align` (for example, `Anchor::top_left(widget)`).

## Example

An `Axis(Direction::Vertical, children)` layout component shares its top edge with the first child and its bottom edge with the last child. It shares both horizontal (x-coordinate) edges with all children.

# WidgetTrait

`WidgetTrait::layout()` returns visual `Children`. It defines constraints on the component hitbox and mounts child widgets using `display!()` or `slots.set()`.

`layout()` receives `LayoutInput`, including `relayout: Signal`. Stores read with `store.affect(relayout)` signal the application to relayout when their value changes.

`render()` receives `RenderInput`, which includes a resolved `hitbox: Rect` and draws onto a `vello::Scene` through `GraphicsScene`. It carries the global `rerender` signal for render-only updates.

Event handlers receive borrowed typed inputs: `MouseEvent`, `KeyPress`, `AllEvents`, and `OtherEvent`. Each includes `relayout: Signal` and optional window access. `forward_event()` dispatches an incoming `Event` to the appropriate handler.

## Signals and state

`Signal` sends requests (`RenderRequest::Rerender` or `RenderRequest::Layout`) through an unbounded channel to the UI loop.

The UI loop coalesces requests over a short debounce window (1 ms). A `Rerender` request redraws the current scene. A `Layout` request rebuilds the widget tree layout, solves constraints with HiGHS, and redraws.

`Store<T>` holds a value and tracks subscribers passed to `.affect()`. Calling `store.set(value)` notifies all subscribed signals. `store.read()` accesses the value without subscribing.

`State<T>` is the read/affect trait object abstraction (`Box<dyn StateTrait<Output = T>>`). `Constant<T>` implements `StateTrait` without tracking subscriptions.

`memoization()` derives a cached `State` from dependency stores. It tracks store versions and reruns its async calculation only when a source version changes. Calling `.affect(signal)` subscribes that signal to every dependency store.

# Component

The `display!(child)` macro mounts a child widget into a component. It generates a stable source-location ID using `num_id!()` and calls `slots.set(id, child).await?`.

For dynamic lists of widgets (such as in loops or iterators), call `slots.set(id, child)` directly with unique integer keys, analogous to keys in UI frameworks.

A component tracks the lifetime of a widget instance, its hitbox, its child slots, and focusability across layout passes. Focus is preserved across layout changes as long as the component identity is maintained.

`SharedComponent` wraps `Arc<Mutex<Component>>` and also implements `WidgetTrait` by forwarding calls to its internal widget.

## Text

`TextContext` manages shared font and layout contexts backed by `Store`.

`Text` and `Ansi` widgets memoize both their shaped Parley layout and measured dimensions. The memoization recomputes only when text or font dependencies change.

`ansi::Content` incrementally parses ANSI escape sequences, preserving style and link state across appends.

# Layouter

Layout equations are compiled into a mixed-integer linear programming (MILP) problem solved by HiGHS. Constraints are declared through `Formula` or `Problem`.

Constraints should flow from parent to child. Avoid constraining a parent's dimensions from a child.

## Constraint priorities

Layout objectives use weighted priorities rather than HiGHS' lexicographic priorities. Because variables represent screen coordinates or dimensions within known bounds, priorities use a geometric scale: priority `p` uses `BLENDED_GOAL_WEIGHT.powi(p)`.

Weighted priorities preserve the required constraint ordering without paying the solve-time penalty of multi-pass lexicographic solves.
