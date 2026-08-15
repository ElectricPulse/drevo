use std::{collections::HashSet, sync::Weak};

use color_eyre::eyre::Result;

use crate::component::{Child_reference, Shared_component};

#[derive(Default)]
pub(crate) struct Focused_path(HashSet<usize>);

impl Focused_path {
    pub(crate) fn contains(&self, component: &Shared_component) -> bool {
        self.0.contains(&component.identity())
    }
}

#[derive(Clone, Default)]
pub struct Focus(pub Child_reference);

#[derive(Clone, Copy)]
pub enum Focus_search_direction {
    Left,
    Right,
}

impl Focus {
    pub fn new() -> Self {
        Self(Weak::new())
    }

    pub fn upgrade(&self) -> Option<Shared_component> {
        self.0.upgrade().map(Shared_component::new)
    }

    pub fn compare(&self, node: &Shared_component) -> bool {
        if let Some(this) = self.upgrade() {
            return this.compare(node);
        }

        false
    }

    /// Collects both the exact focus target and each of its ancestors as focused.
    ///
    /// This is intentional: input events bubble through this same parent chain, so every
    /// component that receives an event as part of the focused subtree also receives focused
    /// state. The path is collected once per layout or render pass so components only need an
    /// O(1) membership check.
    pub(crate) async fn focused_path(&self) -> Result<Focused_path> {
        let Some(mut focused) = self.upgrade() else {
            return Ok(Focused_path::default());
        };
        let mut path = HashSet::new();

        loop {
            let _ = path.insert(focused.identity());

            let parent = focused.lock().await?.parent.clone();
            let Some(parent) = parent.and_then(|parent| parent.upgrade()) else {
                return Ok(Focused_path(path));
            };
            focused = Shared_component::new(parent);
        }
    }

    pub fn reset(&mut self) {
        self.0 = Weak::new();
    }

    pub fn set_with_reference(&mut self, focus: &Child_reference) {
        self.0 = focus.clone()
    }

    pub fn set(&mut self, focus: &Shared_component) {
        self.set_with_reference(&focus.as_reference());
    }
}
