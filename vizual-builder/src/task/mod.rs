use async_trait::async_trait;

use crate::target::{Output_constraints, Target};
use color_eyre::eyre::Result;
use vizual::{
    Render,
    state::State,
    theme::Theme,
    widget::{Shared_widget, Widget, Widget_trait},
};

struct Empty_widget;

impl Widget_trait for Empty_widget {}

pub fn empty_widget() -> Shared_widget<Widget> {
    let widget: Widget = Box::new(Empty_widget);
    widget.into_shared()
}

pub async fn set_widget(shared: &Shared_widget<Widget>, widget: impl Widget_trait) -> Result<()> {
    *shared.lock().await? = Box::new(widget);
    Ok(())
}

#[derive(Clone)]
pub enum Status {
    Built,
    Already_built,
}

#[async_trait]
pub trait Task_trait: Widget_trait + Send + Sync {
    type Output: Send;
    // In the future some check() should get seperated from build().
    // Leaving place for the target to have two types of dependencies one set for check() -> for example a database connection.
    // And one set of dependencies for build() -> the information to rebuild the database record if during check it notices that they dont exist.
    // There could be a third set of dependencies called optional. These would be .get() deps that during build get conditionally required
    // Also note that currently dependencies that are used via .get() inside task still have to be included in dependencies duplicitly
    async fn run(&self, manager: &mut Manager<'_>) -> Task_result<Self::Output>;
}

pub type Task_result<Output = ()> = Result<(Output, Status)>;

pub struct View {
    pub render: Render,
    pub theme: State<Theme>,
}

impl View {
    pub fn refresh(&self) {
        self.render.send();
    }
}

pub struct Manager<'a> {
    pub view: &'a View,
}

impl<'a> Manager<'a> {
    pub fn new(view: &'a View) -> Self {
        Self { view }
    }

    pub async fn get<Output: Output_constraints>(
        &mut self,
        target: &Target<Output>,
    ) -> Result<Output> {
        target.get(self.view).await
    }
}
