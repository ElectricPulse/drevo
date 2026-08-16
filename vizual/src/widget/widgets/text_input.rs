use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

use super::super::{Focus_provider, Layout_input, Widget_trait};
use super::title_block::Title_block;
use crate::{
    Vizual_command, Vizual_msg,
    component::{Children, context::Component_context},
    event::{Event, Key_code, Key_event},
    handlers::Submit_handler,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::Store,
    style::Color,
    theme::Theme,
    widget::widgets::{
        block::{Block, Block_style, Border_style},
        text::Text,
    },
};

#[derive(Clone)]
pub struct Text_input {
    title: String,
    input: String,
    cursor: usize,
    submit_handler: Box<dyn Submit_handler<String>>,
}

impl Text_input {
    pub fn new(title: impl Into<String>, submit_handler: Box<dyn Submit_handler<String>>) -> Self {
        Self {
            title: title.into(),
            input: String::new(),
            cursor: 0,
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
impl Widget_trait for Text_input {
    async fn layout(
        &mut self,
        Layout_input {
            focus, slots, ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        focus.set_active(true);

        let content = Text::new(self.input.clone());

        let content = Block::new(
            content,
            Block_style {
                padding: 0.0,
                background: Color::Black,
                border: Border_style {
                    color: Color::Black,
                    thickness: 0.0,
                    radius: 1.0,
                },
                focused_border: Border_style {
                    color: Color::Black,
                    thickness: 0.0,
                    radius: 1.0,
                },
            },
        );

        let block = Title_block::new(content, self.title.clone());

        Ok(vec![display!(block)])
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        if matches!(key.code, Key_code::Escape) {
            return self
                .submit_handler
                .on_submit(self.input.clone())
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
