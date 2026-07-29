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
    component::{Child_reference, Component, Shared_component, context::Component_context},
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

    pub async fn set(
        &mut self,
        widget: impl Widget_trait,
        mut problem: Component_context,
    ) -> Result<Shared_component> {
        let widget = Box::new(widget);

        problem.component_path.push(self.name.clone());
        let component_path = problem.component_path.join(".");
        /*let hitbox = {
            let mut problem = problem.lock().await?;
            // It is assumed that between two distinct calls of set on one component slot the
            // problem is going to change, in which case the variables inside Hitbox need to be
            // recreated.
            Hitbox::new(
                &mut problem,
                self.name.clone(),
                component_path,
                self.path.clone(),
            )
        };*/

        let lock = if let Some(lock) = self.reference.upgrade() {
            let mut reference = lock.lock().await?;
            reference.name = self.name.clone();
            reference.widget = widget;
            //reference.hitbox = hitbox;
            reference.slot_manager.set_problem(problem);

            Shared_component::new(lock.clone())
        } else {
            let hitbox = {
                let mut problem = problem.lock().await?;
                // It is assumed that between two distinct calls of set on one component slot the
                // problem is going to change, in which case the variables inside Hitbox need to be
                // recreated.
                Hitbox::new(
                    &mut problem,
                    self.name.clone(),
                    component_path,
                    self.path.clone(),
                )
            };

            let lock = Shared_component::new(Arc::new(Mutex::new(Component {
                name: self.name.clone(),
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

        Ok(lock)
    }
}

impl Default for Component_slot {
    fn default() -> Self {
        Self::new()
    }
}
