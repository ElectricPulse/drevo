pub mod tab;

use async_trait::async_trait;
use color_eyre::eyre::Result;
use uuid::Uuid;
use vizual_macros::display;

use super::{
    super::{Focus_provider, Shared_widget, Widget, Widget_trait},
    layout::axis::Axis,
    positioning::anchor::Anchor,
    text::Text,
};
use crate::{
    component::{Children, context::Component_context},
    event::{Key_code, Key_event},
    geometry::{Direction, Rect},
    graphics::scene::Scene,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::{State, Store},
    theme::Theme,
};

use self::tab::{Tab, Tab_specification};
use crate::utils;

#[derive(Clone)]
pub struct Tabs {
    header: Shared_widget<Tab_bar>,
}

impl Tabs {
    pub fn new(pages: Vec<Tab_specification>) -> Self {
        let pages: Vec<Tab> = pages.into_iter().map(Tab::new).collect();
        let selected_page = Store::new(pages.first().map(|page| page.id).unwrap_or_default());

        let header = Tab_bar::new(selected_page.clone(), pages);

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
struct Tab_bar {
    selected_page: Store<Uuid>,
    pages: Vec<Page>,
}

impl Tab_bar {
    fn new(selected_page: Store<Uuid>, tabs: Vec<Tab>) -> Self {
        Self {
            pages: tabs.into_iter().map(|tab| Page { tab }).collect(),
            selected_page,
        }
    }
}

impl Tab_bar {
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
            *self.selected_page.write().await? = page.tab.id;
        }
        Ok(())
    }

    async fn get_selected(&self) -> Result<Option<Widget>> {
        let selected_page = *self.selected_page.read().await?;
        let Some(index) = self.find_id(selected_page) else {
            return Ok(None);
        };

        Ok(Some(self.pages[index].tab.specification.widget.clone()))
    }
}

#[async_trait]
impl Widget_trait for Tab_bar {
    async fn layout(
        &mut self,
        render: crate::Render,
        theme: Store<Theme>,
        focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::graphics::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        focus.set_active(true);
        let mut buttons: Vec<Widget> = Vec::with_capacity(self.pages.len());

        let selected_page = *self.selected_page.affect(render.clone()).await?;
        let theme = theme.affect(render).await?;
        for page in self.pages.iter_mut() {
            let active = selected_page == page.tab.id;
            let mut text = Text::new(&page.tab.specification.name);
            text.style.set(match active {
                true => theme.specific.text.selected_subtitle,
                false => theme.specific.text.subtitle,
            });
            let button = page.tab.button(text, self.selected_page.clone());
            let button = Anchor::left(button);
            buttons.push(Box::new(button));
        }

        let axis = Axis::new(Direction::Horizontal, buttons);
        Ok(vec![display!(axis)])
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<crate::Vizual_msg> {
        if let Key_code::Character(char) = key.code
            && let Some(digit) = char.to_digit(10)
        {
            let digit = digit as usize;
            if digit >= self.pages.len() {
                return crate::Vizual_msg::none();
            }

            if digit == self.get_page_index().await? {
                return crate::Vizual_msg::new(crate::Vizual_command::None);
            }

            self.set_page_index(digit).await?;
            return crate::Vizual_msg::new(crate::Vizual_command::Layout);
        }

        if let Some(page_index) =
            utils::handle_keys_for_iterable(key, self.pages.len(), self.get_page_index().await?)
        {
            self.set_page_index(page_index).await?;
            return crate::Vizual_msg::new(crate::Vizual_command::Layout);
        }

        crate::Vizual_msg::none()
    }

    async fn render(
        &mut self,
        _render: crate::Render,
        _theme: Store<Theme>,
        focus: &mut Focus_provider,
        _hitbox: Rect,
        _scene: &mut Scene<'_>,
        _text_context: &mut crate::graphics::text::Text_context,
        _context: &crate::component::Render_context<'_>,
    ) -> Result<Option<Hitbox>> {
        focus.set_active(true);
        Ok(None)
    }
}

#[async_trait]
impl Widget_trait for Tabs {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: Store<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::graphics::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let mut elements: Vec<Widget> = vec![Box::new(self.header.clone())];
        {
            let selected = self.header.lock().await?.get_selected().await?;
            if let Some(widget) = selected {
                elements.push(Box::new(widget));
            }
        }

        let axis = Axis::new(Direction::Vertical, elements);
        Ok(vec![display!(axis)])
    }
}
