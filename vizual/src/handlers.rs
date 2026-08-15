use async_trait::async_trait;
use color_eyre::eyre::Result;

use crate::{Vizual_command, Vizual_msg, sync::Thread_safe};

#[async_trait]
pub trait Submit_handler<T: Thread_safe>: Thread_safe + dyn_clone::DynClone {
    async fn on_submit(&mut self, payload: T) -> Result<Vizual_msg>;
}

dyn_clone::clone_trait_object!(<T> Submit_handler<T> where T: Thread_safe);

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
impl<T: Thread_safe> Submit_handler<T> for Command_submit_handler {
    async fn on_submit(&mut self, _payload: Option<T>) -> Result<Vizual_msg> {
        Vizual_msg::new(self.command.clone())
    }
}

#[async_trait]
pub trait Retrieve_handler<Value>: Thread_safe + dyn_clone::DynClone {
    async fn on_retrieve(&mut self) -> Result<Value>;
}

dyn_clone::clone_trait_object!(<Value> Retrieve_handler<Value>);
