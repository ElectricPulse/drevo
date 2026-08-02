mod task;

use crate::target::{Dependencies, Target};
use std::path::PathBuf;

pub fn copy_file(
    name: impl Into<String>,
    source: PathBuf,
    destination: PathBuf,
    dependencies: Dependencies,
) -> Target<()> {
    Target::new(
        name,
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
    Target::new(
        name,
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
    Target::new(name, task::Create_dir_task { path }, dependencies)
}

pub fn write_file(
    name: impl Into<String>,
    path: PathBuf,
    content: String,
    dependencies: Dependencies,
) -> Target<()> {
    Target::new(name, task::Write_file_task { path, content }, dependencies)
}

pub fn create_directory(
    name: impl Into<String>,
    path: PathBuf,
    subdirs: Vec<String>,
    dependencies: Dependencies,
) -> Target<PathBuf> {
    Target::new(
        name,
        task::Create_directory_task { path, subdirs },
        dependencies,
    )
}
