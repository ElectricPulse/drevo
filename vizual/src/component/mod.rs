pub mod context;
pub(crate) mod debug;

use async_recursion::async_recursion;
use color_eyre::eyre::{ContextCompat, Result};
use std::sync::{Arc, Weak};
use vello::{Scene as Vello_scene, kurbo::Affine};

use crate::{
    Render,
    focus::Focused_path,
    geometry::{Direction, Rect},
    graphics::text::Text_context,
    layouter::{Solution, hitbox::Hitbox},
    slot::manager::Slot_records,
    state::Store,
    sync::{Mutex, MutexGuard},
    theme::Theme,
    widget::{Focus_provider, Widget},
};

use self::{context::Component_context, debug::Component_debug};

pub type Id = u64;

pub type Child = Shared_component;

pub type Children = Vec<Child>;

pub type Parent = Option<Child_reference>;

pub struct Component {
    pub name: String,
    pub(crate) debug: Component_debug,
    pub(crate) hitbox: Hitbox,
    pub widget: Widget,
    // TODO: Convert focusability/focus tracking into reactive state when per-component
    // relayouting is implemented, so a focus change only notifies components that subscribe to
    // it. Refocusing currently relayouts the whole tree, so storing this as a bool is sufficient.
    pub focusable: bool,
    pub parent: Parent,
    pub children: Children,
    pub slot_manager: Slot_records,
    /// Logical is a fantastic bodge if you want to disable two annoying default features that come with having parent hitboxes being smaller than child
    /// ie. child hitbox isnt contained in parent hitbox - like in a dialog window opened from a button
    /// 1. it disables the default overflow hidden rendering
    /// 2. it makes focus continue searching for focusables inside it even when the elements hitbox wasnt clicked
    /// setting it to true is like saying ignore this hitbox it's only exists so that things will align properly
    pub logical: bool,
}

/// A component as attached to its parent.
///
/// `layer` belongs to this child relationship rather than the component allocation. This keeps a
/// component's default layer at zero whenever a slot returns it while allowing callers to adjust
/// the value immediately after `display!()`.
#[derive(Clone)]
pub struct Shared_component {
    component: Arc<Mutex<Component>>,
    pub layer: usize,
}

#[derive(Clone)]
pub(crate) struct Layered_component {
    pub component: Shared_component,
    pub layer: usize,
    pub tree_order: usize,
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
        Self {
            component,
            layer: 0,
        }
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

    pub(crate) async fn layered_components(&self) -> Result<Vec<Layered_component>> {
        let mut components = Vec::new();
        self.collect_layered_components(0, &mut components).await?;
        Ok(components)
    }

    #[async_recursion]
    async fn collect_layered_components(
        &self,
        inherited_layer: usize,
        components: &mut Vec<Layered_component>,
    ) -> Result<()> {
        let layer = inherited_layer.max(self.layer);
        let component = self.lock().await?;
        let children = component.children.clone();

        drop(component);
        let tree_order = components.len();
        components.push(Layered_component {
            component: self.clone(),
            layer,
            tree_order,
        });

        for child in children {
            child.collect_layered_components(layer, components).await?;
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
        mut problem: Component_context,
        text_context: &mut Text_context,
    ) -> Result<Children> {
        let mut this = self.lock().await?;

        this.parent = parent_reference;
        problem.component_path.push(this.name.clone());
        let children = {
            let Component {
                widget,
                slot_manager,
                hitbox,
                focusable,
                logical,
                ..
            } = &mut *this;

            let mut focus = Focus_provider::new(focused_path.contains(self));

            let children = {
                let mut slots = slot_manager.slots(hitbox);
                let children = widget
                    .layout(
                        render,
                        theme,
                        &mut focus,
                        hitbox,
                        parent.clone(),
                        problem.clone(),
                        text_context,
                        &mut slots,
                        logical,
                    )
                    .await?;

                hitbox.constrain_shared(&parent, &problem).await?;

                children
            };
            *focusable = focus.is_active();

            slot_manager.evaluate().await?;
            children
        };

        this.children = children.clone();

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
                )
                .await?;
        }

        Ok(())
    }

    pub async fn render(
        &mut self,
        render: crate::Render,
        theme: Store<Theme>,
        scene: &mut crate::graphics::scene::Scene<'_>,
        text_context: &mut Text_context,
        context: &Render_context<'_>,
    ) -> Result<()> {
        let mut components = self.layered_components().await?;
        components.sort_by_key(|component| (component.layer, component.tree_order));

        for mut component in components {
            component
                .component
                .render_component(render.clone(), theme.clone(), scene, text_context, context)
                .await?;
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
        let (hitbox, mask) = {
            let this = self.lock().await?;
            let hitbox = this.hitbox.get_resolved(context.solution);
            let mut mask: Rect = hitbox.clone();
            let mut current_parent = this.parent.clone();

            while let Some(parent_ref) = current_parent {
                let parent = parent_ref
                    .upgrade()
                    .wrap_err("Found link to stale parent")?;
                let parent_lock = parent.lock().await?;

                current_parent = parent_lock.parent.clone();

                if parent_lock.logical {
                    // This should in reality just be continue statement as in ignore me but continue clipping
                    // but since it often is the case that parents of a logical component are often wrapped
                    // in Anchor or smth than its really hard to turn on logical for all of them
                    mask = hitbox;
                    break;
                }

                let parent_rect = parent_lock.hitbox.get_resolved(context.solution);
                mask = mask.intersect(parent_rect)
            }
            (hitbox, mask)
        };

        let mut logical_scene = Vello_scene::new();

        let maybe_hitbox = {
            let mut logical_scene_wrapper = crate::graphics::scene::Scene::new(&mut logical_scene);
            let mut this = self.lock().await?;
            let focused = context.focused_path.contains(self);
            let mut focus = Focus_provider::new(focused);
            let maybe_hitbox = this
                .widget
                .render(
                    render,
                    theme,
                    &mut focus,
                    hitbox,
                    &mut logical_scene_wrapper,
                    text_context,
                    context,
                )
                .await?;
            this.focusable = focus.is_active();
            maybe_hitbox
        };

        scene.append_clipped(&logical_scene, mask, Affine::IDENTITY);

        Ok(())
    }
}

pub type Child_reference = Weak<Mutex<Component>>;

#[cfg(test)]
mod tests;
