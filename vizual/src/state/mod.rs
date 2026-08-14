mod store;

use std::ops::Deref;

use async_trait::async_trait;
use color_eyre::eyre::Result;
use dyn_clone::DynClone;

use crate::{
    Render,
    sync::{MutexGuard, Thread_safe},
};

pub use store::{Store, Write_guard};

/// A value read from a [`State`].
///
/// Store-backed values keep the store locked for the lifetime of this guard. Constant-backed
/// values are borrowed directly from the constant.
pub struct Read_guard<'a, Value> {
    inner: Read_guard_inner<'a, Value>,
}

enum Read_guard_inner<'a, Value> {
    Store(MutexGuard<'a, store::Store_content<Value>>),
    Constant(&'a Value),
}

impl<'a, Value> Read_guard<'a, Value> {
    pub(in crate::state) fn store(guard: MutexGuard<'a, store::Store_content<Value>>) -> Self {
        Self {
            inner: Read_guard_inner::Store(guard),
        }
    }

    fn constant(value: &'a Value) -> Self {
        Self {
            inner: Read_guard_inner::Constant(value),
        }
    }
}

impl<Value> Deref for Read_guard<'_, Value> {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        match &self.inner {
            Read_guard_inner::Store(guard) => &guard.value,
            Read_guard_inner::Constant(value) => value,
        }
    }
}

#[async_trait]
pub trait State: Thread_safe + DynClone {
    type Output: Thread_safe;

    /// Reads the current value without subscribing a renderer.
    async fn read(&self) -> Result<Read_guard<'_, Self::Output>>;

    /// Reads the current value and subscribes the supplied renderer to later writes.
    async fn affect(&self, signal: Render) -> Result<Read_guard<'_, Self::Output>>;
}

dyn_clone::clone_trait_object!(<Output> State<Output = Output> where Output: Thread_safe);

#[derive(Clone)]
pub struct Constant<Value>(Value);

impl<Value> From<Value> for Constant<Value> {
    fn from(value: Value) -> Self {
        Self(value)
    }
}

#[async_trait]
impl<Value> State for Constant<Value>
where
    Value: Thread_safe + Clone,
{
    type Output = Value;

    async fn read(&self) -> Result<Read_guard<'_, Self::Output>> {
        Ok(Read_guard::constant(&self.0))
    }

    async fn affect(&self, _signal: Render) -> Result<Read_guard<'_, Self::Output>> {
        self.read().await
    }
}

impl<Value> From<Value> for Box<dyn State<Output = Value>>
where
    Value: Thread_safe + Clone,
{
    fn from(value: Value) -> Self {
        Box::new(Constant::from(value))
    }
}

impl<Value> From<Constant<Value>> for Box<dyn State<Output = Value>>
where
    Value: Thread_safe + Clone,
{
    fn from(value: Constant<Value>) -> Self {
        Box::new(value)
    }
}

impl<Value> From<Store<Value>> for Box<dyn State<Output = Value>>
where
    Value: Thread_safe,
{
    fn from(value: Store<Value>) -> Self {
        Box::new(value.clone())
    }
}

impl From<&str> for Box<dyn State<Output = String>> {
    fn from(value: &str) -> Self {
        Box::new(Constant::from(value.to_owned()))
    }
}

impl From<&String> for Box<dyn State<Output = String>> {
    fn from(value: &String) -> Self {
        Box::new(Constant::from(value.clone()))
    }
}

// TODO: A global render signal is only a temporary invalidation mechanism. Eventually each
// component, or even each layout call, should receive its own signal so a state write can
// invalidate only the affected part of the component tree.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render_manager::Render_manager;

    struct Not_clone(u8);

    #[tokio::test]
    async fn store_clone_only_clones_the_arc() -> Result<()> {
        let store = Store::new(Not_clone(1));
        let cloned = store.clone();

        cloned.write().await?.0 = 2;

        assert_eq!(store.read().await?.0, 2);
        Ok(())
    }

    #[tokio::test]
    async fn affect_deduplicates_render_ids_and_write_notifies_after_drop() -> Result<()> {
        let mut first_manager = Render_manager::new();
        let mut second_manager = Render_manager::new();
        assert_ne!(first_manager.render.id, second_manager.render.id);
        assert_eq!(first_manager.render.id, first_manager.render.clone().id);

        let store = Store::new(1_u8);
        drop(store.read().await?);
        assert!(first_manager.reciever.0.try_recv().is_err());

        drop(store.affect(first_manager.render.clone()).await?);
        drop(store.affect(first_manager.render.clone()).await?);
        drop(store.affect(second_manager.render.clone()).await?);

        let mut value = store.write().await?;
        *value = 2;
        assert!(first_manager.reciever.0.try_recv().is_err());
        assert!(second_manager.reciever.0.try_recv().is_err());
        drop(value);

        assert_eq!(first_manager.reciever.0.recv().await, Some(()));
        assert_eq!(second_manager.reciever.0.recv().await, Some(()));
        assert!(first_manager.reciever.0.try_recv().is_err());
        assert!(second_manager.reciever.0.try_recv().is_err());
        assert_eq!(*store.read().await?, 2);
        Ok(())
    }

    #[tokio::test]
    async fn constant_never_subscribes() -> Result<()> {
        let mut manager = Render_manager::new();
        let constant = Constant::from(String::from("constant"));

        assert_eq!(&*constant.read().await?, "constant");
        assert_eq!(&*constant.affect(manager.render.clone()).await?, "constant");
        assert!(manager.reciever.0.try_recv().is_err());
        Ok(())
    }
}
