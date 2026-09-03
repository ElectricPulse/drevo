pub mod memoization;
mod store;

use std::{ops::Deref, sync::Arc};

use async_trait::async_trait;
use color_eyre::eyre::Result;
use dyn_clone::DynClone;

use crate::{Signal, sync::ThreadSafe};

pub use store::Store;

/// A value read from a [`StateTrait`].
pub struct ReadGuard<Value> {
    inner: Arc<Value>,
}

impl<Value> ReadGuard<Value> {
    pub fn new(value: Arc<Value>) -> Self {
        Self { inner: value }
    }
}

impl<Value> Deref for ReadGuard<Value> {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[async_trait]
pub trait StateTrait: ThreadSafe + DynClone {
    type Output: ThreadSafe;

    /// Reads the current value without subscribing a renderer.
    async fn read(&self) -> Result<ReadGuard<Self::Output>>;

    /// Reads the current value and subscribes the supplied renderer to later writes.
    async fn affect(&self, signal: Signal) -> Result<ReadGuard<Self::Output>>;
}

dyn_clone::clone_trait_object!(<Output> StateTrait<Output = Output> where Output: ThreadSafe);

pub type State<Output> = Box<dyn StateTrait<Output = Output>>;

pub struct Constant<Value>(Arc<Value>);

impl<Value> Constant<Value> {
    pub fn new(value: Value) -> Self {
        Self(Arc::new(value))
    }
}

impl<Value> Clone for Constant<Value> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<Value> From<Value> for Constant<Value> {
    fn from(value: Value) -> Self {
        Self(Arc::new(value))
    }
}

#[async_trait]
impl<Value: ThreadSafe> StateTrait for Constant<Value> {
    type Output = Value;

    async fn read(&self) -> Result<ReadGuard<Self::Output>> {
        Ok(ReadGuard::new(self.0.clone()))
    }

    async fn affect(&self, _signal: Signal) -> Result<ReadGuard<Self::Output>> {
        self.read().await
    }
}

impl<Value: ThreadSafe> From<Value> for State<Value> {
    fn from(value: Value) -> Self {
        Box::new(Constant::from(value))
    }
}

impl<Value: ThreadSafe> From<Constant<Value>> for State<Value> {
    fn from(value: Constant<Value>) -> Self {
        Box::new(value)
    }
}

impl<Value: ThreadSafe> From<Store<Value>> for State<Value> {
    fn from(value: Store<Value>) -> Self {
        Box::new(value)
    }
}

impl From<&str> for State<String> {
    fn from(value: &str) -> Self {
        Box::new(Constant::from(value.to_owned()))
    }
}

impl From<&String> for State<String> {
    fn from(value: &String) -> Self {
        Box::new(Constant::from(value.clone()))
    }
}

#[cfg(test)]
mod tests;
