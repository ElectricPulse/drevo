pub mod context;
pub(crate) mod debug;

use async_recursion::async_recursion;
use color_eyre::eyre::Result;
use std::sync::{Arc, Weak};
use vello::{Scene as VelloScene, kurbo::Affine};

use crate::{
    Signal,
    focus::FocusedPath,
    geometry::Direction,
    graphics::text::TextContext,
    layouter::{Formula, Solution, hitbox::Hitbox, variables::Variables},
    slot::manager::SlotRecords,
    state::Store,
    sync::{Mutex, MutexGuard},
    theme::Theme,
    widget::{FocusProvider, LayoutInput, RenderInput, Widget},
};

use self::{context::ComponentContext, debug::ComponentDebug};

pub type Id = u64;

pub type Child = SharedComponent;

pub type Children = Vec<Child>;

pub type Parent = Option<ChildReference>;

pub struct Component {
    /// Stable identity used by layout-state subscriptions.
    pub(crate) id: Id,
    pub name: String,
    pub(crate) debug: ComponentDebug,
    pub(crate) hitbox: Hitbox,
    pub(crate) formula: Option<Formula>,
    pub(crate) variables: Arc<Variables>,
    pub widget: Widget,
    // TODO: Convert focusability/focus tracking into reactive state when per-component
    // relayouting is implemented, so a focus change only notifies components that subscribe to
    // it. Refocusing currently relayouts the whole tree, so storing this as a bool is sufficient.
    pub focusable: bool,
    pub parent: Parent,
    pub children: Children,
    // TODO: Replace `children` and `layout_children` with a child enum that distinguishes
    // graphical children from virtual children. Logical children are still experimental, and
    // introducing that enum requires changing the `WidgetTrait` layout signature.
    /// Includes logical children, which participate in layout and formula assembly.
    pub(crate) layout_children: Children,
    pub slot_manager: SlotRecords,
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
pub struct SharedComponent {
    component: Arc<Mutex<Component>>,
}

pub struct RenderContext<'a> {
    pub(crate) focused_path: &'a FocusedPath,
    pub(crate) solution: &'a Solution,
}

impl From<SharedComponent> for Parent {
    fn from(value: SharedComponent) -> Self {
        Some(value.as_reference())
    }
}

impl SharedComponent {
    pub fn new(component: Arc<Mutex<Component>>) -> Self {
        Self { component }
    }

    pub async fn lock(&self) -> Result<MutexGuard<'_, Component>> {
        self.component.lock().await
    }

    pub fn compare(&self, node: &SharedComponent) -> bool {
        Arc::ptr_eq(&self.component, &node.component)
    }

    pub(crate) fn identity(&self) -> usize {
        Arc::as_ptr(&self.component) as usize
    }

    pub fn as_reference(&self) -> ChildReference {
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
        problem: &ComponentContext,
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
        rerender: Signal,
        theme: Store<Theme>,
        focused_path: &FocusedPath,
        parent_reference: Parent,
        parent: Hitbox,
        _problem: ComponentContext,
        text_context: &mut TextContext,
        root: &SharedComponent,
    ) -> Result<Children> {
        let mut this = self.lock().await?;

        if this.formula.is_some() {
            return Ok(this.layout_children.clone());
        }

        // Slots reset a child before its parent configures it.  Parent layouts (such as Axis)
        // deliberately mark child edges independent before the child's own layout begins, so a
        // second reset here would erase that parent-owned choice.
        let mut problem = ComponentContext::new(Arc::new(Mutex::new(Formula::new(Arc::clone(
            &this.variables,
        )))));

        this.parent = parent_reference;
        problem.component_path.push(this.name.clone());
        let relayout = rerender.for_component(this.id);
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

            let mut focus = FocusProvider::new(focused_path.contains(self));

            let children = {
                let mut slots = slot_manager.slots(hitbox);
                let input = LayoutInput {
                    relayout,
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
        rerender: Signal,
        theme: Store<Theme>,
        focused_path: &FocusedPath,
        children: Children,
        mut problem: ComponentContext,
        text_context: &mut TextContext,
        root: &SharedComponent,
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
                    rerender.clone(),
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
                    rerender.clone(),
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
        rerender: crate::Signal,
        theme: Store<Theme>,
        scene: &mut crate::graphics::scene::Scene<'_>,
        text_context: &mut TextContext,
        context: &RenderContext<'_>,
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
            let mut logical_scene = VelloScene::new();
            {
                let mut logical_scene_wrapper =
                    crate::graphics::scene::Scene::new(&mut logical_scene);

                self.render_component(
                    rerender.clone(),
                    theme.clone(),
                    &mut logical_scene_wrapper,
                    text_context,
                    context,
                )
                .await?;

                for mut child in children {
                    child
                        .render(
                            rerender.clone(),
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
            self.render_component(
                rerender.clone(),
                theme.clone(),
                scene,
                text_context,
                context,
            )
            .await?;

            for mut child in children {
                child
                    .render(
                        rerender.clone(),
                        theme.clone(),
                        scene,
                        text_context,
                        context,
                    )
                    .await?;
            }
        }

        Ok(())
    }

    async fn render_component(
        &mut self,
        rerender: Signal,
        theme: Store<Theme>,
        scene: &mut crate::graphics::scene::Scene<'_>,
        text_context: &mut TextContext,
        context: &RenderContext<'_>,
    ) -> Result<()> {
        let hitbox = {
            let this = self.lock().await?;
            this.hitbox.get_resolved(context.solution)
        };

        let mut this = self.lock().await?;
        let focus = FocusProvider::new(context.focused_path.contains(self));
        let input = RenderInput {
            rerender,
            theme,
            focus: &focus,
            hitbox,
            scene,
            text_context,
            context,
        };
        this.widget.render(input).await?;

        Ok(())
    }
}

pub type ChildReference = Weak<Mutex<Component>>;

#[cfg(test)]
mod tests;
