# Architecture

Vizual is a component-based UI framework with state tracking, an asynchronous widget lifecycle, and a constraint layouter.

## State tracking

`Store<T>` tracks which widgets read a value. When a widget reads a store during `layout` or `render` via `store.affect(render).await`, the framework records a dependency between that widget and the store. When the store value changes, the render manager schedules another layout and render pass for those widgets.

Mutations within a frame are deduplicated into a single pass.

## Layouter

The layouter represents geometry as linear equations and inequalities over edge variables (`start`, `end`, `origin`, `size`).

Constraints are solved using HiGHS with lexicographical priorities:
- Priority 0 handles hard requirements, such as parent boundary containment and minimum sizes.
- Priorities 1 and 2 resolve dynamic spacing, alignments, and maximize/minimize objectives.

Paragraph widgets use the solver's width to measure and wrap text height during layout.

## Widgets and composition

Widgets implement `Widget_trait` with asynchronous `layout` and `render` methods.

Multi-child layout containers (`Axis`, `Grid`) accept tuples of different widget types through `Into_widgets` or dynamic `Vec<Widget>` collections.

## Focus and events

Keyboard navigation (`Tab`, `Shift + Tab`, arrow keys) traverses the component tree.

Pointer events follow the focus and hitbox hierarchy. A widget must be interactive (`focus.set_interactive(true)` or a focusable container) to receive pointer clicks.

## Performance

Current scrolling performance is around 5 FPS.

The layouter solves the full constraint system on layout updates. In addition, text rendering currently recreates Parley structures on each render pass.

Example timing from a layout and render pass:

```text
app problem layout took 79.471867ms
17:52:25 [INFO]   lexicographic model: 7805 variables, 3593 constraints, 3 priorities
17:52:25 [INFO]     lexicographic model recreation took 6.354568ms
17:52:25 [INFO]   lexicographic solve took 24.950131ms
17:52:25 [INFO] layout full solve took 40.687032ms
17:52:25 [INFO] app problem render took 45.587774ms
```
