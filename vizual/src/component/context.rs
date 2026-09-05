use std::sync::Arc;

use crate::layouter::variables::Variables;

/// Per-pass mounting data. Layout declarations belong to `Formula`.
#[derive(Clone)]
pub struct ComponentContext {
    pub variables: Arc<Variables>,
    pub component_path: Vec<String>,
}

impl ComponentContext {
    pub fn new(variables: Arc<Variables>) -> Self {
        Self {
            variables,
            component_path: Vec::new(),
        }
    }

    // This is a bodge and is supposed to prevent that unnecessary cloning that the profiler deemed bad.
    pub fn push(&mut self, _name: impl IntoComponentPath) {
        #[cfg(debug_assertions)]
        self.component_path.push(_name.into_component_path());
    }

    pub fn join(&self) -> String {
        #[cfg(debug_assertions)]
        return self.component_path.join(".");
        #[cfg(not(debug_assertions))]
        return String::new();
    }
}

pub trait IntoComponentPath {
    fn into_component_path(self) -> String;
}

impl<F: FnOnce() -> String> IntoComponentPath for F {
    fn into_component_path(self) -> String {
        self()
    }
}

impl IntoComponentPath for String {
    fn into_component_path(self) -> String {
        self
    }
}

impl IntoComponentPath for &String {
    fn into_component_path(self) -> String {
        self.clone()
    }
}

impl IntoComponentPath for &str {
    fn into_component_path(self) -> String {
        self.to_string()
    }
}
