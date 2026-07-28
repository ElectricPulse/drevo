# Creating custom components

Custom primitives implement `Control` and `Renderable`. This focusable counter
uses framework events, logical geometry, and the Vello/Parley paint context:

```rust
use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual::{Vizual_command, Vizual_msg, backend::graphics::Paint_context};
use vizual::event::{Key_code, Key_event};
use vizual::geometry::Rect;
use vizual::hitbox::Hitbox;
use vizual::style::Color;
use vizual::widget::{Control, Focus_provider, Renderable};
use vizual::widget::widgets::text::Text_style;

struct Counter(i64);

#[async_trait]
impl Control for Counter {
    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        match key.code {
            Key_code::Arrow_up => self.0 += 1,
            Key_code::Arrow_down => self.0 -= 1,
            _ => return Vizual_msg::none(),
        }
        Vizual_msg::new(Vizual_command::Render)
    }
}

#[async_trait]
impl Renderable for Counter {
    async fn render(
        &mut self,
        focus: &mut Focus_provider,
        area: Rect,
        paint: &mut Paint_context<'_>,
    ) -> Result<Option<Hitbox>> {
        let color = if focus.get() { Color::Blue } else { Color::White };
        paint.stroke_rect(area.inset(0.5), color, 1.0);
        let _ = paint.draw_text(
            &self.0.to_string(),
            area.origin,
            Text_style { size: 16.0, color },
        );
        Ok(None)
    }
}
```

`Paint_context` exposes the Vello `Scene`, Parley `FontContext`, and Parley
`LayoutContext` for specialized painting. Its convenience methods cover
solid rectangles, borders, single-style text, and text measurement. Custom
components should not introduce automatic subtree clipping.

## Children and slots

Container layout methods add constraints and return their children through
`Widget_type::Visual(Vec<Child>)`. Allocate children through the supplied
`Slots`; the `display!` macro assigns a call-site-stable slot. Dynamic children
should use stable IDs with `Slots::set`. Preserve the existing child hierarchy
and use `Child::fill` only where the component explicitly needs the solver
maximize objective.

Compositions can return `Widget_type::Virtual` from `Renderable::layout` to
lay out another renderable with the same hitbox, problem, focus provider, and
slots. This flattens the returned renderable rather than adding a component
node; the virtual renderable's own render callback is not invoked.

## Delegation derives

`vizual-macros` provides `Control` and `Renderable` derives. A single-field
wrapper delegates automatically, or `#[control(field = name)]` and
`#[renderable(field = name)]` select a named field. The generated render
method uses framework `Rect`, `Paint_context`, and normalized events.

## Guidelines

- Return `Vizual_msg::none()` for unhandled events.
- Use `new_propagated` when an ancestor must also observe an event.
- Keep paint deterministic and move I/O to Tokio tasks.
- Request rerender after background-visible state changes.
- Use Parley metrics rather than character counts for text geometry.
- Do not hold shared component guards across unrelated async work.
