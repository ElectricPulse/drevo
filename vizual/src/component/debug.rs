use async_recursion::async_recursion;
use color_eyre::eyre::Result;

use super::SharedComponent;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ComponentDebug {
    pub source_path: String,
}

impl ComponentDebug {
    pub fn new(source_path: String) -> Self {
        Self { source_path }
    }

    pub fn source_path(&self) -> &str {
        &self.source_path
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ComponentSource {
    pub component_path: String,
    pub name: String,
    pub source_path: String,
    pub depth: usize,
}

pub(crate) type ComponentTree = Vec<ComponentSource>;

impl SharedComponent {
    pub(crate) async fn component_tree(&self) -> Result<ComponentTree> {
        self.component_tree_from(String::new(), 0).await
    }

    #[async_recursion]
    async fn component_tree_from(
        &self,
        parent_path: String,
        depth: usize,
    ) -> Result<ComponentTree> {
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
        let mut tree = vec![ComponentSource {
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
