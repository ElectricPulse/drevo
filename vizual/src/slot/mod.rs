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
        ChildReference, Component, SharedComponent, context::ComponentContext,
        debug::ComponentDebug,
    },
    layouter::hitbox::Hitbox,
    sync::Mutex,
    widget::WidgetTrait,
};

use self::manager::SlotRecords;

static NEXT_COMPONENT_NAME: AtomicU64 = AtomicU64::new(1);
static NEXT_COMPONENT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub struct ComponentSlot {
    reference: ChildReference,
    name: String,
    path: String,
}

impl ComponentSlot {
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

    pub fn get_reference(&self) -> ChildReference {
        self.reference.clone()
    }

    pub(crate) async fn dismount(&mut self) -> Result<()> {
        if let Some(component) = self.reference.upgrade() {
            SharedComponent::new(component).dismount().await?;
        }
        self.reference = Weak::new();
        Ok(())
    }

    pub async fn set(
        &mut self,
        widget: impl WidgetTrait,
        problem: ComponentContext,
    ) -> Result<SharedComponent> {
        self.set_with_parent(widget, problem, None).await
    }

    pub async fn set_child(
        &mut self,
        widget: impl WidgetTrait,
        problem: ComponentContext,
        parent: &Hitbox,
    ) -> Result<SharedComponent> {
        self.set_with_parent(widget, problem, Some(parent)).await
    }

    async fn set_with_parent(
        &mut self,
        widget: impl WidgetTrait,
        mut problem: ComponentContext,
        parent: Option<&Hitbox>,
    ) -> Result<SharedComponent> {
        let widget = widget.as_any();

        problem.component_path.push(self.name.clone());
        let component_path = problem.component_path.join(".");
        let variables = problem.lock().await?.registry();

        let lock = if let Some(lock) = self.reference.upgrade() {
            let mut reference = lock.lock().await?;
            reference.name = self.name.clone();
            reference.debug.source_path = self.path.clone();
            reference.widget = widget;
            reference.slot_manager.set_problem(problem);
            reference.formula = None;
            reference.hitbox.reset_shared();
            reference.logical = false;
            reference.mask = false;
            if parent.is_none() {
                reference.hitbox.make_independent();
            }

            SharedComponent::new(lock.clone())
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

            let lock = SharedComponent::new(Arc::new(Mutex::new(Component {
                id: NEXT_COMPONENT_ID.fetch_add(1, Ordering::Relaxed),
                name: self.name.clone(),
                debug: ComponentDebug::new(self.path.clone()),
                hitbox,
                formula: None,
                variables,
                widget,
                focusable: false,
                children: Vec::new(),
                layout_children: Vec::new(),
                parent: None,
                slot_manager: SlotRecords::new(problem),
                logical: false,
                mask: false,
            })));

            self.reference = lock.as_reference();
            lock
        };

        Ok(lock)
    }
}

impl Default for ComponentSlot {
    fn default() -> Self {
        Self::new()
    }
}
