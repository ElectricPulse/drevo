mod store;

use std::{ops::Deref, sync::Arc};

use async_trait::async_trait;
use color_eyre::eyre::Result;
use dyn_clone::DynClone;

use crate::{Signal, sync::Thread_safe};

pub use store::Store;

/// A value read from a [`State_trait`].
pub struct Read_guard<Value> {
    inner: Arc<Value>,
}

impl<Value> Read_guard<Value> {
    pub fn new(value: Arc<Value>) -> Self {
        Self { inner: value }
    }
}

impl<Value> Deref for Read_guard<Value> {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[async_trait]
pub trait State_trait: Thread_safe + DynClone {
    type Output: Thread_safe;

    /// Reads the current value without subscribing a renderer.
    async fn read(&self) -> Result<Read_guard<Self::Output>>;

    /// Reads the current value and subscribes the supplied renderer to later writes.
    async fn affect(&self, signal: Signal) -> Result<Read_guard<Self::Output>>;
}

dyn_clone::clone_trait_object!(<Output> State_trait<Output = Output> where Output: Thread_safe);

pub type State<Output> = Box<dyn State_trait<Output = Output>>;

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
impl<Value: Thread_safe> State_trait for Constant<Value> {
    type Output = Value;

    async fn read(&self) -> Result<Read_guard<Self::Output>> {
        Ok(Read_guard::new(self.0.clone()))
    }

    async fn affect(&self, _signal: Signal) -> Result<Read_guard<Self::Output>> {
        self.read().await
    }
}

impl<Value: Thread_safe> From<Value> for State<Value> {
    fn from(value: Value) -> Self {
        Box::new(Constant::from(value))
    }
}

impl<Value: Thread_safe> From<Constant<Value>> for State<Value> {
    fn from(value: Constant<Value>) -> Self {
        Box::new(value)
    }
}

impl<Value: Thread_safe> From<Store<Value>> for State<Value> {
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
