pub mod tab;

use crate::macros::display;
use async_trait::async_trait;
use color_eyre::eyre::Result;
use uuid::Uuid;

use super::{
    super::{LayoutInput, RenderInput, SharedWidget, Widget, WidgetTrait},
    layout::axis::Axis,
    positioning::anchor::Anchor,
    text::Text,
};
use crate::{
    component::Children,
    event::KeyCode,
    geometry::Direction,
    state::Store,
    widget::widgets::positioning::anchor::{AnchorPosition, Anchors},
};

use self::tab::{Tab, TabSpecification};
use crate::utils;

#[derive(Clone)]
pub struct Tabs {
    header: SharedWidget<TabBar>,
}

impl Tabs {
    pub fn new(pages: Vec<TabSpecification>) -> Self {
        let pages: Vec<Tab> = pages.into_iter().map(Tab::new).collect();
        let selected_page = Store::new(pages.first().map(|page| page.id).unwrap_or_default());

        let header = TabBar::new(selected_page.clone(), pages);

        Self {
            header: header.into_shared(),
        }
    }
}

#[derive(Clone)]
struct Page {
    tab: Tab,
}

#[derive(Clone)]
struct TabBar {
    selected_page: Store<Uuid>,
    pages: Vec<Page>,
}

impl TabBar {
    fn new(selected_page: Store<Uuid>, tabs: Vec<Tab>) -> Self {
        Self {
            pages: tabs.into_iter().map(|tab| Page { tab }).collect(),
            selected_page,
        }
    }
}

impl TabBar {
    fn find_id(&self, id: Uuid) -> Option<usize> {
        self.pages.iter().position(|page| page.tab.id == id)
    }

    async fn get_page_index(&self) -> Result<usize> {
        let selected_page = *self.selected_page.read().await?;
        self.find_id(selected_page)
            .ok_or_else(|| color_eyre::eyre::eyre!("selected tab is not present"))
    }

    async fn set_page_index(&self, page_index: usize) -> Result<()> {
        if let Some(page) = self.pages.get(page_index) {
            self.selected_page.set(page.tab.id).await?;
        }
        Ok(())
    }

    async fn get_selected(&self, relayout: crate::Signal) -> Result<Option<Widget>> {
        // The owner also depends on the selected page: switching tabs changes which child exists.
        let selected_page = *self.selected_page.affect(relayout).await?;
        let Some(index) = self.find_id(selected_page) else {
            return Ok(None);
        };

        Ok(Some(self.pages[index].tab.specification.widget.clone()))
    }
}

#[async_trait]
impl WidgetTrait for TabBar {
    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            theme,
            focus,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        focus.set_interactive(true);
        let mut buttons: Vec<Widget> = Vec::with_capacity(self.pages.len());

        let selected_page = *self.selected_page.affect(relayout.clone()).await?;
        let theme = theme.affect(relayout).await?;
        for page in self.pages.iter_mut() {
            let active = selected_page == page.tab.id;
            let mut style = theme.specific.text.button;
            if !active {
                style.color = theme.semantic.text.muted;
            }
            let button = page.tab.button(
                Text::new(&page.tab.specification.name).style(style),
                self.selected_page.clone(),
            );
            let button = Anchor::new(
                button,
                Anchors {
                    vertical: Some(AnchorPosition::Middle),
                    horizontal: None,
                },
            );
            buttons.push(button.as_any());
        }

        let axis = Anchor::left(Axis::new(Direction::Horizontal, buttons));
        let axis = display!(axis);
        Ok(vec![axis])
    }

    async fn on_key_press(
        &mut self,
        input: crate::widget::KeyPress<'_>,
    ) -> Result<crate::VizualMsg> {
        let key = input.key;
        if let KeyCode::Character(char) = key.code
            && let Some(digit) = char.to_digit(10)
        {
            let digit = digit as usize;
            if digit >= self.pages.len() {
                return crate::VizualMsg::none();
            }

            if digit == self.get_page_index().await? {
                return crate::VizualMsg::new(crate::VizualCommand::None);
            }

            self.set_page_index(digit).await?;
            return crate::VizualMsg::none();
        }

        if let Some(page_index) =
            utils::handle_keys_for_iterable(key, self.pages.len(), self.get_page_index().await?)
        {
            self.set_page_index(page_index).await?;
            return crate::VizualMsg::none();
        }

        crate::VizualMsg::none()
    }

    async fn render(&mut self, RenderInput { .. }: RenderInput<'_, '_>) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl WidgetTrait for Tabs {
    async fn layout(
        &mut self,
        LayoutInput {
            slots, relayout, ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let selected = self.header.lock().await?.get_selected(relayout).await?;
        let axis = match selected {
            Some(widget) => Axis::new(Direction::Vertical, (self.header.clone(), widget)),
            None => Axis::new(Direction::Vertical, (self.header.clone(),)),
        };
        let axis = display!(axis);
        Ok(vec![axis])
    }
}
