use async_trait::async_trait;
use color_eyre::eyre::Result;

use crate::{
    Vizual_command, Vizual_msg,
    state::State,
    sync::Thread_safe,
};

#[async_trait]
pub trait Submit_handler<T: Thread_safe + Clone>: Thread_safe + dyn_clone::DynClone {
    async fn on_submit(&mut self, payload: T) -> Result<Vizual_msg>;
}

dyn_clone::clone_trait_object!(<T> Submit_handler<T> where T: Thread_safe);

pub trait Into_submit_result: Thread_safe {
    fn into_submit_result(self) -> Result<Vizual_msg>;
}

impl Into_submit_result for Result<Vizual_msg> {
    fn into_submit_result(self) -> Result<Vizual_msg> {
        self
    }
}

impl Into_submit_result for Vizual_msg {
    fn into_submit_result(self) -> Result<Vizual_msg> {
        Ok(self)
    }
}

impl Into_submit_result for Result<()> {
    fn into_submit_result(self) -> Result<Vizual_msg> {
        self.and_then(|()| Vizual_msg::none())
    }
}

impl Into_submit_result for () {
    fn into_submit_result(self) -> Result<Vizual_msg> {
        Vizual_msg::none()
    }
}

impl Into_submit_result for Vizual_command {
    fn into_submit_result(self) -> Result<Vizual_msg> {
        Vizual_msg::new(self)
    }
}

#[async_trait]
impl<F, Fut, T: Clone, Output> Submit_handler<T> for F
where
    F: FnMut(T) -> Fut + Clone + Thread_safe,
    Fut: std::future::Future<Output = Output> + Send + 'static,
    Output: Into_submit_result,
    T: Thread_safe,
{
    async fn on_submit(&mut self, payload: T) -> Result<Vizual_msg> {
        (self)(payload).await.into_submit_result()
    }
}

#[derive(Clone)]
pub struct Command_submit_handler {
    command: Vizual_command,
}

impl Command_submit_handler {
    pub fn new(command: Vizual_command) -> Self {
        Self { command }
    }
}

#[async_trait]
impl<T: Thread_safe + Clone> Submit_handler<T> for Command_submit_handler {
    async fn on_submit(&mut self, _payload: T) -> Result<Vizual_msg> {
        Vizual_msg::new(self.command.clone())
    }
}

#[async_trait]
pub trait Retrieve_handler<Value: Thread_safe>: Thread_safe + dyn_clone::DynClone {
    async fn on_retrieve(&mut self) -> Result<State<Value>>;
}

dyn_clone::clone_trait_object!(<Value> Retrieve_handler<Value> where Value: Thread_safe);
