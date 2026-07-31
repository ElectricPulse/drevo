pub mod context;

use async_recursion::async_recursion;
use color_eyre::eyre::Result;
use derive_new::new;
use std::sync::{Arc, Weak};

use crate::{
    focus::Focus,
    geometry::Direction,
    layouter::{
        Solution, constraints::shrink_wrap, expression::Expression, hitbox::Hitbox,
        variables::Variables,
    },
    slot::manager::{Slot_records, Slots},
    sync::{Mutex, MutexGuard},
    text::Text_context,
    widget::{Control, Focus_provider, Widget, Widget_trait},
};

use self::context::Component_context;

pub type Id = u64;

pub type Child = Shared_component;

pub type Children = Vec<Child>;

pub type Parent = Option<Child_reference>;

pub struct Component {
    pub name: String,
    pub hitbox: Hitbox,
    pub widget: Widget,
    pub focusable: bool,
    pub parent: Parent,
    pub children: Children,
    pub slot_manager: Slot_records,
    pub(crate) variables: Arc<Variables>,
}

#[derive(Clone, new)]
pub struct Shared_component(Arc<Mutex<Component>>);

impl Control for Shared_component {}

#[async_trait::async_trait]
impl Widget_trait for Shared_component {
    async fn layout(
        &mut self,
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

    pub async fn share_start(
        &self,
        parent: Hitbox,
        problem: &Component_context,
        direction: Direction,
    ) -> Result<()> {
        self.lock()
            .await?
            .hitbox
            .share_start(parent, problem, direction)
            .await
    }

    pub async fn share_end(
        &self,
        parent: Hitbox,
        problem: &Component_context,
        direction: Direction,
    ) -> Result<()> {
        self.lock()
            .await?
            .hitbox
            .share_end(parent, problem, direction)
            .await
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
            .share_dimension(parent, problem, direction)
            .await
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

    pub fn compare_with_reference(&self, node: &Child_reference) -> bool {
        let Some(node) = node.upgrade() else {
            return false;
        };

        self.compare(&Shared_component(node))
    }

    pub async fn layout(
        &mut self,
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
                let mut slots = slot_manager.slots();
                let children = widget
                    .layout(
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
        children: Children,
        parent_hitbox: Hitbox,
        mut problem: Component_context,
        text_context: &mut Text_context,
    ) -> Result<()> {
        let hitbox = {
            let component = self.lock().await?;
            problem.component_path.push(component.name.clone());
            component.hitbox
        };

        for child in &children {
            let mut child = child.clone();
            let grandchildren = child
                .clone()
                .layout(self.clone().into(), hitbox, problem.clone(), text_context)
                .await?;
            child
                .layout_children(grandchildren, hitbox, problem.clone(), text_context)
                .await?;
        }

        for direction in [Direction::Horizontal, Direction::Vertical] {
            shrink_wrap(&problem, hitbox, parent_hitbox, &children, direction).await?;
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
