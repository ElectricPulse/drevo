# Widget hitbox

Each widget has its own hitbox, which by default is shared with the parent.
The method `hitbox.variable.make_independent()` unlinks that variable from the parent.

This is designed so that elements are, by default, the exact same width and height as their parents. It prevents the CSS hell of having to put `width: 100%; height: 100%` on every container.

Either the widget is the same size as the parent container, or you explicitly position it with an `Anchor`/`Space`/`Alignment` component, most likely `Anchor::top_left(widget)`.

## Example

An `Axis(Direction::Vertical, children)` layout component for example shares the top edge with the first child element, it shares the bottom edge with the last edge. And it shares the x-coordinate side edges with all the child elements.

# Widget_trait

`Widget_trait::layout()` returns `Children`. It defines constraints for the component hitbox and chooses child widgets with `display!()` or `slots.set()`.

`layout()` receives `Layout_input`, including `relayout: Signal`. Stores read with `.affect(relayout)` signal this component when their value changes.

`render()` receives a resolved hitbox and draws onto a `vello::Scene`. Its `Render_input` carries the global `rerender` signal for render-only updates.

Event handlers receive borrowed typed inputs: `Mouse_event`, `Key_press`, `All_events`, and `Other_event`. Each includes the owning component's `relayout` signal. `forward_event()` selects the appropriate handler from an `Event`.

## Signals and state

`Render_manager::new()` creates one global `rerender` signal and its receiver. `Signal::for_component(id)` derives a component-targeted `relayout` signal from that same channel.

The UI loop batches signal requests for 10 ms. A global signal redraws. A component signal invalidates its cached formula, rebuilds layout, solves, then redraws.

`Store<T>` holds a value and records signals passed to `.affect()`. `Store::set()` signals those subscribers. `State<T>` is the read/affect abstraction; constants do not subscribe.

`memoization()` creates a cached derived `State`. It records source-store versions and reruns its async callback only after a dependency changes. Calling `.affect(signal)` subscribes that signal to every source store.

# Component

The primary function that converts a widget into a component is display!(). Underneath, what it does is assign a compile-time-generated ID and then call slots.set(id, child).

In other words, if you are generating widgets in a map, etc., please use slots.set() manually with an ID that remains unique to an element, just like the key in <Element key={}> in React.

A component tracks the lifetime of a widget instance, its hitbox, cached formula, child slots, and focusability.

So you can click a text form and begin typing while the UI changes its anchor from `display!(Anchor::left(form))` to `display!(Anchor::right(form))` and it will retain focus.

`SharedComponent` also implements `WidgetTrait` by forwarding to its stored widget. As a current
workaround when different widgets must be stored in one variable, first pass each alternative
through `display!()`, then store the resulting `SharedComponent`. This preserves a component
identity for each alternative while `display!()` still cannot distinguish a stable widget from a
replacement widget between layout calls - which is especially important if the widget wants to cache results from layout() calls

## Text

`Text_context` owns the Parley font and layout contexts as stores. They do not need to be dynamic today, but are stores so they can be replaced later.

`Text` and `Ansi` memoize both their shaped Parley layout and its measured size. The memo survives widget cloning and recomputes when either text-context store changes or when the styled text changes.

`Ansi::Content` incrementally parses ANSI input. `Content::append()` parses only the new sequence and preserves parser state, including open SGR styles and OSC-8 hyperlinks.

# Layouter

The entire layout is handled by a MILP solver. Currently, you can constrain widgets in a web of equations however you like. The primary flow of these constraints should be from the parent onto its children.

Avoid constraining a parent from a child; this will probably be prohibited in the future.

## Constraint priorities

Layout uses weighted priorities rather than HiGHS' lexicographic priorities. Every variable
ultimately represents a hitbox coordinate or dimension, so objectives stay within a known,
reasonable range. That makes choosing a sufficiently large blend weight straightforward; priority
`p` uses `BLENDED_GOAL_WEIGHT.powi(p)`. See `BLENDED_GOAL_WEIGHT` in `config.rs` for the
calculation.

Lexicographic priorities are available, but are much slower. Weighted priorities preserve the
needed ordering without paying that solve-time cost.
