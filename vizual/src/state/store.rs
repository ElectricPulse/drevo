use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use async_trait::async_trait;
use color_eyre::eyre::Result;

use super::{Read_guard, State};
use crate::{
    Render,
    sync::{Mutex, MutexGuard, Thread_safe},
};

pub(super) struct Store_content<Value> {
    pub(super) subscribers: HashMap<u64, Render>,
    pub(super) value: Value,
}

pub struct Store<Value>(Arc<Mutex<Store_content<Value>>>);

impl<Value> Store<Value> {
    pub fn new(value: Value) -> Self {
        Self(Arc::new(Mutex::new(Store_content {
            subscribers: HashMap::new(),
            value,
        })))
    }

    pub async fn write(&self) -> Result<Write_guard<'_, Value>> {
        Ok(Write_guard {
            guard: Some(self.0.lock().await?),
        })
    }
}

impl<Value> Clone for Store<Value> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

#[async_trait]
impl<Value> State for Store<Value>
where
    Value: Thread_safe,
{
    type Output = Value;

    async fn read(&self) -> Result<Read_guard<'_, Self::Output>> {
        Ok(Read_guard::store(self.0.lock().await?))
    }

    async fn affect(&self, signal: Render) -> Result<Read_guard<'_, Self::Output>> {
        let mut content = self.0.lock().await?;
        let _ = content.subscribers.entry(signal.id).or_insert(signal);
        Ok(Read_guard::store(content))
    }
}

pub struct Write_guard<'a, Value> {
    guard: Option<MutexGuard<'a, Store_content<Value>>>,
}

impl<Value> Deref for Write_guard<'_, Value> {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        &self
            .guard
            .as_ref()
            .expect("write guard must contain its mutex guard")
            .value
    }
}

impl<Value> DerefMut for Write_guard<'_, Value> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self
            .guard
            .as_mut()
            .expect("write guard must contain its mutex guard")
            .value
    }
}

impl<Value> Drop for Write_guard<'_, Value> {
    fn drop(&mut self) {
        let guard = self
            .guard
            .take()
            .expect("write guard must contain its mutex guard");
        let subscribers = guard.subscribers.values().cloned().collect::<Vec<_>>();
        drop(guard);

        for subscriber in subscribers {
            subscriber.send();
        }
    }
}
