use async_recursion::async_recursion;
use color_eyre::eyre::Result;

use super::Shared_component;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Component_debug {
    pub source_path: String,
}

impl Component_debug {
    pub fn new(source_path: String) -> Self {
        Self { source_path }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Component_source {
    pub component_path: String,
    pub name: String,
    pub source_path: String,
    pub depth: usize,
}

pub(crate) type Component_tree = Vec<Component_source>;

impl Shared_component {
    pub(crate) async fn component_tree(&self) -> Result<Component_tree> {
        self.component_tree_from(String::new(), 0).await
    }

    #[async_recursion]
    async fn component_tree_from(
        &self,
        parent_path: String,
        depth: usize,
    ) -> Result<Component_tree> {
        let (name, source_path, children) = {
            let component = self.lock().await?;
            (
                component.name.clone(),
                component.debug.source_path.clone(),
                component.children.clone(),
            )
        };
        let component_path = match parent_path.is_empty() {
            true => name.clone(),
            false => format!("{parent_path}.{name}"),
        };
        let mut tree = vec![Component_source {
            component_path: component_path.clone(),
            name,
            source_path,
            depth,
        }];

        for child in children {
            tree.extend(
                child
                    .component_tree_from(component_path.clone(), depth + 1)
                    .await?,
            );
        }

        Ok(tree)
    }
}
