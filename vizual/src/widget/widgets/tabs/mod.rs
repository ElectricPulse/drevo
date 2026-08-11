pub mod tab;

use async_trait::async_trait;
use color_eyre::eyre::Result;
use uuid::Uuid;
use vizual_macros::display;

use super::{
    super::{Focus_provider, Shared_widget, Widget, Widget_trait},
    layout::axis::Axis,
    positioning::anchor::{Anchor, Anchors},
    text::Text,
};
use crate::{
    Render,
    component::{Children, context::Component_context},
    display::Display,
    event::{Key_code, Key_event},
    geometry::{Direction, Rect},
    layouter::hitbox::Hitbox,
    slot::{Component_slot, manager::Slots},
    state::State,
    theme::Theme,
};

use self::tab::{Tab, Tab_specification};
use crate::utils;

#[derive(Clone)]
pub struct Tabs {
    header: Shared_widget<Tab_bar>,
}

impl Tabs {
    pub fn new(render: Render, pages: Vec<Tab_specification>) -> Self {
        let selected_page = render.new_state(Uuid::default());
        let pages: Vec<Tab> = pages.into_iter().map(Tab::new).collect();

        if let Some(initial_page) = pages.first().map(|page| page.id) {
            selected_page.store(initial_page);
        }

        let header = Tab_bar::new(selected_page.clone(), pages);

        Self {
            header: header.into_shared(),
        }
    }
}

#[derive(Clone)]
struct Page {
    slot: Component_slot,
    tab: Tab,
}

#[derive(Clone)]
struct Tab_bar {
    selected_page: State<Uuid>,
    pages: Vec<Page>,
}

impl Tab_bar {
    fn new(selected_page: State<Uuid>, tabs: Vec<Tab>) -> Self {
        Self {
            pages: tabs
                .into_iter()
                .map(|tab| Page {
                    tab,
                    slot: Component_slot::new(),
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

    fn get_selected(&self) -> Option<Widget> {
        let index = self.find_id(*self.selected_page.load())?;

        Some(self.pages[index].tab.specification.widget.clone())
    }
}

#[async_trait]
impl Widget_trait for Tab_bar {
    async fn layout(
        &mut self,
        _render: crate::Render,
        theme: State<Theme>,
        focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        focus.set_active(true);
        let mut buttons = Vec::with_capacity(self.pages.len());

        for page in self.pages.iter_mut() {
            let active = self.selected_page.load() == page.tab.id.into();
            let mut text = Text::new(&page.tab.specification.name);
            text.style.set(match active {
                true => theme.load().specific.text.selected_subtitle,
                false => theme.load().specific.text.subtitle,
            });
            let button = page.tab.button(text, self.selected_page.clone());
            let button = Anchor::new(button, Anchors::top_left());
            let button = page.slot.set_child(button, problem.clone(), hitbox).await?;

            buttons.push(button);
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

    async fn render(
        &mut self,
        _theme: State<Theme>,
        focus: &mut Focus_provider,
        _hitbox: Rect,
        _display: &mut Display<'_>,
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
        _theme: State<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let header = Anchor::new(self.header.clone(), Anchors::top_left());
        let header = display!(header);
        let mut elements = vec![header];
        {
            let selected = self.header.lock().await?.get_selected();
            if let Some(widget) = selected {
                let widget = Anchor::new(widget, Anchors::top_left());
                elements.push(display!(widget));
            }
        }

        let axis = Axis::new(Direction::Vertical, elements);
        Ok(vec![display!(axis)])
    }
}
