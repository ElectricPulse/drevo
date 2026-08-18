pub mod manager;

use std::{
    panic::Location,
    sync::{
        Arc, Weak,
        atomic::{AtomicU64, Ordering},
    },
};

use color_eyre::eyre::Result;

use crate::{
    component::{
        Child_reference, Component, Shared_component, context::Component_context,
        debug::Component_debug,
    },
    layouter::hitbox::Hitbox,
    sync::Mutex,
    widget::Widget_trait,
};

use self::manager::Slot_records;

static NEXT_COMPONENT_NAME: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub struct Component_slot {
    reference: Child_reference,
    name: String,
    path: String,
}

impl Component_slot {
    #[track_caller]
    pub fn new() -> Self {
        Self::new_at(Location::caller())
    }

    pub(crate) fn new_at(location: &'static Location<'static>) -> Self {
        Self {
            reference: Weak::new(),
            name: format!("c{}", NEXT_COMPONENT_NAME.fetch_add(1, Ordering::Relaxed)),
            path: format!("{}:{}", location.file(), location.line()),
        }
    }

    pub fn get_reference(&self) -> Child_reference {
        self.reference.clone()
    }

    pub(crate) async fn dismount(&mut self) -> Result<()> {
        if let Some(component) = self.reference.upgrade() {
            Shared_component::new(component).dismount().await?;
        }
        self.reference = Weak::new();
        Ok(())
    }

    pub async fn set(
        &mut self,
        widget: impl Widget_trait,
        problem: Component_context,
    ) -> Result<Shared_component> {
        self.set_with_parent(widget, problem, None).await
    }

    pub async fn set_child(
        &mut self,
        widget: impl Widget_trait,
        problem: Component_context,
        parent: &Hitbox,
    ) -> Result<Shared_component> {
        self.set_with_parent(widget, problem, Some(parent)).await
    }

    async fn set_with_parent(
        &mut self,
        widget: impl Widget_trait,
        mut problem: Component_context,
        parent: Option<&Hitbox>,
    ) -> Result<Shared_component> {
        let widget = widget.as_any();

        problem.component_path.push(self.name.clone());
        let component_path = problem.component_path.join(".");
        let variables = problem.lock().await?.variables();

        let lock = if let Some(lock) = self.reference.upgrade() {
            let mut reference = lock.lock().await?;
            reference.name = self.name.clone();
            reference.debug.source_path = self.path.clone();
            reference.widget = widget;
            reference.slot_manager.set_problem(problem);
            reference.hitbox.reset_shared();
            reference.logical = false;
            reference.mask = false;
            if parent.is_none() {
                reference.hitbox.make_independent();
            }

            Shared_component::new(lock.clone())
        } else {
            let mut hitbox = Hitbox::new(
                &variables,
                self.name.clone(),
                component_path,
                self.path.clone(),
            );
            if parent.is_none() {
                hitbox.make_independent();
            }

            let lock = Shared_component::new(Arc::new(Mutex::new(Component {
                name: self.name.clone(),
                debug: Component_debug::new(self.path.clone()),
                hitbox,
                widget,
                focusable: false,
                children: Vec::new(),
                parent: None,
                slot_manager: Slot_records::new(problem),
                logical: false,
                mask: false,
            })));

            self.reference = lock.as_reference();
            lock
        };

        Ok(lock)
    }
}

impl Default for Component_slot {
    fn default() -> Self {
        Self::new()
    }
}
