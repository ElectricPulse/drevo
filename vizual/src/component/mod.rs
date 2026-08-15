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
    /// Makes this component a traversal boundary: the component is included, but its children are
    /// not visited.
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
        let logical = component.logical;
        drop(component);
        let tree_order = components.len();
        components.push(Layered_component {
            component: self.clone(),
            layer,
            tree_order,
        });

        if logical {
            return Ok(());
        }

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
        let (hitbox, parent_hitbox) = {
            let this = self.lock().await?;
            let hitbox = this.hitbox.get_resolved(context.solution);
            let mut clip_rect: Option<Rect> = None;
            let mut current_parent = this.parent.clone();
            while let Some(parent_ref) = current_parent {
                let parent = parent_ref.upgrade().wrap_err("Found link to stale parent")?;
                let parent_lock = parent.lock().await?;
                let parent_rect = parent_lock.hitbox.get_resolved(context.solution);
                clip_rect = match clip_rect {
                    Some(rect) => Some(rect.intersect(parent_rect)),
                    None => Some(parent_rect),
                };
                current_parent = parent_lock.parent.clone();
            }
            (hitbox, clip_rect)
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

        if let Some(parent_hitbox) = parent_hitbox {
            scene.append_clipped(&logical_scene, parent_hitbox, Affine::IDENTITY);
        } else {
            scene.scene.append(&logical_scene, None);
        }

        if let Some(hitbox) = maybe_hitbox {
            self.lock().await?.hitbox.point_to(&hitbox);
        };

        Ok(())
    }
}

pub type Child_reference = Weak<Mutex<Component>>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        focus::Focus,
        layouter::{Problem, variables::Variables},
        widget::Widget_trait,
    };

    #[derive(Clone)]
    struct Empty_widget;

    #[derive(Clone, vizual_macros::Widget_trait)]
    struct Derived_widget {
        widget: Empty_widget,
    }

    #[async_trait::async_trait]
    impl Widget_trait for Empty_widget {
        async fn layout(
            &mut self,
            _render: crate::Render,
            _theme: crate::state::Store<crate::theme::Theme>,
            _focus: &mut crate::widget::Focus_provider,
            _hitbox: &mut Hitbox,
            _parent: Hitbox,
            _problem: Component_context,
            _text_context: &mut crate::graphics::text::Text_context,
            _slots: &mut crate::slot::manager::Slots,
        ) -> Result<Children> {
            Ok(vec![])
        }
    }

    #[test]
    fn widget_derive_forwards_the_current_trait_interface() {
        fn assert_widget<Widget: Widget_trait>() {}

        assert_widget::<Derived_widget>();
        let _ = Derived_widget {
            widget: Empty_widget,
        };
    }

    fn component(
        name: &str,
        variables: &Variables,
        problem: Component_context,
    ) -> Shared_component {
        Shared_component::new(Arc::new(Mutex::new(Component {
            name: name.to_string(),
            debug: Component_debug::new("test".to_string()),
            hitbox: Hitbox::new(
                variables,
                name.to_string(),
                name.to_string(),
                "test".to_string(),
            ),
            widget: Box::new(Empty_widget),
            focusable: false,
            parent: None,
            children: Vec::new(),
            slot_manager: Slot_records::new(problem),
            logical: false,
        })))
    }

    #[tokio::test]
    async fn child_layers_are_inherited_by_their_subtrees() -> Result<()> {
        let variables = Arc::new(Variables::new());
        let problem = Arc::new(Mutex::new(Problem::new(Arc::clone(&variables))));
        let context = Component_context::new(problem);

        let root = component("root", &variables, context.clone());
        let mut layer_two = component("layer-two", &variables, context.clone());
        layer_two.layer = 2;
        let layer_two_child = component("layer-two-child", &variables, context.clone());
        layer_two.lock().await?.children = vec![layer_two_child];

        let mut layer_one = component("layer-one", &variables, context);
        layer_one.layer = 1;
        root.lock().await?.children = vec![layer_two, layer_one];

        let components = root.layered_components().await?;
        assert_eq!(
            components
                .iter()
                .map(|component| component.layer)
                .collect::<Vec<_>>(),
            vec![0, 2, 2, 1]
        );

        let mut paint_order = components;
        paint_order.sort_by_key(|component| (component.layer, component.tree_order));
        let mut names = Vec::new();
        for component in paint_order {
            names.push(component.component.lock().await?.name.clone());
        }
        assert_eq!(
            names,
            vec!["root", "layer-one", "layer-two", "layer-two-child"]
        );

        Ok(())
    }

    #[tokio::test]
    async fn logical_components_stop_component_traversal_at_their_children() -> Result<()> {
        let variables = Arc::new(Variables::new());
        let problem = Arc::new(Mutex::new(Problem::new(Arc::clone(&variables))));
        let context = Component_context::new(problem);

        let root = component("root", &variables, context.clone());
        let logical_child = component("logical", &variables, context.clone());
        logical_child.lock().await?.logical = true;
        let grandchild = component("grandchild", &variables, context);
        logical_child.lock().await?.children = vec![grandchild.clone()];
        root.lock().await?.children = vec![logical_child.clone()];

        let components = root.layered_components().await?;
        assert_eq!(components.len(), 2);
        assert!(components[1].component.compare(&logical_child));

        logical_child.lock().await?.logical = false;
        let components = root.layered_components().await?;
        assert_eq!(components.len(), 3);
        assert!(components[2].component.compare(&grandchild));

        Ok(())
    }

    #[tokio::test]
    async fn focused_path_contains_the_target_and_its_parents() -> Result<()> {
        let variables = Arc::new(Variables::new());
        let problem = Arc::new(Mutex::new(Problem::new(Arc::clone(&variables))));
        let context = Component_context::new(problem);
        let parent = component("parent", &variables, context.clone());
        let child = component("child", &variables, context.clone());
        let unrelated = component("unrelated", &variables, context);
        child.lock().await?.parent = Some(parent.as_reference());
        parent.lock().await?.children = vec![child.clone()];

        let mut focus = Focus::new();
        focus.set(&child);
        let focused_path = focus.focused_path().await?;

        assert!(focused_path.contains(&child));
        assert!(focused_path.contains(&parent));
        assert!(!focused_path.contains(&unrelated));
        Ok(())
    }
}
