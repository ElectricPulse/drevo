use std::{future::Future, pin::Pin, sync::Arc};

use async_trait::async_trait;
use color_eyre::Result;

use super::{Read_guard, State_trait, Store};
use crate::{
    Signal,
    sync::{Mutex, Thread_safe},
};

type Callback_future<Value> = Pin<Box<dyn Future<Output = Result<Value>> + Send>>;
type Callback<Value> = dyn Fn() -> Callback_future<Value> + Send + Sync;

#[async_trait]
pub trait Memoization_store: Thread_safe {
    async fn version(&self) -> Result<u64>;
    async fn affect(&self, signal: Signal) -> Result<()>;
}

#[async_trait]
impl<Value: Thread_safe> Memoization_store for Store<Value> {
    async fn version(&self) -> Result<u64> {
        self.version().await
    }

    async fn affect(&self, signal: Signal) -> Result<()> {
        let _ = self.affect(signal).await?;
        Ok(())
    }
}

pub type Dependency = Arc<dyn Memoization_store>;

pub fn dependency<Value: Thread_safe>(store: Store<Value>) -> Dependency {
    Arc::new(store)
}

struct Cached<Value> {
    versions: Vec<u64>,
    value: Value,
}

/// A cached state derived from stores.
///
/// Calling `affect` subscribes the supplied signal to every dependency. A dependency update then
/// signals the consumer; its next read reruns the callback only when a source version changed.
pub struct Memoization<Value: Thread_safe + Clone> {
    callback: Arc<Callback<Value>>,
    stores: Vec<Dependency>,
    cached: Arc<Mutex<Option<Cached<Value>>>>,
}

impl<Value: Thread_safe + Clone> Clone for Memoization<Value> {
    fn clone(&self) -> Self {
        Self {
            callback: Arc::clone(&self.callback),
            stores: self.stores.clone(),
            cached: Arc::clone(&self.cached),
        }
    }
}

/// Creates a memoized state from `stores`.
pub fn memoization<Value, Callback, Callback_future>(
    callback: Callback,
    stores: Vec<Dependency>,
) -> Memoization<Value>
where
    Value: Thread_safe + Clone,
    Callback: Fn() -> Callback_future + Send + Sync + 'static,
    Callback_future: Future<Output = Result<Value>> + Send + 'static,
{
    Memoization {
        callback: Arc::new(move || Box::pin(callback())),
        stores,
        cached: Arc::new(Mutex::new(None)),
    }
}

impl<Value: Thread_safe + Clone> Memoization<Value> {
    async fn value(&self, signal: Option<Signal>) -> Result<Read_guard<Value>> {
        let mut versions = Vec::with_capacity(self.stores.len());
        for store in &self.stores {
            versions.push(store.version().await?);
            if let Some(signal) = &signal {
                store.affect(signal.clone()).await?;
            }
        }

        let mut cached = self.cached.lock().await?;
        match cached.as_ref() {
            Some(cached) if cached.versions == versions => {
                Ok(Read_guard::new(Arc::new(cached.value.clone())))
            }
            _ => {
                let value = (self.callback)().await?;
                *cached = Some(Cached {
                    versions,
                    value: value.clone(),
                });
                Ok(Read_guard::new(Arc::new(value)))
            }
        }
    }
}

#[async_trait]
impl<Value: Thread_safe + Clone> State_trait for Memoization<Value> {
    type Output = Value;

    async fn read(&self) -> Result<Read_guard<Self::Output>> {
        self.value(None).await
    }

    async fn affect(&self, signal: Signal) -> Result<Read_guard<Self::Output>> {
        self.value(Some(signal)).await
    }
}
