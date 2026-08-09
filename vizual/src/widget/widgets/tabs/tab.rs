use async_trait::async_trait;
use color_eyre::eyre::Result;
use uuid::Uuid;

use super::super::button::Button;
use crate::widget::{General_shared_widget, Shared_widget, Widget_trait as _};
use crate::{Vizual_command, Vizual_msg, handlers::Submit_handler, state::State};

pub struct Tab_specification {
    pub widget: General_shared_widget,
    pub name: String,
}

impl Tab_specification {
    pub fn new(name: impl Into<String>, widget: impl Into<General_shared_widget>) -> Self {
        Self {
            widget: widget.into(),
            name: name.into(),
        }
    }
}

pub struct Tab {
    pub specification: Tab_specification,
    pub button: Shared_widget<Button>,
    pub id: Uuid,
}

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
    pub fn new(specification: Tab_specification, selected_page: State<Uuid>) -> Self {
        let id = Uuid::new_v4();

        Self {
            button: Button::new(
                &specification.name,
                Box::new(Tab_button_click_handler {
                    state: selected_page,
                    id,
                }),
            )
            .into_shared(),
            specification,
            id,
        }
    }
}
