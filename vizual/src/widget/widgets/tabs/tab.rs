use uuid::Uuid;

use super::super::button::Button;
use crate::widget::{Widget, Widget_trait};
use crate::{Vizual_command, Vizual_msg, state::Store};

#[derive(Clone)]
pub struct Tab_specification {
    pub widget: Widget,
    pub name: String,
}

impl Tab_specification {
    pub fn new(name: impl Into<String>, widget: impl Widget_trait) -> Self {
        Self {
            widget: widget.as_any(),
            name: name.into(),
        }
    }
}

#[derive(Clone)]
pub struct Tab {
    pub specification: Tab_specification,
    pub id: Uuid,
}

impl Tab {
    pub fn new(specification: Tab_specification) -> Self {
        let id = Uuid::new_v4();

        Self { specification, id }
    }

    pub(super) fn button(&self, content: impl Widget_trait, selected_page: Store<Uuid>) -> Button {
        let id = self.id;
        Button::new(content, move |_| {
            let selected_page = selected_page.clone();
            async move {
                selected_page.set(id).await?;
                Vizual_msg::new(Vizual_command::Layout)
            }
        })
    }
}
