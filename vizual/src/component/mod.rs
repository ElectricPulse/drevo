pub mod context;

use async_recursion::async_recursion;
use color_eyre::eyre::Result;
use derive_new::new;
use std::sync::{Arc, Weak};

use crate::{
    constraint,
    focus::Focus,
    geometry::Direction,
    layouter::{
        Solution, constraints::Objective, expression::Expression, hitbox::Hitbox,
        variables::Variables,
    },
    slot::{
        Component_slot,
        manager::{Slot_records, Slots},
    },
    sync::{Mutex, MutexGuard},
    text::Text_context,
    widget::{Control, Focus_provider, Widget, Widget_trait, Widget_type},
};

use self::context::Component_context;

pub type Id = u64;

pub type Children = Vec<Shared_component>;

pub type Parent = Option<Child_reference>;

pub struct Component {
    pub name: String,
    pub hitbox: Hitbox,
    pub widget: Widget,
    pub focusable: bool,
    pub parent: Parent,
    pub children: Children,
    pub slot_manager: Slot_records,
    pub virtual_child: Component_slot,
    pub(crate) variables: Arc<Variables>,
}

#[derive(Clone, new)]
pub struct Shared_component(Arc<Mutex<Component>>);

#[derive(Clone, Copy)]
pub enum Horizontal_anchor {
    Left,
    Right,
}

#[derive(Clone, Copy)]
pub enum Vertical_anchor {
    Top,
    Bottom,
}

impl Control for Shared_component {}

#[async_trait::async_trait]
impl Widget_trait for Shared_component {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: Hitbox,
        _problem: Component_context,
        _text_context: &mut Text_context,
        _slots: &mut Slots,
    ) -> Result<Widget_type> {
        Ok(Widget_type::Visual(vec![self.clone()]))
    }
}

impl From<Shared_component> for Parent {
    fn from(value: Shared_component) -> Self {
        Some(value.as_reference())
    }
}

impl Shared_component {
    pub async fn lock(&self) -> Result<MutexGuard<'_, Component>> {
        self.0.lock().await
    }

    pub fn compare(&self, node: &Shared_component) -> bool {
        Arc::ptr_eq(&self.0, &node.0)
    }

    pub fn as_reference(&self) -> Child_reference {
        Arc::downgrade(&self.0)
    }

    pub async fn get_hitbox(&self) -> Result<Hitbox> {
        Ok(self.lock().await?.hitbox)
    }

    #[async_recursion]
    pub(crate) async fn dismount(&mut self) -> Result<()> {
        let (children, hitbox, variables) = {
            let component = self.lock().await?;
            (
                component.children.clone(),
                component.hitbox,
                Arc::clone(&component.variables),
            )
        };

        hitbox.remove_variables(&variables);
        for mut child in children {
            child.dismount().await?;
        }

        Ok(())
    }

    pub async fn fill(self, problem: Component_context) -> Result<Self> {
        let priority = 1;
        let hitbox = self.get_hitbox().await?;

        problem
            .maximize(
                Expression::from(hitbox.get_dimension(Direction::Horizontal)),
                priority,
            )
            .await?;
        problem
            .maximize(
                Expression::from(hitbox.get_dimension(Direction::Vertical)),
                priority,
            )
            .await?;

        Ok(self)
    }

    pub async fn anchor(
        self,
        horizontal: Horizontal_anchor,
        vertical: Vertical_anchor,
        objective: Objective,
    ) -> Result<Self> {
        let priority = 1;
        let (hitbox, problem) = {
            let child = self.lock().await?;
            (child.hitbox, child.slot_manager.problem.clone())
        };
        let horizontal = match horizontal {
            Horizontal_anchor::Left => Expression::from(hitbox.x),
            Horizontal_anchor::Right => hitbox.get_end_position(Direction::Horizontal),
        };
        let vertical = match vertical {
            Vertical_anchor::Top => Expression::from(hitbox.y),
            Vertical_anchor::Bottom => hitbox.get_end_position(Direction::Vertical),
        };
        let vertex = horizontal + vertical;

        match objective {
            Objective::Maximize => problem.maximize(vertex, priority).await?,
            Objective::Minimize | Objective::Minimize_difference => {
                problem.minimize(vertex, priority).await?
            }
        }

        Ok(self)
    }

    pub fn compare_with_reference(&self, node: &Child_reference) -> bool {
        let Some(node) = node.upgrade() else {
            return false;
        };

        self.compare(&Shared_component(node))
    }

    pub async fn layout(
        &mut self,
        parent: Parent,
        mut problem: Component_context,
        text_context: &mut Text_context,
    ) -> Result<Children> {
        let mut this = self.lock().await?;

        this.parent = parent;
        problem.component_path.push(this.name.clone());
        let children = {
            let Component {
                widget,
                slot_manager,
                virtual_child,
                hitbox,
                focusable,
                ..
            } = &mut *this;

            let mut focus = Focus_provider::new(false);

            let children = {
                let mut slots = slot_manager.slots();
                let widget_type = widget
                    .layout(
                        &mut focus,
                        *hitbox,
                        problem.clone(),
                        text_context,
                        &mut slots,
                    )
                    .await?;

                match widget_type {
                    Widget_type::Virtual(widget) => {
                        let child = virtual_child
                            .set_init(widget, problem.clone(), Some(*hitbox))
                            .await?;
                        vec![child]
                    }
                    Widget_type::Visual(children) => children,
                }
            };
            *focusable = focus.is_active();

            for child in children.iter() {
                let child_hitbox = child.get_hitbox().await?;
                let hitbox = *hitbox;

                for direction in [Direction::Horizontal, Direction::Vertical] {
                    let (start_bound_name, end_bound_name) = match direction {
                        Direction::Horizontal => {
                            ("child_horizontal_start_bound", "child_horizontal_end_bound")
                        }
                        Direction::Vertical => {
                            ("child_vertical_start_bound", "child_vertical_end_bound")
                        }
                    };

                    problem
                        .constrain(
                            constraint!(
                                hitbox.get_start_position(direction)
                                    <= child_hitbox.get_start_position(direction)
                            )
                            .set_name(start_bound_name.to_string()),
                        )
                        .await?;
                    problem
                        .constrain(
                            constraint!(
                                hitbox.get_end_position(direction)
                                    >= child_hitbox.get_end_position(direction)
                            )
                            .set_name(end_bound_name.to_string()),
                        )
                        .await?;
                }
            }

            // TODO: Ordered containers such as `Layout` can shrink-wrap their main axis exactly
            // by constraining it from the first visible child's start to the last visible child's
            // end. Once components provide exact bounds for an axis, remove its generic
            // shrink-wrap objective here; if priority 0 becomes empty, its solve can be skipped.
            problem
                .minimize(Expression::from(hitbox.dimensions.width), 0)
                .await?;
            problem
                .minimize(Expression::from(hitbox.dimensions.height), 0)
                .await?;

            slot_manager.evaluate().await?;
            children
        };

        this.children = children.clone();

        Ok(children)
    }

    #[async_recursion]
    pub async fn layout_children(
        &mut self,
        children: Children,
        mut problem: Component_context,
        text_context: &mut Text_context,
    ) -> Result<()> {
        problem.component_path.push(self.lock().await?.name.clone());

        for mut child in children {
            let grandchildren = child
                .clone()
                .layout(self.clone().into(), problem.clone(), text_context)
                .await?;
            child
                .layout_children(grandchildren, problem.clone(), text_context)
                .await?;
        }

        Ok(())
    }

    pub async fn render(
        &mut self,
        focus: Focus,
        display: &mut crate::display::Display<'_>,
        solution: &Solution,
    ) -> Result<()> {
        // TODO: I think the cloning of children is not necessary here
        let children = {
            let mut this = self.lock().await?;

            let hitbox = this.hitbox.get_resolved(solution);
            let focused = focus.compare(self);
            let mut focus = Focus_provider::new(focused);
            let maybe_hitbox = this.widget.render(&mut focus, hitbox, display).await?;
            this.focusable = focus.is_active();

            if let Some(hitbox) = maybe_hitbox {
                this.hitbox = hitbox;
            };

            this.children.clone()
        };

        self.render_children(children, focus, display, solution)
            .await
    }

    #[async_recursion]
    pub async fn render_children(
        &mut self,
        children: Children,
        focus: Focus,
        display: &mut crate::display::Display<'_>,
        solution: &Solution,
    ) -> Result<()> {
        for mut child in children {
            child.render(focus.clone(), display, solution).await?;
        }

        Ok(())
    }
}

pub type Child_reference = Weak<Mutex<Component>>;
