use crate::target::{Dependencies, Dependency};
use color_eyre::eyre::Result;
use std::collections::HashSet;

pub(crate) async fn get_targets(root: &Dependency) -> Result<Dependencies> {
    let mut remaining = vec![root.clone()];
    let mut unique = Dependencies::new();
    let mut target_ids = HashSet::new();

    while let Some(target) = remaining.pop() {
        let metadata = target.get_metadata().await?;
        if target_ids.insert(metadata.id) {
            remaining.extend(metadata.dependencies.into_iter().rev());
            unique.push(target);
        }
    }

    Ok(unique)
}
