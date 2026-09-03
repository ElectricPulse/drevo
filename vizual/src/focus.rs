use std::{collections::HashSet, sync::Weak};

use color_eyre::eyre::Result;

use crate::component::{ChildReference, SharedComponent};

#[derive(Default)]
pub(crate) struct FocusedPath(HashSet<usize>);

impl FocusedPath {
    pub(crate) fn contains(&self, component: &SharedComponent) -> bool {
        self.0.contains(&component.identity())
    }
}

#[derive(Clone, Default)]
pub struct Focus(pub ChildReference);

#[derive(Clone, Copy)]
pub enum FocusSearchDirection {
    Left,
    Right,
}

impl Focus {
    pub fn new() -> Self {
        Self(Weak::new())
    }

    pub fn upgrade(&self) -> Option<SharedComponent> {
        self.0.upgrade().map(SharedComponent::new)
    }

    pub fn compare(&self, node: &SharedComponent) -> bool {
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
    pub(crate) async fn focused_path(&self) -> Result<FocusedPath> {
        let Some(mut focused) = self.upgrade() else {
            return Ok(FocusedPath::default());
        };
        let mut path = HashSet::new();

        loop {
            let _ = path.insert(focused.identity());

            let parent = focused.lock().await?.parent.clone();
            let Some(parent) = parent.and_then(|parent| parent.upgrade()) else {
                return Ok(FocusedPath(path));
            };
            focused = SharedComponent::new(parent);
        }
    }

    pub fn reset(&mut self) {
        self.0 = Weak::new();
    }

    pub fn set_with_reference(&mut self, focus: &ChildReference) {
        self.0 = focus.clone()
    }

    pub fn set(&mut self, focus: &SharedComponent) {
        self.set_with_reference(&focus.as_reference());
    }
}
