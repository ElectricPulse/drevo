use std::sync::Arc;

use arc_swap::{ArcSwap, ArcSwapOption};

use crate::Render;

type Load<Value> = Arc<dyn Fn() -> Arc<Value> + Send + Sync>;
type Store<Value> = Arc<dyn Fn(Value) + Send + Sync>;

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

impl<Value: Clone + Send + Sync + 'static> State<Value> {
    pub fn project<Field, Get_field>(&self, field: Get_field) -> State<Field>
    where
        Field: Clone + Send + Sync + 'static,
        Get_field: for<'a> Fn(&'a Value) -> &'a Field + Send + Sync + 'static,
    {
        let field = Arc::new(field);
        let parent_load = self.load.clone();
        let override_value = Arc::new(ArcSwapOption::empty());
        let load = {
            let field = field.clone();
            let parent_load = parent_load.clone();
            let override_value = override_value.clone();
            Arc::new(move || {
                override_value
                    .load_full()
                    .unwrap_or_else(|| Arc::new(field(&parent_load()).clone()))
            }) as Load<Field>
        };
        let store = Arc::new(move |value| override_value.store(Some(Arc::new(value))));

        State {
            load,
            store,
            render: self.render.clone(),
        }
    }
}
