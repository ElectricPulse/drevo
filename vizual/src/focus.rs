use std::sync::Weak;

use crate::component::{Child_reference, Shared_component};

#[derive(Clone)]
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

impl Default for Focus {
    fn default() -> Self {
        Self::new()
    }
}
