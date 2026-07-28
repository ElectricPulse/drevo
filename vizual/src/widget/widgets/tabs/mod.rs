pub mod tab;

use async_trait::async_trait;
use color_eyre::eyre::Result;
use uuid::Uuid;
use vizual_macros::display;

use super::{
    super::{Any_renderable, Control, Focus_provider, Renderable, Shared_renderable, Widget_type},
    layout::{Layout, Style as Layout_style},
};
use crate::{
    Rerender,
    backend::graphics::Paint_context,
    component::Child_slot,
    event::{Key_code, Key_event},
    geometry::Rect,
    hitbox::{Direction, Hitbox},
    layouter::{Problem_context, constraints::Objective},
    slot_manager::Slots,
    state::State,
    theme::Theme,
};

use self::tab::{Tab, Tab_specification};
use crate::utils;

pub struct Tabs {
    header: Shared_renderable<Tab_bar>,
}

impl Tabs {
    pub fn new(rerender: Rerender, pages: Vec<Tab_specification>, theme: State<Theme>) -> Self {
        let selected_page = State::new(rerender);
        let pages: Vec<Tab> = pages
            .into_iter()
            .map(|page| Tab::new(page, selected_page.clone(), theme.clone()))
            .collect();

        if let Some(initial_page) = pages.first().map(|page| page.id) {
            selected_page.store(initial_page);
        }

        let header = Tab_bar::new(selected_page.clone(), pages);

        Self {
            header: header.into_shared(),
        }
    }
}

struct Page {
    slot: Child_slot,
    tab: Tab,
}

struct Tab_bar {
    selected_page: State<Uuid>,
    pages: Vec<Page>,
}

#[async_trait]
impl Control for Tab_bar {
    async fn on_key_press(&mut self, key: &Key_event) -> Result<crate::Vizual_msg> {
        if let Key_code::Character(char) = key.code
            && let Some(digit) = char.to_digit(10)
        {
            let digit = digit as usize;
            if digit >= self.pages.len() {
                return crate::Vizual_msg::none();
            }

            if digit == self.get_page_index() {
                return crate::Vizual_msg::new(crate::Vizual_command::None);
            }

            self.set_page_index(digit);
            return crate::Vizual_msg::new(crate::Vizual_command::Layout);
        }

        if let Some(page_index) =
            utils::handle_keys_for_iterable(key, self.pages.len(), self.get_page_index())
        {
            self.set_page_index(page_index);
            return crate::Vizual_msg::new(crate::Vizual_command::Layout);
        }

        crate::Vizual_msg::none()
    }
}

impl Tab_bar {
    fn new(selected_page: State<Uuid>, tabs: Vec<Tab>) -> Self {
        Self {
            pages: tabs
                .into_iter()
                .map(|tab| Page {
                    tab,
                    slot: Child_slot::new(),
                })
                .collect(),
            selected_page,
        }
    }
}

impl Tab_bar {
    fn find_id(&self, id: Uuid) -> Option<usize> {
        self.pages.iter().position(|page| page.tab.id == id)
    }

    fn get_page_index(&self) -> usize {
        self.find_id(*self.selected_page.load()).unwrap()
    }

    fn set_page_index(&self, page_index: usize) {
        if let Some(page) = self.pages.get(page_index) {
            self.selected_page.store(page.tab.id);
        }
    }

    async fn get_selected(&mut self) -> Option<Any_renderable> {
        let index = self.find_id(*self.selected_page.load())?;

        Some(self.pages[index].tab.specification.widget.get().await)
    }
}

#[async_trait]
impl Renderable for Tab_bar {
    async fn layout(
        &mut self,
        focus: &mut Focus_provider,
        _hitbox: Hitbox,
        problem: Problem_context,
        _slots: &mut Slots,
    ) -> Result<Widget_type> {
        focus.set_active(true);
        let mut buttons = Vec::with_capacity(self.pages.len());

        for page in self.pages.iter_mut() {
            let active = self.selected_page.load() == page.tab.id.into();
            page.tab.button.lock().await?.active = active;
            let button = page
                .slot
                .set(page.tab.button.clone(), problem.clone())
                .await?;

            buttons.push(Some(button));
        }

        let layout = Layout::new(
            Direction::Horizontal,
            buttons,
            Layout_style::default(),
            Objective::default(),
            2,
        );

        Ok(Widget_type::Virtual(Box::new(layout)))
    }

    async fn render(
        &mut self,
        focus: &mut Focus_provider,
        _hitbox: Rect,
        _paint: &mut Paint_context<'_>,
    ) -> Result<Option<Hitbox>> {
        focus.set_active(true);
        Ok(None)
    }
}

impl Control for Tabs {}

#[async_trait]
impl Renderable for Tabs {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: Hitbox,
        _problem: Problem_context,
        slots: &mut Slots,
    ) -> Result<Widget_type> {
        let tab = {
            let selected = self.header.lock().await?.get_selected().await;
            if let Some(widget) = selected {
                Some(display!(widget))
            } else {
                None
            }
        };

        let header = self.header.clone();

        let layout = Layout::new(
            Direction::Vertical,
            vec![Some(display!(header)), tab],
            Layout_style::default(),
            Objective::default(),
            2,
        );

        Ok(Widget_type::Virtual(Box::new(layout)))
    }
}
