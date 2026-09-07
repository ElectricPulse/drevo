use std::path::PathBuf;

use async_trait::async_trait;
use color_eyre::eyre::Result;
use drevo::{
    DrevoMsg,
    component::Children,
    constraint,
    event::{Event, PointerButton},
    geometry::{Direction, Point, Rect},
    id,
    layouter::constraints::prohibit_overlap,
    macros::display,
    state::Store,
    widget::{
        LayoutInput, MouseEvent, RenderInput, Widget, WidgetTrait,
        widgets::{
            default_root::DefaultRoot,
            icon::Icon,
            layout::axis::Axis,
            paper::Paper,
            positioning::{
                align::Align,
                anchor::Anchor,
                space::{Space, SpaceMode},
            },
            text::{Text, TextStyle},
        },
    },
};
use lucide_icons::Icon as LucideIcon;

/// An icon that starts at `position` and follows the pointer while held.
#[derive(Clone)]
struct DraggableIcon {
    position: Store<Point>,
    last_pointer: Store<Option<Point>>,
    grid_hitbox: Store<Rect>,
    resolved_hitbox: Store<Rect>,
}

/// A labeled card around an icon.
#[derive(Clone)]
struct IconContainer {
    label: String,
    icon: Widget,
}

impl IconContainer {
    fn new(label: impl Into<String>, icon: impl WidgetTrait) -> Self {
        Self {
            label: label.into(),
            icon: icon.as_any(),
        }
    }
}

#[async_trait]
impl WidgetTrait for IconContainer {
    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            theme,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let theme = theme.affect(relayout).await?;
        let block = Space::top(self.icon.clone(), theme.units.em * 1.6, SpaceMode::Padding);

        let block = Paper::new(Axis::new(
            Direction::Vertical,
            (
                Anchor::h_middle(Text::new(self.label.clone())),
                Anchor::h_middle(block),
            ),
        ));
        Ok(vec![display!(block)])
    }
}

impl DraggableIcon {
    fn new(position: Point, grid_hitbox: Store<Rect>) -> Self {
        Self {
            position: Store::new(position),
            last_pointer: Store::new(None),
            grid_hitbox,
            resolved_hitbox: Store::new(Rect::default()),
        }
    }

    async fn position_is_inside_grid(&self, position: Point) -> Result<bool> {
        let grid = *self.grid_hitbox.read().await?;
        let current = *self.resolved_hitbox.read().await?;
        let target = Rect::new(
            grid.origin.x + position.x,
            grid.origin.y + position.y,
            current.size.width,
            current.size.height,
        );

        Ok(target.origin.x >= grid.origin.x
            && target.origin.y >= grid.origin.y
            && target.right() <= grid.right()
            && target.bottom() <= grid.bottom())
    }
}

#[async_trait]
impl WidgetTrait for DraggableIcon {
    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            focus,
            hitbox,
            parent,
            formula,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        focus.set_interactive(true);
        let position = *self.position.affect(relayout).await?;

        hitbox.make_independent();
        formula.constrain(
            id!(),
            constraint!(
                hitbox.get_start_position(Direction::Horizontal)
                    == parent.get_start_position(Direction::Horizontal) + position.x
            ),
        )?;
        formula.constrain(
            id!(),
            constraint!(
                hitbox.get_start_position(Direction::Vertical)
                    == parent.get_start_position(Direction::Vertical) + position.y
            ),
        )?;

        Ok(vec![display!(IconContainer::new(
            "Drag me",
            Icon::new(LucideIcon::Grip).style(TextStyle {
                size: 50.0,
                ..TextStyle::default()
            }),
        ))])
    }

    async fn on_mouse_event(&mut self, input: MouseEvent<'_>) -> Result<DrevoMsg> {
        match input.event {
            Event::Pointer(pointer) if pointer.button == PointerButton::Primary => {
                self.last_pointer.set(Some(pointer.position)).await?;
            }
            Event::PointerMoved(pointer) => {
                let Some(previous) = *self.last_pointer.read().await? else {
                    return DrevoMsg::none();
                };
                let current = *self.position.read().await?;
                let next = Point::new(
                    current.x + pointer.x - previous.x,
                    current.y + pointer.y - previous.y,
                );
                if self.position_is_inside_grid(next).await? {
                    self.position.set(next).await?;
                }
                self.last_pointer.set(Some(*pointer)).await?;
            }
            Event::PointerReleased(pointer) if pointer.button == PointerButton::Primary => {
                self.last_pointer.set(None).await?;
            }
            _ => {}
        }
        DrevoMsg::none()
    }

    async fn render(&mut self, RenderInput { hitbox, .. }: RenderInput<'_, '_>) -> Result<()> {
        self.resolved_hitbox.set(hitbox).await
    }
}

#[derive(Clone)]
struct Gallery {
    draggable_icon: DraggableIcon,
    grid_hitbox: Store<Rect>,
}

impl Gallery {
    fn new() -> Self {
        let grid_hitbox = Store::new(Rect::default());
        Self {
            draggable_icon: DraggableIcon::new(Point::new(240.0, 48.0), grid_hitbox.clone()),
            grid_hitbox,
        }
    }
}

#[async_trait]
impl WidgetTrait for Gallery {
    async fn layout(
        &mut self,
        LayoutInput {
            hitbox,
            formula,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let static_icon = slots
            .set(
                0,
                Align::top_left(IconContainer::new(
                    "Always gonna try to be on the top left",
                    Icon::new(LucideIcon::MoveLeft).style(TextStyle {
                        size: 128.0,
                        ..TextStyle::default()
                    }),
                )),
            )
            .await?;
        let draggable_icon = slots.set(1, self.draggable_icon.clone()).await?;
        let items = [static_icon.clone(), draggable_icon.clone()];

        // Keep the draggable card away from the centered static card.
        const GAP: f64 = 8.0;
        prohibit_overlap(
            formula,
            draggable_icon.get_hitbox().await?,
            static_icon.get_hitbox().await?,
            GAP,
        )?;

        for item in &items {
            let item_hitbox = item.get_hitbox().await?;
            for direction in [Direction::Horizontal, Direction::Vertical] {
                formula.constrain(
                    id!(),
                    constraint!(
                        item_hitbox.get_start_position(direction)
                            >= hitbox.get_start_position(direction)
                    ),
                )?;
                formula.constrain(
                    id!(),
                    constraint!(
                        item_hitbox.get_end_position(direction)
                            <= hitbox.get_end_position(direction)
                    ),
                )?;
            }
        }

        Ok(items.into())
    }

    async fn render(&mut self, RenderInput { hitbox, .. }: RenderInput<'_, '_>) -> Result<()> {
        self.grid_hitbox.set(hitbox).await
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    drevo::init_logging(None::<PathBuf>)?;

    drevo::run(
        "Drevo image gallery",
        DefaultRoot::new("Drevo image gallery", Gallery::new()),
    )
}
