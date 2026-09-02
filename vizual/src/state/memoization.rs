use std::sync::Arc;

use async_trait::async_trait;
use color_eyre::Result;

use super::{Read_guard, State_trait, Store};
use crate::{
    Signal,
    sync::{Mutex, Thread_safe},
};

type Callback<Dependency, Value> = dyn Fn(&[Read_guard<Dependency>]) -> Value + Send + Sync;

struct Cached<Value> {
    versions: Vec<u64>,
    value: Arc<Value>,
}

/// A value derived from stores of one value type.
///
/// It does not need to be a cached state today, but is one in case computing the value becomes
/// expensive. Dependencies must share one type because Rust vectors are homogeneous.
pub struct Memoization<Dependency: Thread_safe, Value: Thread_safe> {
    callback: Arc<Callback<Dependency, Value>>,
    stores: Vec<Store<Dependency>>,
    cached: Arc<Mutex<Option<Cached<Value>>>>,
}

impl<Dependency: Thread_safe, Value: Thread_safe> Clone for Memoization<Dependency, Value> {
    fn clone(&self) -> Self {
        Self {
            callback: Arc::clone(&self.callback),
            stores: self.stores.clone(),
            cached: Arc::clone(&self.cached),
        }
    }
}

/// Creates a memoized value from `stores`.
///
/// The callback is rerun only after one of those stores has changed. Calling `affect` also
/// subscribes its signal to every dependency.
pub fn memoization<Dependency, Value>(
    callback: impl Fn(&[Read_guard<Dependency>]) -> Value + Send + Sync + 'static,
    stores: Vec<Store<Dependency>>,
) -> Memoization<Dependency, Value>
where
    Dependency: Thread_safe,
    Value: Thread_safe,
{
    Memoization {
        callback: Arc::new(callback),
        stores,
        cached: Arc::new(Mutex::new(None)),
    }
}

impl<Dependency: Thread_safe, Value: Thread_safe> Memoization<Dependency, Value> {
    async fn value(&self, signal: Option<Signal>) -> Result<Read_guard<Value>> {
        let mut values = Vec::with_capacity(self.stores.len());
        let mut versions = Vec::with_capacity(self.stores.len());

        for store in &self.stores {
            versions.push(store.version().await?);
            let value = match &signal {
                Some(signal) => store.affect(signal.clone()).await?,
                None => store.read().await?,
            };
            values.push(value);
        }

        let mut cached = self.cached.lock().await?;
        match cached.as_ref() {
            Some(cached) if cached.versions == versions => {
                Ok(Read_guard::new(Arc::clone(&cached.value)))
            }
            _ => {
                let value = Arc::new((self.callback)(&values));
                *cached = Some(Cached {
                    versions,
                    value: Arc::clone(&value),
                });
                Ok(Read_guard::new(value))
            }
        }
    }
}

#[async_trait]
impl<Dependency: Thread_safe, Value: Thread_safe> State_trait for Memoization<Dependency, Value> {
    type Output = Value;

    async fn read(&self) -> Result<Read_guard<Self::Output>> {
        self.value(None).await
    }

    async fn affect(&self, signal: Signal) -> Result<Read_guard<Self::Output>> {
        self.value(Some(signal)).await
    }
}
