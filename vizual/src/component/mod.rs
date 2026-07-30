pub mod context;

use async_recursion::async_recursion;
use color_eyre::eyre::Result;
use derive_new::new;
use std::{
    panic::Location,
    sync::{
        Arc, Weak,
        atomic::{AtomicU64, Ordering},
    },
};

use crate::{
    focus::Focus,
    geometry::Direction,
    layouter::{Solution, expression::Expression, hitbox::Hitbox, variables::Variables},
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

pub type Child = Shared_component;

pub type Children = Vec<Child>;

pub type Parent = Option<Child_reference>;

static NEXT_UNMANAGED_COMPONENT_NAME: AtomicU64 = AtomicU64::new(1);

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

impl Component {
    #[track_caller]
    pub async fn new(widget: impl Widget_trait, mut problem: Component_context) -> Result<Self> {
        let location = Location::caller();
        let name = format!(
            "u{}",
            NEXT_UNMANAGED_COMPONENT_NAME.fetch_add(1, Ordering::Relaxed)
        );
        let path = format!("{}:{}", location.file(), location.line());
        problem.component_path.push(name.clone());
        let component_path = problem.component_path.join(".");
        let variables = problem.lock().await?.variables();
        let hitbox = Hitbox::new(&variables, name.clone(), component_path, path);

        Ok(Self {
            name,
            hitbox,
            widget: Box::new(widget),
            focusable: false,
            parent: None,
            children: Children::new(),
            slot_manager: Slot_records::new(problem),
            virtual_child: Component_slot::new(),
            variables,
        })
    }

    pub fn into_child(self) -> Child {
        Shared_component::new(Arc::new(Mutex::new(self)))
    }
}

#[derive(Clone, new)]
pub struct Shared_component(Arc<Mutex<Component>>);

impl Control for Shared_component {}

#[async_trait::async_trait]
impl Widget_trait for Shared_component {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: Hitbox,
        problem: Component_context,
        _text_context: &mut Text_context,
        _slots: &mut Slots,
    ) -> Result<Widget_type> {
        Widget_type::wrap(vec![self.clone()], hitbox, &problem, true, true).await
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
                    Widget_type::None => Vec::new(),
                    Widget_type::Virtual(widget) => {
                        let child = virtual_child
                            .set_init(widget, problem.clone(), Some(*hitbox))
                            .await?;
                        vec![child]
                    }
                    Widget_type::Visual { children } => children,
                }
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
