use tokio::sync::mpsc;

use crate::Render;

pub struct Render_reciever(pub(crate) mpsc::UnboundedReceiver<()>);

pub struct Render_manager {
    pub render: Render,
    pub(crate) reciever: Render_reciever,
}

impl Render_manager {
    pub fn new() -> Self {
        let (sender, reciever) = mpsc::unbounded_channel();

        Self {
            render: Render::new(sender),
            reciever: Render_reciever(reciever),
        }
    }
}
