use tokio::sync::mpsc;

use crate::{Render, Render_request};

pub struct Render_receiver(pub(crate) mpsc::UnboundedReceiver<Render_request>);

pub struct Render_manager {
    pub render: Render,
    pub(crate) receiver: Render_receiver,
}

impl Render_manager {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        Self {
            render: Render::new(sender),
            receiver: Render_receiver(receiver),
        }
    }
}
