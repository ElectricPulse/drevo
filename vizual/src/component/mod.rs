pub mod context;
pub(crate) mod debug;

use async_recursion::async_recursion;
use color_eyre::eyre::Result;
use std::sync::{Arc, Weak};
use vello::{Scene as Vello_scene, kurbo::Affine};

use crate::{
    Render,
    focus::Focused_path,
    geometry::Direction,
    graphics::text::Text_context,
    layouter::{Formula, Solution, hitbox::Hitbox, variables::Variables},
    slot::manager::Slot_records,
    state::Store,
    sync::{Mutex, MutexGuard},
    theme::Theme,
    widget::{Focus_provider, Layout_input, Render_input, Widget},
};

use self::{context::Component_context, debug::Component_debug};

pub type Id = u64;

pub type Child = Shared_component;

pub type Children = Vec<Child>;

pub type Parent = Option<Child_reference>;

pub struct Component {
    /// Stable identity used by layout-state subscriptions.
    pub(crate) id: Id,
    pub name: String,
    pub(crate) debug: Component_debug,
    pub(crate) hitbox: Hitbox,
    pub(crate) formula: Option<Formula>,
    pub(crate) variables: Arc<Variables>,
    /// The component-targeted signal installed during layout and reused by event handlers.
    pub(crate) layout_signal: Option<Render>,
    pub widget: Widget,
    // TODO: Convert focusability/focus tracking into reactive state when per-component
    // relayouting is implemented, so a focus change only notifies components that subscribe to
    // it. Refocusing currently relayouts the whole tree, so storing this as a bool is sufficient.
    pub focusable: bool,
    pub parent: Parent,
    pub children: Children,
    /// Includes logical children, which participate in layout and formula assembly.
    pub(crate) layout_children: Children,
    pub slot_manager: Slot_records,
    /// Marks this component as a logical child rather than a graphical child.
    ///
    /// Excludes the component from the layout parent's `children` list after `layout`,
    /// allowing it to be positioned relative to the parent while being mounted to a different
    /// graphical container (such as `root`).
    pub logical: bool,
    /// When true, this component acts as a clipping mask for itself and all its graphical children.
    pub mask: bool,
}

/// A component as attached to its parent.
#[derive(Clone)]
pub struct Shared_component {
    component: Arc<Mutex<Component>>,
}

pub struct Render_context<'a> {
    pub(crate) focused_path: &'a Focused_path,
    pub(crate) solution: &'a Solution,
}

impl From<Shared_component> for Parent {
    fn from(value: Shared_component) -> Self {
        Some(value.as_reference())
    }
}

impl Shared_component {
    pub fn new(component: Arc<Mutex<Component>>) -> Self {
        Self { component }
    }

    pub async fn lock(&self) -> Result<MutexGuard<'_, Component>> {
        self.component.lock().await
    }

    pub fn compare(&self, node: &Shared_component) -> bool {
        Arc::ptr_eq(&self.component, &node.component)
    }

    pub(crate) fn identity(&self) -> usize {
        Arc::as_ptr(&self.component) as usize
    }

    pub fn as_reference(&self) -> Child_reference {
        Arc::downgrade(&self.component)
    }

    pub async fn get_hitbox(&self) -> Result<Hitbox> {
        Ok(self.lock().await?.hitbox.clone())
    }

    #[async_recursion]
    pub(crate) async fn add_cached_formulas(
        &self,
        problem: &mut crate::layouter::Problem,
    ) -> Result<()> {
        let (formula, children) = {
            let component = self.lock().await?;
            (component.formula.clone(), component.layout_children.clone())
        };
        let formula = formula.expect("formula must be cached before solving");
        problem.add_formula(&formula);
        for child in children {
            child.add_cached_formulas(problem).await?;
        }
        Ok(())
    }

    #[async_recursion]
    pub(crate) async fn invalidate_formula(&self, id: Id) -> Result<bool> {
        let children = {
            let mut component = self.lock().await?;
            if component.id == id {
                component.formula = None;
                return Ok(true);
            }
            component.layout_children.clone()
        };
        for child in children {
            if child.invalidate_formula(id).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub async fn share_dimension(
        &self,
        parent: Hitbox,
        problem: &Component_context,
        direction: Direction,
    ) -> Result<()> {
        self.lock()
            .await?
            .hitbox
            .share_dimension(&parent, problem, direction)
            .await
    }

    #[async_recursion]
    pub(crate) async fn dismount(&mut self) -> Result<()> {
        let children = self.lock().await?.children.clone();

        for mut child in children {
            child.dismount().await?;
        }

        Ok(())
    }

    pub(crate) async fn layout(
        &mut self,
        render: Render,
        theme: Store<Theme>,
        focused_path: &Focused_path,
        parent_reference: Parent,
        parent: Hitbox,
        _problem: Component_context,
        text_context: &mut Text_context,
        root: &Shared_component,
    ) -> Result<Children> {
        let mut this = self.lock().await?;

        if this.formula.is_some() {
            return Ok(this.layout_children.clone());
        }

        // Slots reset a child before its parent configures it.  Parent layouts (such as Axis)
        // deliberately mark child edges independent before the child's own layout begins, so a
        // second reset here would erase that parent-owned choice.
        let mut problem = Component_context::new(Arc::new(Mutex::new(Formula::new(Arc::clone(
            &this.variables,
        )))));

        this.parent = parent_reference;
        problem.component_path.push(this.name.clone());
        let render = render.for_component(this.id);
        this.layout_signal = Some(render.clone());
        let children = {
            let Component {
                widget,
                slot_manager,
                hitbox,
                focusable,
                mask,
                ..
            } = &mut *this;

            slot_manager.set_problem(problem.clone());

            let mut focus = Focus_provider::new(focused_path.contains(self));

            let children = {
                let mut slots = slot_manager.slots(hitbox);
                let input = Layout_input {
                    render,
                    theme,
                    focus: &mut focus,
                    hitbox,
                    parent: parent.clone(),
                    problem: problem.clone(),
                    text_context,
                    slots: &mut slots,
                    root,
                    mask,
                };
                let children = widget.layout(input).await?;

                hitbox.constrain_shared(&parent, &problem).await?;

                children
            };
            *focusable = focus.is_active();

            slot_manager.evaluate().await?;
            children
        };

        this.formula = Some(problem.lock().await?.clone());
        this.layout_children = children.clone();

        let mut non_logical_children = Vec::new();
        for child in &children {
            if !child.lock().await?.logical {
                non_logical_children.push(child.clone());
            }
        }
        this.children = non_logical_children;

        Ok(children)
    }

    #[async_recursion]
    pub(crate) async fn layout_children(
        &mut self,
        render: Render,
        theme: Store<Theme>,
        focused_path: &Focused_path,
        children: Children,
        mut problem: Component_context,
        text_context: &mut Text_context,
        root: &Shared_component,
    ) -> Result<()> {
        let hitbox = {
            let component = self.lock().await?;
            problem.component_path.push(component.name.clone());
            component.hitbox.clone()
        };

        for child in &children {
            let mut child = child.clone();
            let layout_parent = hitbox.clone();
            let grandchildren = child
                .clone()
                .layout(
                    render.clone(),
                    theme.clone(),
                    focused_path,
                    self.clone().into(),
                    layout_parent,
                    problem.clone(),
                    text_context,
                    root,
                )
                .await?;
            child
                .layout_children(
                    render.clone(),
                    theme.clone(),
                    focused_path,
                    grandchildren,
                    problem.clone(),
                    text_context,
                    root,
                )
                .await?;
        }

        Ok(())
    }

    #[async_recursion]
    pub async fn render(
        &mut self,
        render: crate::Render,
        theme: Store<Theme>,
        scene: &mut crate::graphics::scene::Scene<'_>,
        text_context: &mut Text_context,
        context: &Render_context<'_>,
    ) -> Result<()> {
        let (is_mask, hitbox, children) = {
            let this = self.lock().await?;
            (
                this.mask,
                this.hitbox.get_resolved(context.solution),
                this.children.clone(),
            )
        };

        if is_mask {
            let mut logical_scene = Vello_scene::new();
            {
                let mut logical_scene_wrapper =
                    crate::graphics::scene::Scene::new(&mut logical_scene);

                self.render_component(
                    render.clone(),
                    theme.clone(),
                    &mut logical_scene_wrapper,
                    text_context,
                    context,
                )
                .await?;

                for mut child in children {
                    child
                        .render(
                            render.clone(),
                            theme.clone(),
                            &mut logical_scene_wrapper,
                            text_context,
                            context,
                        )
                        .await?;
                }
            }
            scene.append_clipped(&logical_scene, hitbox, Affine::IDENTITY);
        } else {
            self.render_component(render.clone(), theme.clone(), scene, text_context, context)
                .await?;

            for mut child in children {
                child
                    .render(render.clone(), theme.clone(), scene, text_context, context)
                    .await?;
            }
        }

        Ok(())
    }

    async fn render_component(
        &mut self,
        render: Render,
        theme: Store<Theme>,
        scene: &mut crate::graphics::scene::Scene<'_>,
        text_context: &mut Text_context,
        context: &Render_context<'_>,
    ) -> Result<()> {
        let hitbox = {
            let this = self.lock().await?;
            this.hitbox.get_resolved(context.solution)
        };

        let mut this = self.lock().await?;
        let focused = context.focused_path.contains(self);
        let mut focus = Focus_provider::new(focused);
        let input = Render_input {
            render,
            theme,
            focus: &mut focus,
            hitbox,
            scene,
            text_context,
            context,
        };
        this.widget.render(input).await?;
        this.focusable = focus.is_active();

        Ok(())
    }
}

pub type Child_reference = Weak<Mutex<Component>>;

#[cfg(test)]
mod tests;
