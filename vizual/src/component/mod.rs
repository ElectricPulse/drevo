pub mod context;
pub(crate) mod debug;

use async_recursion::async_recursion;
use color_eyre::eyre::Result;
use std::sync::{Arc, Weak};

use crate::{
    Render,
    focus::Focus,
    geometry::Direction,
    layouter::{Solution, hitbox::Hitbox},
    slot::manager::{Slot_records, Slots},
    state::State,
    sync::{Mutex, MutexGuard},
    text::Text_context,
    theme::Theme,
    widget::{Focus_provider, Widget, Widget_trait},
};

use self::{context::Component_context, debug::Component_debug};

pub type Id = u64;

pub type Child = Shared_component;

pub type Children = Vec<Child>;

pub type Parent = Option<Child_reference>;

pub struct Component {
    pub name: String,
    pub(crate) debug: Component_debug,
    pub hitbox: Hitbox,
    pub widget: Widget,
    pub focusable: bool,
    pub parent: Parent,
    pub children: Children,
    pub slot_manager: Slot_records,
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

#[async_trait::async_trait]
impl Widget_trait for Shared_component {
    async fn layout(
        &mut self,
        _render: Render,
        _theme: State<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut Text_context,
        _slots: &mut Slots,
    ) -> Result<Children> {
        Ok(vec![self.clone()])
    }
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

    pub fn as_reference(&self) -> Child_reference {
        Arc::downgrade(&self.component)
    }

    pub async fn get_hitbox(&self) -> Result<Hitbox> {
        Ok(self.lock().await?.hitbox.clone())
    }

    /// Maximizes this component in both directions at priority 1.
    /// Uses priority-based expansion and may leave room for higher-priority layout objectives.
    pub async fn fill(self) -> Result<Self> {
        let (hitbox, problem) = {
            let component = self.lock().await?;
            (
                component.hitbox.clone(),
                component.slot_manager.problem.clone(),
            )
        };

        for direction in [Direction::Horizontal, Direction::Vertical] {
            problem
                .lock()
                .await?
                .maximize(hitbox.get_dimension(direction), 1)?;
        }

        Ok(self)
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

    pub fn compare_with_reference(&self, node: &Child_reference) -> bool {
        let Some(node) = node.upgrade() else {
            return false;
        };

        self.compare(&Shared_component::new(node))
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
        let children = self.lock().await?.children.clone();
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

    pub async fn layout(
        &mut self,
        render: Render,
        theme: State<Theme>,
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

            let mut focus = Focus_provider::new(false);

            let children = {
                let mut slots = slot_manager.slots(hitbox);
                let children = widget
                    .layout(
                        render,
                        theme,
                        &mut focus,
                        hitbox,
                        parent,
                        problem.clone(),
                        text_context,
                        &mut slots,
                    )
                    .await?;

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
    pub async fn layout_children(
        &mut self,
        render: Render,
        theme: State<Theme>,
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
            let grandchildren = child
                .clone()
                .layout(
                    render.clone(),
                    theme.clone(),
                    self.clone().into(),
                    hitbox.clone(),
                    problem.clone(),
                    text_context,
                )
                .await?;
            child
                .layout_children(
                    render.clone(),
                    theme.clone(),
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
        theme: State<Theme>,
        focus: Focus,
        display: &mut crate::display::Display<'_>,
        solution: &Solution,
    ) -> Result<()> {
        let mut components = self.layered_components().await?;
        components.sort_by_key(|component| (component.layer, component.tree_order));

        for mut component in components {
            component
                .component
                .render_component(theme.clone(), focus.clone(), display, solution)
                .await?;
        }

        Ok(())
    }

    async fn render_component(
        &mut self,
        theme: State<Theme>,
        focus: Focus,
        display: &mut crate::display::Display<'_>,
        solution: &Solution,
    ) -> Result<()> {
        let mut this = self.lock().await?;
        let hitbox = this.hitbox.get_resolved(solution);
        let focused = focus.compare(self);
        let mut focus = Focus_provider::new(focused);
        let maybe_hitbox = this
            .widget
            .render(theme, &mut focus, hitbox, display)
            .await?;
        this.focusable = focus.is_active();

        if let Some(hitbox) = maybe_hitbox {
            this.hitbox = hitbox;
        };

        Ok(())
    }
}

pub type Child_reference = Weak<Mutex<Component>>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        layouter::{Problem, variables::Variables},
        widget::Widget_trait,
    };

    #[derive(Clone)]
    struct Empty_widget;

    #[async_trait::async_trait]
    impl Widget_trait for Empty_widget {}

    fn component(name: &str, problem: Component_context) -> Shared_component {
        Shared_component::new(Arc::new(Mutex::new(Component {
            name: name.to_string(),
            debug: Component_debug::new("test".to_string()),
            hitbox: Hitbox::default(),
            widget: Box::new(Empty_widget),
            focusable: false,
            parent: None,
            children: Vec::new(),
            slot_manager: Slot_records::new(problem),
        })))
    }

    #[tokio::test]
    async fn child_layers_are_inherited_by_their_subtrees() -> Result<()> {
        let variables = Arc::new(Variables::new());
        let problem = Arc::new(Mutex::new(Problem::new(variables)));
        let context = Component_context::new(problem);

        let root = component("root", context.clone());
        let mut layer_two = component("layer-two", context.clone());
        layer_two.layer = 2;
        let layer_two_child = component("layer-two-child", context.clone());
        layer_two.lock().await?.children = vec![layer_two_child];

        let mut layer_one = component("layer-one", context);
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
}
