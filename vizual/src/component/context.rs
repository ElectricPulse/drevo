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
}
