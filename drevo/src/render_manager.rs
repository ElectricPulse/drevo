use tokio::sync::mpsc;

use crate::{RenderRequest, Signal};

pub struct RenderReceiver(pub(crate) mpsc::UnboundedReceiver<RenderRequest>);

pub struct RenderManager {
    pub rerender: Signal,
    pub layout: Signal,
    pub(crate) receiver: RenderReceiver,
}

impl RenderManager {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        Self {
            rerender: Signal::new(sender.clone(), RenderRequest::Rerender),
            layout: Signal::new(sender, RenderRequest::Layout),
            receiver: RenderReceiver(receiver),
        }
    }
}
