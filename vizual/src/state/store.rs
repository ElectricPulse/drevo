use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use color_eyre::eyre::Result;

use super::{Read_guard, State, State_trait};
use crate::{
    Render,
    sync::{Mutex, Thread_safe},
};

pub(super) struct Store_content<Value: Thread_safe> {
    pub(super) subscribers: HashMap<u64, Render>,
    pub(super) value: State<Value>,
}

pub struct Store<Value: Thread_safe>(Arc<Mutex<Store_content<Value>>>);

impl<Value: Thread_safe> Store<Value> {
    pub fn new(value: impl Into<State<Value>>) -> Self {
        Self(Arc::new(Mutex::new(Store_content {
            subscribers: HashMap::new(),
            value: value.into(),
        })))
    }

    pub async fn set(&self, state: impl Into<State<Value>>) -> Result<()> {
        let mut content = self.0.lock().await?;
        let new_state = state.into();

        for subscriber in content.subscribers.values() {
            let _ = new_state.affect(subscriber.clone()).await;
        }

        content.value = new_state;

        let subscribers = content.subscribers.values().cloned().collect::<Vec<_>>();
        drop(content);

        for subscriber in subscribers {
            subscriber.send();
        }

        Ok(())
    }

    pub async fn get(&self) -> Result<Read_guard<Value>> {
        self.read().await
    }

    pub async fn read(&self) -> Result<Read_guard<Value>> {
        let inner = self.0.lock().await?.value.clone();
        inner.read().await
    }

    pub async fn affect(&self, signal: Render) -> Result<Read_guard<Value>> {
        let inner = {
            let mut content = self.0.lock().await?;
            let _ = content
                .subscribers
                .entry(signal.id)
                .or_insert(signal.clone());
            content.value.clone()
        };
        inner.affect(signal).await
    }
}

impl<Value: Thread_safe> Clone for Store<Value> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

#[async_trait]
impl<Value: Thread_safe> State_trait for Store<Value> {
    type Output = Value;

    async fn read(&self) -> Result<Read_guard<Self::Output>> {
        self.read().await
    }

    async fn affect(&self, signal: Render) -> Result<Read_guard<Self::Output>> {
        self.affect(signal).await
    }
}
