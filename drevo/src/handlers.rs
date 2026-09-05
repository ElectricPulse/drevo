use async_trait::async_trait;
use color_eyre::eyre::Result;

use crate::{VizualCommand, VizualMsg, state::State, sync::ThreadSafe};

#[async_trait]
pub trait SubmitHandler<T: ThreadSafe + Clone>: ThreadSafe + dyn_clone::DynClone {
    async fn on_submit(&mut self, payload: T) -> Result<VizualMsg>;
}

dyn_clone::clone_trait_object!(<T> SubmitHandler<T> where T: ThreadSafe);

pub trait IntoSubmitResult: ThreadSafe {
    fn into_submit_result(self) -> Result<VizualMsg>;
}

impl IntoSubmitResult for Result<VizualMsg> {
    fn into_submit_result(self) -> Result<VizualMsg> {
        self
    }
}

impl IntoSubmitResult for VizualMsg {
    fn into_submit_result(self) -> Result<VizualMsg> {
        Ok(self)
    }
}

impl IntoSubmitResult for Result<()> {
    fn into_submit_result(self) -> Result<VizualMsg> {
        self.and_then(|()| VizualMsg::none())
    }
}

impl IntoSubmitResult for () {
    fn into_submit_result(self) -> Result<VizualMsg> {
        VizualMsg::none()
    }
}

impl IntoSubmitResult for VizualCommand {
    fn into_submit_result(self) -> Result<VizualMsg> {
        VizualMsg::new(self)
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
    async fn on_submit(&mut self, payload: T) -> Result<VizualMsg> {
        (self)(payload).await.into_submit_result()
    }
}

#[derive(Clone)]
pub struct CommandSubmitHandler {
    command: VizualCommand,
}

impl CommandSubmitHandler {
    pub fn new(command: VizualCommand) -> Self {
        Self { command }
    }
}

#[async_trait]
impl<T: ThreadSafe + Clone> SubmitHandler<T> for CommandSubmitHandler {
    async fn on_submit(&mut self, _payload: T) -> Result<VizualMsg> {
        VizualMsg::new(self.command.clone())
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
