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
mod tests;
