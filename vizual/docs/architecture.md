# Widget

Each widget has it's own hitbox which by default is shared with the parent.
The method `hitbox.variable.make_independent()` unlinks that variable from the parent.
This is all made so that elements by default are the exact same width/height as their parents.
It prevents the CSS hell of putting `width: 100%, height: 100%` on every container.
Either the widget is the same size as the container or you explicitly position it with an Anchor/Alignment component, most likely `Anchor::top_left(widget)`.

As for the widget trait. It has two methods one that gets run on state/content change inside the widget called layout() -> Children. This is where you define the constraints of your own hitbox and choose what other widgets you want to render by returning them encased in a display!() macro.

# Component
The primary function that converts a widget into a component is display!(). Underneath what it does is assign a compile time generated id and then call `slots.set(id, child)`.
In other words if you are generating widgets in a map &c please use `slots.set()` manually with an id that stays unique to an element. Just like key in `<Element key={}>` in React.

A component tracks the lifetime of a widget instanciation. The primary thing it needs to track is focus.
So you can change a widgets Anchor from `display!(Anchor::left(widget))` to `display!(Anchor::right(widget))` and it will still stay focus.

# Layouter
The whole layouting is done via a MILP solver - basically you can constrain widgets in a web of 


