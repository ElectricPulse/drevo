use tokio::sync::mpsc;

use crate::{Render_request, Signal};

pub struct Render_receiver(pub(crate) mpsc::UnboundedReceiver<Render_request>);

pub struct Render_manager {
    pub rerender: Signal,
    pub(crate) receiver: Render_receiver,
}

impl Render_manager {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        Self {
            rerender: Signal::new(sender),
            receiver: Render_receiver(receiver),
        }
    }
}
