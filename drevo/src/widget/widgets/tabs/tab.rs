use uuid::Uuid;

use super::super::button::Button;
use crate::widget::{Widget, WidgetTrait};
use crate::{DrevoMsg, state::Store};

#[derive(Clone)]
pub struct TabSpecification {
    pub widget: Widget,
    pub name: String,
}

impl TabSpecification {
    pub fn new(name: impl Into<String>, widget: impl WidgetTrait) -> Self {
        Self {
            widget: widget.as_any(),
            name: name.into(),
        }
    }
}

#[derive(Clone)]
pub struct Tab {
    pub specification: TabSpecification,
    pub id: Uuid,
}

impl Tab {
    pub fn new(specification: TabSpecification) -> Self {
        let id = Uuid::new_v4();

        Self { specification, id }
    }

    pub(super) fn button(&self, content: impl WidgetTrait, selected_page: Store<Uuid>) -> Button {
        let id = self.id;
        Button::new(content, move |_| {
            let selected_page = selected_page.clone();
            async move {
                selected_page.set(id).await?;
                DrevoMsg::none()
            }
        })
    }
}
