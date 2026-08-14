use std::sync::Arc;

use arc_swap::ArcSwap;
use async_trait::async_trait;

use crate::Render;

type Load<Value> = Arc<dyn Fn() -> Arc<Value> + Send + Sync>;
type Store<Value> = Arc<dyn Fn(Value) + Send + Sync>;


#[async_trait]
trait State<Value> {
    async fn get(&self, signal: Render) -> Result<Value>;
}

struct Store {
    subscribers: 
}

// TODO: this is pretty trash LLM code
/// Application state created by [`Render::new_state`].
#[derive(Clone)]
pub struct State<Value> {
    load: Load<Value>,
    store: Store<Value>,
    pub render: Render,
}

impl Render {
    pub fn new_state<Value: Send + Sync + 'static>(&self, value: Value) -> State<Value> {
        let value = Arc::new(ArcSwap::from_pointee(value));
        let load_value = value.clone();
        let store_value = value;

        State {
            load: Arc::new(move || load_value.load_full()),
            store: Arc::new(move |value| store_value.store(Arc::new(value))),
            render: self.clone(),
        }
    }
}

impl<Value> State<Value> {
    pub fn load(&self) -> Arc<Value> {
        (self.load)()
    }

    pub fn store(&self, value: Value) {
        (self.store)(value);
        self.render.send();
    }

    pub fn set(&self, value: Value) {
        self.store(value);
    }
}
