use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use color_eyre::eyre::Result;

use super::{ReadGuard, State, StateTrait};
use crate::{
    Signal,
    sync::{Mutex, ThreadSafe},
};

pub(super) struct StoreContent<Value: ThreadSafe> {
    pub(super) subscribers: HashMap<u64, Signal>,
    pub(super) value: State<Value>,
    pub(super) version: u64,
}

pub struct Store<Value: ThreadSafe>(Arc<Mutex<StoreContent<Value>>>);

impl<Value: ThreadSafe> Store<Value> {
    pub fn new(value: impl Into<State<Value>>) -> Self {
        Self(Arc::new(Mutex::new(StoreContent {
            subscribers: HashMap::new(),
            value: value.into(),
            version: 0,
        })))
    }

    pub async fn set(&self, state: impl Into<State<Value>>) -> Result<()> {
        let mut content = self.0.lock().await?;
        let new_state = state.into();

        for subscriber in content.subscribers.values() {
            let _ = new_state.affect(subscriber.clone()).await;
        }

        content.value = new_state;
        content.version = content.version.wrapping_add(1);

        let subscribers = content.subscribers.values().cloned().collect::<Vec<_>>();
        drop(content);

        for subscriber in subscribers {
            subscriber.send();
        }

        Ok(())
    }

    pub async fn get(&self) -> Result<ReadGuard<Value>> {
        self.read().await
    }

    pub(crate) async fn version(&self) -> Result<u64> {
        Ok(self.0.lock().await?.version)
    }

    pub async fn read(&self) -> Result<ReadGuard<Value>> {
        let inner = self.0.lock().await?.value.clone();
        inner.read().await
    }

    pub async fn affect(&self, signal: Signal) -> Result<ReadGuard<Value>> {
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

impl<Value: ThreadSafe> Clone for Store<Value> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

#[async_trait]
impl<Value: ThreadSafe> StateTrait for Store<Value> {
    type Output = Value;

    async fn read(&self) -> Result<ReadGuard<Self::Output>> {
        self.read().await
    }

    async fn affect(&self, signal: Signal) -> Result<ReadGuard<Self::Output>> {
        self.affect(signal).await
    }
}
