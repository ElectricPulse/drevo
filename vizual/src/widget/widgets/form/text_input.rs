use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

use super::super::super::{Focus_provider, Widget_trait};
use super::super::title_block::Title_block;
use crate::{
    Vizual_command, Vizual_msg,
    component::{Children, context::Component_context},
    display::Display,
    event::{Event, Key_code, Key_event},
    geometry::{Direction, Point, Rect, Size},
    handlers::Submit_handler,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::State,
    style::Color,
    sync::Mutex,
    text::{Styled_text, Text_window},
    theme::Theme,
    widget::widgets::{
        positioning::anchor::{Anchor, Anchors},
        text::Text,
    },
};

#[derive(Clone)]
struct Text_input_content {
    input: String,
    cursor: usize,
    scroll_x: Arc<Mutex<f64>>,
    focused: Arc<AtomicBool>,
}

#[derive(Clone)]
pub struct Text_input {
    title: String,
    input: String,
    cursor: usize,
    scroll_x: Arc<Mutex<f64>>,
    focused: Arc<AtomicBool>,
    submit_handler: Box<dyn Submit_handler<String>>,
}

impl Text_input {
    pub fn new(title: impl Into<String>, submit_handler: Box<dyn Submit_handler<String>>) -> Self {
        Self {
            title: title.into(),
            input: String::new(),
            cursor: 0,
            scroll_x: Arc::new(Mutex::new(0.0)),
            focused: Arc::new(AtomicBool::new(false)),
            submit_handler,
        }
    }

    pub fn set_selected(&mut self, text: impl Into<String>) {
        self.input = text.into();
        self.cursor = self.input.len();
    }

    pub fn get_selected(&self) -> String {
        self.input.clone()
    }

    fn previous_boundary(&self) -> usize {
        self.input[..self.cursor]
            .char_indices()
            .next_back()
            .map(|(index, _)| index)
            .unwrap_or_default()
    }

    fn next_boundary(&self) -> usize {
        self.input[self.cursor..]
            .chars()
            .next()
            .map(|character| self.cursor + character.len_utf8())
            .unwrap_or(self.input.len())
    }

    fn insert(&mut self, text: &str) {
        self.input.insert_str(self.cursor, text);
        self.cursor += text.len();
    }

    fn edit_key(&mut self, key: &Key_event) -> bool {
        match key.code {
            Key_code::Arrow_left => self.cursor = self.previous_boundary(),
            Key_code::Arrow_right => self.cursor = self.next_boundary(),
            Key_code::Home => self.cursor = 0,
            Key_code::End => self.cursor = self.input.len(),
            Key_code::Backspace if self.cursor > 0 => {
                let previous = self.previous_boundary();
                self.input.replace_range(previous..self.cursor, "");
                self.cursor = previous;
            }
            Key_code::Delete if self.cursor < self.input.len() => {
                self.input
                    .replace_range(self.cursor..self.next_boundary(), "");
            }
            Key_code::Character(_) | Key_code::Space
                if !key.modifiers.control && !key.modifiers.alt =>
            {
                let Some(text) = &key.text else {
                    return false;
                };
                self.insert(text);
            }
            _ => return false,
        }
        true
    }
}

#[async_trait]
impl Widget_trait for Text_input_content {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: State<Theme>,
        focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        hitbox.set_static_dimension(&problem, Direction::Vertical, 20.0);
        hitbox.set_static_dimension(&problem, Direction::Horizontal, 10.0);

        Ok(vec![])
    }
    async fn render(
        &mut self,
        _theme: State<Theme>,
        _focus: &mut Focus_provider,
        hitbox: Rect,
        display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        let color = match self.input.is_empty() {
            true => Color::White,
            false => Color::Light_green,
        };
        let cursor_x = display.measure_text(&self.input[..self.cursor]).width;
        let mut scroll_x = self.scroll_x.lock().await?;
        if cursor_x < *scroll_x {
            *scroll_x = cursor_x;
        } else if cursor_x + 1.0 > *scroll_x + hitbox.size.width {
            *scroll_x = (cursor_x + 1.0 - hitbox.size.width).max(0.0);
        }

        let styled = Styled_text::plain(&self.input, color);
        let layout = display.build_layout(&styled);
        display.paint_layout(
            &layout,
            hitbox.origin,
            Some(Text_window {
                offset: Point::new(*scroll_x, 0.0),
                size: hitbox.size,
            }),
        );

        if self.focused.load(Ordering::Relaxed) {
            let height = display.measure_text(" ").height.min(hitbox.size.height);
            display.fill_rect(
                Rect {
                    origin: Point::new(hitbox.origin.x + cursor_x - *scroll_x, hitbox.origin.y),
                    size: Size::new(1.0, height),
                },
                color,
            );
        }

        Ok(None)
    }
}

#[async_trait]
impl Widget_trait for Text_input {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: State<Theme>,
        focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        focus.set_active(true);

        let content = Text_input_content {
            input: self.input.clone(),
            cursor: self.cursor,
            scroll_x: self.scroll_x.clone(),
            focused: self.focused.clone(),
        };

        let block = Title_block::new(Text::new("hi"), self.title.clone());

        Ok(vec![display!(block)])
    }

    async fn render(
        &mut self,
        _theme: State<Theme>,
        focus: &mut Focus_provider,
        _hitbox: Rect,
        _display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        self.focused.store(focus.get(), Ordering::Relaxed);
        Ok(None)
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        if matches!(key.code, Key_code::Escape) {
            return self
                .submit_handler
                .on_submit(Some(self.input.clone()))
                .await;
        }

        match self.edit_key(key) {
            true => Vizual_msg::new(Vizual_command::Layout),
            false => Vizual_msg::none(),
        }
    }

    async fn on_other_event(&mut self, event: &Event) -> Result<Vizual_msg> {
        let Event::Text(text) = event else {
            return Vizual_msg::none();
        };
        self.insert(text);
        Vizual_msg::new(Vizual_command::Layout)
    }
}
