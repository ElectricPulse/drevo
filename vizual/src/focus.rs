use std::{collections::HashSet, sync::Weak};

use color_eyre::eyre::Result;

use crate::{
    Signal,
    component::{ChildReference, SharedComponent},
    state::{ReadGuard, Store},
};

#[derive(Default)]
pub(crate) struct FocusedPath(HashSet<usize>);

impl FocusedPath {
    pub(crate) fn contains(&self, component: &SharedComponent) -> bool {
        self.0.contains(&component.identity())
    }
}

/// The component that owns input focus.
///
/// Focus is state: layouts subscribe through [`Self::affect`], so moving or clearing focus
/// invalidates the layout that decided which focused widgets exist.
#[derive(Clone)]
pub struct Focus(Store<ChildReference>);

#[derive(Clone, Copy)]
pub enum FocusSearchDirection {
    Left,
    Right,
}

impl Focus {
    pub fn new() -> Self {
        Self(Store::new(Weak::new()))
    }

    pub async fn read(&self) -> Result<ReadGuard<ChildReference>> {
        self.0.read().await
    }

    /// Reads focus and subscribes `signal` to subsequent focus changes.
    pub async fn affect(&self, signal: Signal) -> Result<ReadGuard<ChildReference>> {
        self.0.affect(signal).await
    }

    pub async fn upgrade(&self) -> Result<Option<SharedComponent>> {
        Ok(self.read().await?.upgrade().map(SharedComponent::new))
    }

    pub async fn compare(&self, node: &SharedComponent) -> Result<bool> {
        if let Some(this) = self.upgrade().await? {
            return Ok(this.compare(node));
        }

        Ok(false)
    }

    /// Collects both the exact focus target and each of its ancestors as focused.
    ///
    /// This is intentional: input events bubble through this same parent chain, so every
    /// component that receives an event as part of the focused subtree also receives focused
    /// state. The path is collected once per layout or render pass so components only need an
    /// O(1) membership check.
    pub(crate) async fn focused_path(&self) -> Result<FocusedPath> {
        let focus = self.read().await?;
        Self::focused_path_from(&focus).await
    }

    /// Reads focus while subscribing `signal`, then collects its focused component path.
    pub(crate) async fn affected_path(&self, signal: Signal) -> Result<FocusedPath> {
        let focus = self.affect(signal).await?;
        Self::focused_path_from(&focus).await
    }

    async fn focused_path_from(focus: &ChildReference) -> Result<FocusedPath> {
        let Some(mut focused) = focus.upgrade().map(SharedComponent::new) else {
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

    pub async fn reset(&self) -> Result<()> {
        self.0.set(Weak::new()).await
    }

    pub async fn set_with_reference(&self, focus: &ChildReference) -> Result<()> {
        self.0.set(focus.clone()).await
    }

    pub async fn set(&self, focus: &SharedComponent) -> Result<()> {
        self.set_with_reference(&focus.as_reference()).await
    }
}

impl Default for Focus {
    fn default() -> Self {
        Self::new()
    }
}
