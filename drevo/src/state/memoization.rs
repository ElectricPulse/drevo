use std::{future::Future, pin::Pin, sync::Arc};

use async_trait::async_trait;
use color_eyre::Result;

use super::{ReadGuard, StateTrait, Store};
use crate::{
    Signal,
    sync::{Mutex, ThreadSafe},
};

type CallbackFuture<Value> = Pin<Box<dyn Future<Output = Result<Value>> + Send>>;
type Callback<Value> = dyn Fn() -> CallbackFuture<Value> + Send + Sync;

#[async_trait]
pub trait MemoizationStore: ThreadSafe {
    async fn version(&self) -> Result<u64>;
    async fn affect(&self, signal: Signal) -> Result<()>;
}

#[async_trait]
impl<Value: ThreadSafe> MemoizationStore for Store<Value> {
    async fn version(&self) -> Result<u64> {
        self.version().await
    }

    async fn affect(&self, signal: Signal) -> Result<()> {
        let _ = self.affect(signal).await?;
        Ok(())
    }
}

pub type Dependency = Arc<dyn MemoizationStore>;

pub fn dependency<Value: ThreadSafe>(store: Store<Value>) -> Dependency {
    Arc::new(store)
}

struct Cached<Value> {
    versions: Vec<u64>,
    value: Arc<Value>,
}

/// A cached state derived from stores.
///
/// Calling `affect` subscribes the supplied signal to every dependency. A dependency update then
/// signals the consumer; its next read reruns the callback only when a source version changed.
pub struct Memoization<Value: ThreadSafe> {
    callback: Arc<Callback<Value>>,
    stores: Vec<Dependency>,
    cached: Arc<Mutex<Option<Cached<Value>>>>,
}

impl<Value: ThreadSafe> Clone for Memoization<Value> {
    fn clone(&self) -> Self {
        Self {
            callback: Arc::clone(&self.callback),
            stores: self.stores.clone(),
            cached: Arc::clone(&self.cached),
        }
    }
}

/// Creates a memoized state from `stores`.
pub fn memoization<Value, Callback, CallbackFuture>(
    callback: Callback,
    stores: Vec<Dependency>,
) -> Memoization<Value>
where
    Value: ThreadSafe,
    Callback: Fn() -> CallbackFuture + Send + Sync + 'static,
    CallbackFuture: Future<Output = Result<Value>> + Send + 'static,
{
    Memoization {
        callback: Arc::new(move || Box::pin(callback())),
        stores,
        cached: Arc::new(Mutex::new(None)),
    }
}

impl<Value: ThreadSafe> Memoization<Value> {
    async fn value(&self, signal: Option<Signal>) -> Result<ReadGuard<Value>> {
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
                Ok(ReadGuard::new(Arc::clone(&cached.value)))
            }
            _ => {
                let value = Arc::new((self.callback)().await?);
                *cached = Some(Cached {
                    versions,
                    value: Arc::clone(&value),
                });
                Ok(ReadGuard::new(value))
            }
        }
    }
}

#[async_trait]
impl<Value: ThreadSafe> StateTrait for Memoization<Value> {
    type Output = Value;

    async fn read(&self) -> Result<ReadGuard<Self::Output>> {
        self.value(None).await
    }

    async fn affect(&self, signal: Signal) -> Result<ReadGuard<Self::Output>> {
        self.value(Some(signal)).await
    }
}
