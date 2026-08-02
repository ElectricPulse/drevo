mod task;

use crate::target::{Dependencies, Target};
use std::path::PathBuf;

pub fn copy_file(
    name: impl Into<String>,
    source: PathBuf,
    destination: PathBuf,
    dependencies: Dependencies,
) -> Target<()> {
    let path = destination
        .parent()
        .map_or_else(PathBuf::new, PathBuf::from);
    Target::new_with_path(
        name,
        path,
        task::Copy_file_task {
            source,
            destination,
        },
        dependencies,
    )
}

pub fn copy_dir(
    name: impl Into<String>,
    source: PathBuf,
    destination: PathBuf,
    dependencies: Dependencies,
) -> Target<()> {
    let path = destination.clone();
    Target::new_with_path(
        name,
        path,
        task::Copy_dir_task {
            source,
            destination,
        },
        dependencies,
    )
}

pub fn create_dir(
    name: impl Into<String>,
    path: PathBuf,
    dependencies: Dependencies,
) -> Target<()> {
    Target::new_with_path(
        name,
        path.clone(),
        task::Create_dir_task { path },
        dependencies,
    )
}

pub fn write_file(
    name: impl Into<String>,
    path: PathBuf,
    content: String,
    dependencies: Dependencies,
) -> Target<()> {
    let working_path = path.parent().map_or_else(PathBuf::new, PathBuf::from);
    Target::new_with_path(
        name,
        working_path,
        task::Write_file_task { path, content },
        dependencies,
    )
}

pub fn create_directory(
    name: impl Into<String>,
    path: PathBuf,
    subdirs: Vec<String>,
    dependencies: Dependencies,
) -> Target<PathBuf> {
    Target::new_with_path(
        name,
        path.clone(),
        task::Create_directory_task { path, subdirs },
        dependencies,
    )
}
