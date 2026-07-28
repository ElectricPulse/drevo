# Components

Vizual's widgets paint directly into Vello scenes and use Parley for intrinsic
text measurement. Their sizes come from solver constraints, existing spacing,
and explicit `fill()` calls; the backend does not assign fixed menu, tab,
tree, selector, or pane dimensions.

Constraint and painting primitives are exposed under `vizual::renderable`.
Higher-order compositions using the flattened `Widget` adapter are exposed
under `vizual::widgets`.

## Structural widgets

- `Text` is a single intrinsic Parley text layout with a framework `Color`.
- `Block` adds a one-logical-unit border around its child.
- `Paper` adds themed uniform frame padding inside a `Block`.
- `Space` applies optional logical-unit gaps. `Objective` selects whether
  those gaps are maximized, minimized, or kept close to their target values,
  and each constructor accepts the solver objective's priority.
- `Align` positions a child at the start, center, or end of each axis.
- `Minimize` adds minimize objectives for both dimensions.
- `Layout` arranges `Vec<Option<Child>>` horizontally or vertically.
- `Root` adds the default help line and global quit handling.
- `Popup` paints after its ordinary siblings and therefore receives pointer
  hit testing first.

## Interactive widgets

- `Button` preserves active/disabled border and text colors and invokes its
  `Submit_handler<String>` on a pointer press.
- `Menu<T>` uses up/down selection and Enter submission. Pointer presses
  select its child buttons.
- `List` uses up/down selection and a `>>` selection indicator.
- `Tabs` uses left/right keys and zero-based number shortcuts.
- `Form` retains its field traversal, popup, and centered title behavior.
- `Text_input` has private Parley-backed editing state for insertion,
  backspace/delete, arrows, home/end, validation colors, submission, cursor
  painting, and horizontal cursor following. It intentionally has no
  clipboard or pointer selection.

## Paragraph

`Paragraph` is one of the two scrollable widgets. `set_content` accepts static
ANSI text. A private shared viewport parses supported SGR styles, caches its
Parley layout, measures glyph advances and line metrics, selects visible
content, and paints vertical/horizontal scrollbars.

Arrow keys scroll by a text line metric, PageUp/PageDown use the viewport,
Home/End jump vertically, and wheel input scrolls in logical units. Supply a
`Block_style` with `Paragraph::block` when a titled or colored border is
needed.

## Screen

`Screen` is the other scrollable widget. On Unix it runs `/bin/bash -c`,
merges streamed stdout/stderr, decodes UTF-8, and feeds the same private text
viewport as `Paragraph`. It retains status text, vertical navigation, follow
mode, and `Command_handle` cleanup. On non-Unix platforms the `run*` methods
return an unsupported-platform error.

Up/down and PageUp/PageDown scroll, Home or `g` jumps to the beginning, End or
`G` jumps to the end, and `f` resumes follow mode. Wheel input scrolls
vertically. Command output uses the same system sans-serif font as every
other widget.
