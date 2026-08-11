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
        let widget = Box::new(widget);

        problem.component_path.push(self.name.clone());
        let component_path = problem.component_path.join(".");
        let variables = problem.lock().await?.variables();
        let hitbox_problem = problem.clone();

        let lock = if let Some(lock) = self.reference.upgrade() {
            let mut reference = lock.lock().await?;
            reference.name = self.name.clone();
            reference.debug.source_path = self.path.clone();
            reference.widget = widget;
            reference.slot_manager.set_problem(problem);
            match parent {
                Some(parent) => reference.hitbox.point_to(parent),
                None => reference.hitbox.make_independent_at(
                    &variables,
                    &self.name,
                    &component_path,
                    &self.path,
                ),
            }

            Shared_component::new(lock.clone())
        } else {
            let hitbox = match parent {
                Some(parent) => Hitbox::shared(parent),
                None => Hitbox::new(
                    &variables,
                    self.name.clone(),
                    component_path,
                    self.path.clone(),
                ),
            };

            let lock = Shared_component::new(Arc::new(Mutex::new(Component {
                name: self.name.clone(),
                debug: Component_debug::new(self.path.clone()),
                hitbox,
                widget,
                focusable: false,
                children: Vec::new(),
                parent: None,
                slot_manager: Slot_records::new(problem),
            })));

            self.reference = lock.as_reference();
            lock
        };

        // TODO: This blanket hitbox ordering constraint is temporary; widgets should eventually
        // guarantee valid dimensions through their own explicit layout relationships.
        hitbox_problem
            .constrain_hitbox(lock.get_hitbox().await?)
            .await?;

        Ok(lock)
    }
}

impl Default for Component_slot {
    fn default() -> Self {
        Self::new()
    }
}
