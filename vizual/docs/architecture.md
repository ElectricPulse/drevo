# Widget hitbox

Each widget has its own hitbox, which by default is shared with the parent.
The method `hitbox.variable.make_independent()` unlinks that variable from the parent.

This is designed so that elements are, by default, the exact same width and height as their parents. It prevents the CSS hell of having to put `width: 100%; height: 100%` on every container.

Either the widget is the same size as the parent container, or you explicitly position it with an `Anchor`/`Space`/`Alignment` component, most likely `Anchor::top_left(widget)`.

## Example

An `Axis(Direction::Vertical, children)` layout component for example shares the top edge with the first child element, it shares the bottom edge with the last edge. And it shares the x-coordinate side edges with all the child elements.

# Widget_trait

As for the widget trait, it has two methods. One of them, layout() -> Children, runs whenever the state or content inside the widget changes. This is where you define the constraints of your own hitbox and choose which other widgets you want to render by returning them encased in a display!() macro.

render() is the method that gets the resolved hitbox and can draw onto a `vello::Scene`

# Component

The primary function that converts a widget into a component is display!(). Underneath, what it does is assign a compile-time-generated ID and then call slots.set(id, child).

In other words, if you are generating widgets in a map, etc., please use slots.set() manually with an ID that remains unique to an element, just like the key in <Element key={}> in React.

A component tracks the lifetime of a widget instance. The primary thing it needs to track is focus.

So you can click a textform and begin typing while the ui changes it's Anchor from `display!(Anchor::left(form))` to `display!(Anchor::right(form))` and it will still retain focus.

# Layouter

The entire layout is handled by a MILP solver. Currently, you can constrain widgets in a web of equations however you like. The primary flow of these constraints should be from the parent onto its children.

Avoid constraining a parent from a child; this will probably be prohibited in the future.