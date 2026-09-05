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

// `display!` uses `slots.set` to return children from `WidgetTrait::layout`. This raises the
// question of why the trait does not return a `HashMap<Id, Box<dyn WidgetTrait>>` directly.
// Slots let widgets retain control of the resulting `Component` instances when that is needed.
// Portals, such as the dialog in the default-root settings header, use that control to mount a
// layout child in a different graphical parent.
//
// A widget can store a `ComponentSlot` directly when it always renders the same child. That is
// marginally faster than calling `slots.set`, but the difference is too small to justify the
// extra state in most widgets. Prefer `slots.set` unless a widget needs direct component control.
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

        problem.push(|| self.name.clone());
        let component_path = problem.join();
        let variables = Arc::clone(&problem.variables);

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
                name: self.name.clone(),
                debug: ComponentDebug::new(self.path.clone()),
                hitbox,
                formula: crate::layouter::Formula::new(Arc::clone(&variables)),
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
