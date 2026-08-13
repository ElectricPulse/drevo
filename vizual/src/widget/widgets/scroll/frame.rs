use async_recursion::async_recursion;
use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

use crate::{
    Vizual_command, Vizual_msg,
    component::{Children, Shared_component, context::Component_context},
    event::{Event, Pointer_event},
    geometry::{Direction, Point},
    graphics::text::Text_context,
    layouter::{Solution, hitbox::Hitbox},
    slot::manager::Slots,
    state::State,
    theme::Theme,
    widget::{Focus_provider, Widget, Widget_trait},
};

/// Owns the independently laid-out coordinate space rendered by [`super::Scroll`].
#[derive(Clone)]
pub(super) struct Frame {
    child: Widget,
}

impl Frame {
    pub(super) fn new(child: Widget) -> Self {
        Self { child }
    }
}

#[derive(Clone, Copy)]
struct Pointer_rank {
    layer: usize,
    area: f64,
    tree_order: usize,
}

impl Pointer_rank {
    fn outranks(self, other: Self) -> bool {
        self.layer > other.layer
            || (self.layer == other.layer
                && (self.area < other.area
                    || (self.area == other.area && self.tree_order > other.tree_order)))
    }
}

struct Pointer_target {
    component: Shared_component,
    rank: Pointer_rank,
}

#[async_recursion]
async fn find_pointer_target(
    component: Shared_component,
    position: Point,
    solution: &Solution,
    inherited_layer: usize,
    is_frame: bool,
    tree_order: &mut usize,
    target: &mut Option<Pointer_target>,
) -> Result<()> {
    let layer = inherited_layer.max(component.layer);
    let (hitbox, logical, children) = {
        let component = component.lock().await?;
        (
            component.hitbox.get_resolved(solution),
            component.logical,
            component.children.clone(),
        )
    };
    let current_tree_order = *tree_order;
    *tree_order += 1;

    if !is_frame && !logical && hitbox.contains(position) {
        let rank = Pointer_rank {
            layer,
            area: hitbox.size.width * hitbox.size.height,
            tree_order: current_tree_order,
        };
        let replace = target
            .as_ref()
            .is_none_or(|target| rank.outranks(target.rank));
        if replace {
            *target = Some(Pointer_target {
                component: component.clone(),
                rank,
            });
        }
    }

    if logical && !is_frame {
        return Ok(());
    }

    for child in children {
        find_pointer_target(child, position, solution, layer, false, tree_order, target).await?;
    }

    Ok(())
}

pub(super) async fn forward_pointer(
    frame: &Shared_component,
    pointer: &Pointer_event,
    solution: &Solution,
) -> Result<Vizual_msg> {
    let mut target = None;
    let mut tree_order = 0;
    find_pointer_target(
        frame.clone(),
        pointer.position,
        solution,
        0,
        true,
        &mut tree_order,
        &mut target,
    )
    .await?;

    let Some(mut node) = target.map(|target| target.component) else {
        return Vizual_msg::none();
    };
    let event = Event::Pointer(*pointer);
    let mut total_message = Vizual_msg::bare();

    loop {
        if node.compare(frame) {
            return Ok(total_message);
        }

        let mut component = node.lock().await?;
        let parent = component.parent.clone();
        if component.focusable {
            return Vizual_msg::new(Vizual_command::Focus(node.as_reference()));
        }

        total_message.join(component.widget.forward_event(&event).await?);
        if !total_message.propagate {
            return Ok(total_message);
        }

        let Some(parent) = parent.and_then(|parent| parent.upgrade()) else {
            return Ok(total_message);
        };
        drop(component);
        node = Shared_component::new(parent);
    }
}

#[async_trait]
impl Widget_trait for Frame {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: State<Theme>,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        hitbox.make_independent();
        for direction in [Direction::Horizontal, Direction::Vertical] {
            problem
                .constrain(crate::constraint!(
                    hitbox.get_start_position(direction) == 0
                ))
                .await?;
        }

        Ok(vec![display!(self.child.clone())])
    }
}
