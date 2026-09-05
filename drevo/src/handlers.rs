use async_trait::async_trait;
use color_eyre::eyre::Result;

use crate::{DrevoCommand, DrevoMsg, state::State, sync::ThreadSafe};

#[async_trait]
pub trait SubmitHandler<T: ThreadSafe + Clone>: ThreadSafe + dyn_clone::DynClone {
    async fn on_submit(&mut self, payload: T) -> Result<DrevoMsg>;
}

dyn_clone::clone_trait_object!(<T> SubmitHandler<T> where T: ThreadSafe);

pub trait IntoSubmitResult: ThreadSafe {
    fn into_submit_result(self) -> Result<DrevoMsg>;
}

impl IntoSubmitResult for Result<DrevoMsg> {
    fn into_submit_result(self) -> Result<DrevoMsg> {
        self
    }
}

impl IntoSubmitResult for DrevoMsg {
    fn into_submit_result(self) -> Result<DrevoMsg> {
        Ok(self)
    }
}

impl IntoSubmitResult for Result<()> {
    fn into_submit_result(self) -> Result<DrevoMsg> {
        self.and_then(|()| DrevoMsg::none())
    }
}

impl IntoSubmitResult for () {
    fn into_submit_result(self) -> Result<DrevoMsg> {
        DrevoMsg::none()
    }
}

impl IntoSubmitResult for DrevoCommand {
    fn into_submit_result(self) -> Result<DrevoMsg> {
        DrevoMsg::new(self)
    }
}

#[async_trait]
impl<F, Fut, T: Clone, Output> SubmitHandler<T> for F
where
    F: FnMut(T) -> Fut + Clone + ThreadSafe,
    Fut: std::future::Future<Output = Output> + Send + 'static,
    Output: IntoSubmitResult,
    T: ThreadSafe,
{
    async fn on_submit(&mut self, payload: T) -> Result<DrevoMsg> {
        (self)(payload).await.into_submit_result()
    }
}

#[derive(Clone)]
pub struct CommandSubmitHandler {
    command: DrevoCommand,
}

impl CommandSubmitHandler {
    pub fn new(command: DrevoCommand) -> Self {
        Self { command }
    }
}

#[async_trait]
impl<T: ThreadSafe + Clone> SubmitHandler<T> for CommandSubmitHandler {
    async fn on_submit(&mut self, _payload: T) -> Result<DrevoMsg> {
        DrevoMsg::new(self.command.clone())
    }
}

#[async_trait]
pub trait RetrieveHandler<Value: ThreadSafe>: ThreadSafe + dyn_clone::DynClone {
    async fn on_retrieve(&mut self) -> Result<State<Value>>;
}

dyn_clone::clone_trait_object!(<Value> RetrieveHandler<Value> where Value: ThreadSafe);

#[async_trait]
impl<F, Fut, Value> RetrieveHandler<Value> for F
where
    F: FnMut() -> Fut + Clone + ThreadSafe,
    Fut: std::future::Future<Output = Result<State<Value>>> + Send + 'static,
    Value: ThreadSafe,
{
    async fn on_retrieve(&mut self) -> Result<State<Value>> {
        (self)().await
    }
}
