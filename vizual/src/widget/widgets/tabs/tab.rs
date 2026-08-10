use async_trait::async_trait;
use color_eyre::eyre::Result;
use uuid::Uuid;

use super::super::button::Button;
use crate::widget::{Widget, Widget_trait};
use crate::{Vizual_command, Vizual_msg, handlers::Submit_handler, state::State};

#[derive(Clone)]
pub struct Tab_specification {
    pub widget: Widget,
    pub name: String,
}

impl Tab_specification {
    pub fn new(name: impl Into<String>, widget: impl Widget_trait) -> Self {
        Self {
            widget: Box::new(widget),
            name: name.into(),
        }
    }
}

#[derive(Clone)]
pub struct Tab {
    pub specification: Tab_specification,
    pub id: Uuid,
}

#[derive(Clone)]
struct Tab_button_click_handler {
    state: State<Uuid>,
    id: Uuid,
}

#[async_trait]
impl Submit_handler<String> for Tab_button_click_handler {
    async fn on_submit(&mut self, _label: Option<String>) -> Result<Vizual_msg> {
        self.state.store(self.id);
        Vizual_msg::new(Vizual_command::Layout)
    }
}

impl Tab {
    pub fn new(specification: Tab_specification) -> Self {
        let id = Uuid::new_v4();

        Self { specification, id }
    }

    pub(super) fn button(&self, content: impl Widget_trait, selected_page: State<Uuid>) -> Button {
        Button::new(
            content,
            Tab_button_click_handler {
                state: selected_page,
                id: self.id,
            },
        )
    }
}
