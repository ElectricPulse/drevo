use crate::macros::display;
use async_trait::async_trait;
use color_eyre::eyre::Result;

use super::super::{LayoutInput, WidgetTrait};
use super::title_block::TitleBlock;
use crate::{
    VizualMsg,
    component::Children,
    event::{Event, KeyCode, KeyEvent},
    handlers::SubmitHandler,
    style::Color,
    widget::widgets::{
        block::{Block, BlockStyle, BorderStyle},
        text::Text,
    },
};

#[derive(Clone)]
pub struct TextInput {
    title: String,
    input: String,
    cursor: usize,
    submit_handler: Box<dyn SubmitHandler<String>>,
}

impl TextInput {
    pub fn new(title: impl Into<String>, submit_handler: Box<dyn SubmitHandler<String>>) -> Self {
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

    fn edit_key(&mut self, key: &KeyEvent) -> bool {
        match key.code {
            KeyCode::ArrowLeft => self.cursor = self.previous_boundary(),
            KeyCode::ArrowRight => self.cursor = self.next_boundary(),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = self.input.len(),
            KeyCode::Backspace if self.cursor > 0 => {
                let previous = self.previous_boundary();
                self.input.replace_range(previous..self.cursor, "");
                self.cursor = previous;
            }
            KeyCode::Delete if self.cursor < self.input.len() => {
                self.input
                    .replace_range(self.cursor..self.next_boundary(), "");
            }
            KeyCode::Character(_) | KeyCode::Space
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
impl WidgetTrait for TextInput {
    async fn layout(
        &mut self,
        LayoutInput { focus, slots, .. }: LayoutInput<'_>,
    ) -> Result<Children> {
        focus.set_interactive(true);

        let content = Text::new(self.input.clone());

        let content = Block::new(
            content,
            BlockStyle {
                padding: 0.0,
                background: Color::Black,
                border: BorderStyle {
                    color: Color::Black,
                    thickness: 0.0,
                    radius: 1.0,
                },
                focused_border: BorderStyle {
                    color: Color::Black,
                    thickness: 0.0,
                    radius: 1.0,
                },
            },
        );

        let block = TitleBlock::new(content, self.title.clone());

        Ok(vec![display!(block)])
    }

    async fn on_key_press(&mut self, input: crate::widget::KeyPress<'_>) -> Result<VizualMsg> {
        let key = input.key;
        let relayout = input.relayout;
        if matches!(key.code, KeyCode::Escape) {
            return self.submit_handler.on_submit(self.input.clone()).await;
        }

        match self.edit_key(key) {
            true => {
                relayout.send();
                VizualMsg::none()
            }
            false => VizualMsg::none(),
        }
    }

    async fn on_other_event(
        &mut self,
        input: crate::widget::OtherEvent<'_>,
    ) -> Result<VizualMsg> {
        let event = input.event;
        let relayout = input.relayout;
        let Event::Text(text) = event else {
            return VizualMsg::none();
        };
        self.insert(text);
        relayout.send();
        VizualMsg::none()
    }
}
