pub mod status;
pub mod targets;

use async_trait::async_trait;
use dyn_clone::DynClone;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use crate::{
    target::status::Target_status,
    task::{Manager, Task_trait, View},
};
use color_eyre::eyre::{Result, eyre};
use vizual::{
    sync::{Mutex, Thread_safe},
    widget::{Generic_shared_widget, Shared_widget, Widget},
};

pub trait Output_constraints: Thread_safe + Clone {}
impl<T> Output_constraints for T where T: Thread_safe + Clone {}

type Task<Output> = Box<dyn Task_trait<Output = Output>>;

static NEXT_TARGET_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct Target_metadata {
    pub(crate) id: u64,
    pub(crate) name: String,
    /// This path is only used to show the user roughly where the task is working.
    pub(crate) path: Option<PathBuf>,
    pub(crate) dependencies: Dependencies,
    pub(crate) status: Target_status,
}

struct Task_manager<Output: Output_constraints> {
    task: Task<Output>,
    output: Option<Output>,
    metadata: Arc<Mutex<Target_metadata>>,
}

// Note that Task_manager get() will now block if a task is in progress
// We don't close the task even though we don't need it during dependency execution
// because we want any one else waiting for .get() to have to wait for the output not start executing dependencies or the task again
impl<Output: Output_constraints> Task_manager<Output> {
    async fn get(&mut self, view: &View) -> Result<Output> {
        if let Some(output) = &self.output {
            return Ok(output.clone());
        }

        self.set_status(Target_status::Running_dependencies).await?;
        view.refresh();

        let dependencies = self.metadata.lock().await?.dependencies.clone();

        for dependency in dependencies {
            dependency.ensure_ran(view).await?;
        }

        self.set_status(Target_status::Running).await?;
        view.refresh();

        let mut manager = Manager::new(view);
        let result = self.task.run(&mut manager).await;

        let (output, status) = match result {
            Err(err) => {
                self.set_status(Target_status::Error(Arc::new(err))).await?;
                view.refresh();
                return Err(eyre!("Task failed"));
            }
            Ok(result) => result,
        };

        self.set_status(Target_status::Satisfied(status)).await?;
        self.output = Some(output.clone());
        view.refresh();

        Ok(output)
    }

    async fn set_status(&self, status: Target_status) -> Result<()> {
        self.metadata.lock().await?.status = status;
        Ok(())
    }
}

#[async_trait]
// Since Targets are clonable and aren't in an Arc, Target trait should also be clonable
pub trait Target_trait: DynClone + Send + Sync {
    async fn get_metadata(&self) -> Result<Target_metadata>;
    async fn ensure_ran(&self, view: &View) -> Result<()>;
    fn widget(&self) -> Option<Shared_widget<Widget>>;
}

dyn_clone::clone_trait_object!(Target_trait);

// A quick word on how dependencies are added into a Target.
// Dependencies right now are passed via arguments into Target::new()
// it would make more sense to not do that and just make it so that targets are first created as Target_definition
// that can then be converted into a Target -> after adding dependcies the user wants.
// This would add extra complexity and add two new methods add_dependency and some Into<>
// An alternative is to implement add_dependency as an async on the Target - locking dependencies and appending to that vector
// but that is quite unclean aswell.
// Until the issue crystalizes the current architecture looks the cleanest.
pub type Dependency = Box<dyn Target_trait>;
pub type Dependencies = Vec<Dependency>;

// A target is sharable and clonable by default
// This is done because it simplifies the architecture.
// Also a seperate non sharable target would at the cost of a lot of added complexity only offer a small performance benefit
// of removing shared state that only really gets accessed in one place.
// Task_manager needs to be a seperate lock from metadata while one needs to be able to access the metadata regardless of if a task is running
// Task_manager bundles output and task because there is no reason to try fetching the output when a task is in progress -
// one has to wait till it's done seperately
// that also means that task_state needs access to metadata shared state because when it's done it will need to quickly update the status
// Widget is on a seperate shared state because it's gonna get locked during rendering
// That being said, if the above is true there is now no reason to encapsulate Target itself in a an Arc
#[derive(Clone)]
pub struct Target<Output: Output_constraints> {
    metadata: Arc<Mutex<Target_metadata>>,
    task: Arc<Mutex<Task_manager<Output>>>,
    widget: Option<Generic_shared_widget>,
}

impl<Output: Output_constraints> Target<Output> {
    pub fn new_independent(
        name: impl Into<String>,
        task: impl Task_trait<Output = Output> + 'static,
    ) -> Self {
        Self::new(name, task, Dependencies::new())
    }

    pub fn new_independent_with_path(
        name: impl Into<String>,
        path: PathBuf,
        task: impl Task_trait<Output = Output> + 'static,
    ) -> Self {
        Self::new_with_path(name, path, task, Dependencies::new())
    }

    pub fn new(
        name: impl Into<String>,
        task: impl Task_trait<Output = Output> + 'static,
        dependencies: Dependencies,
    ) -> Self {
        Self::create(name, None, task, dependencies)
    }

    pub fn new_with_path(
        name: impl Into<String>,
        path: PathBuf,
        task: impl Task_trait<Output = Output> + 'static,
        dependencies: Dependencies,
    ) -> Self {
        Self::create(name, Some(path), task, dependencies)
    }

    fn create(
        name: impl Into<String>,
        path: Option<PathBuf>,
        task: impl Task_trait<Output = Output> + 'static,
        dependencies: Dependencies,
    ) -> Self {
        let metadata = Arc::new(Mutex::new(Target_metadata {
            id: NEXT_TARGET_ID.fetch_add(1, Ordering::Relaxed),
            name: name.into(),
            path,
            dependencies,
            status: Target_status::Unsatisfied,
        }));

        Self {
            widget: None,
            metadata: metadata.clone(),
            task: Arc::new(Mutex::new(Task_manager {
                output: None,
                task: Box::new(task),
                metadata,
            })),
        }
    }

    pub fn set_widget(&mut self, widget: Generic_shared_widget) {
        self.widget = Some(widget)
    }

    pub async fn get(&self, view: &View) -> Result<Output> {
        self.task.lock().await?.get(view).await
    }
}

impl<Output: Output_constraints> From<Target<Output>> for Dependency {
    fn from(target: Target<Output>) -> Self {
        Box::new(target)
    }
}

#[async_trait]
impl<Output: Output_constraints> Target_trait for Target<Output> {
    async fn get_metadata(&self) -> Result<Target_metadata> {
        Ok(self.metadata.lock().await?.clone())
    }

    fn widget(&self) -> Option<Generic_shared_widget> {
        self.widget.clone()
    }

    async fn ensure_ran(&self, view: &View) -> Result<()> {
        let _ = self.get(view).await?;
        Ok(())
    }
}
