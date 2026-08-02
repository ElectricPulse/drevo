pub mod target;
pub mod task;
mod ui;
mod utils;

use color_eyre::eyre::Result;
use std::path::PathBuf;
use vizual_macros::display;

use crate::{
    target::{Dependency, status::Target_status},
    task::View,
    ui::{header::Header, target_tree::Target_tree},
};
use vizual::{
    self,
    component::{Children, context::Component_context},
    geometry::Direction,
    layouter::hitbox::Hitbox,
    layouter::objective::Objective,
    slot::manager::Slots,
    state::State,
    theme::{Theme, Theme_manager},
    widget::{Focus_provider, Widget_trait, widgets::layout::Layout},
};

pub struct Builder {
    root: Dependency,
    selected_target: State<Option<Dependency>>,
    build_result: State<Option<std::result::Result<(), String>>>,
    name: String,
    working_directory: PathBuf,
    settings_open: State<bool>,
    themes: Theme_manager,
    pub theme: State<Theme>,
}

pub fn new(
    root: Dependency,
    working_directory: PathBuf,
    name: impl Into<String>,
    rerender: vizual::Rerender,
    themes: Theme_manager,
) -> Builder {
    let theme = themes.theme.clone();
    let root_clone = root.clone();
    let build_result = State::new(rerender.clone());
    let build_result_handle = build_result.clone();
    let view = View {
        rerender: rerender.clone(),
        theme: theme.clone(),
    };

    /*let _ = tokio::spawn(async move {
        let result = root_clone
            .ensure_ran(&view)
            .await
            .map_err(|error| error.to_string());
        build_result_handle.store(Some(result));
    });*/

    Builder {
        root,
        selected_target: State::new_with(rerender, None),
        build_result,
        name: name.into(),
        working_directory,
        settings_open: State::new(theme.rerender.clone()),
        themes,
        theme,
    }
}

#[async_trait::async_trait]
impl Widget_trait for Builder {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut vizual::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        /*let selected_widget = self.selected.widget();

        if let Some(mut error_lines) = self.get_target_errors(&targets) {
            if self.error.is_none() {
                self.error = Some(Paragraph::new().into_shared());
            }

            error_lines.push(String::new());
            let error_text = error_lines.join("\n");
            self.error
                .as_mut()
                .expect("error paragraph should exist when errors are present")
                .lock()
                .await?
                .set_content(error_text);
        } else {
            self.error = None;
        }

        let target_menu = self.target_menu.clone();
        let target_menu = Title_block::new(display!(target_menu), "Build", self.theme.clone());
        let mut detail_elements = Vec::new();
        if let Some(widget) = selected_widget {
            detail_elements.push(display!(widget));
        }
        if let Some(error) = &self.error {
            let error = error.clone();
            let error = Title_block::new(display!(error), "Error in tasks", self.theme.clone());
            detail_elements.push(display!(error));
        }

        let detail = Layout::new(
            Direction::Vertical,
            detail_elements,
            Layout_style::default(self.theme.clone()),
            Objective::default(),
            2,
        );

        let main = Layout::new(
            Direction::Horizontal,
            vec![display!(target_menu), display!(detail)],
            Layout_style::default(self.theme.clone()),
            Objective::default(),
            2,
        );

        let mut rows = Vec::new();
        if let Some(result) = self.build_result.load().as_ref() {
            let message = match result {
                Ok(()) => "Build has stopped successfully".to_string(),
                Err(error) => format!("Build has stopped unsuccessfully: {error}"),
            };
            let status = Text::new(message).set_style(self.theme.load().semantic.text.paragraph());
            let linebreak = Linebreak::new(self.theme.clone());
            rows.push(display!(status));
            rows.push(display!(linebreak));
        }
        rows.push(display!(main));
        let layout = Layout::new(
            Direction::Vertical,
            rows,
            Layout_style::default(self.theme.clone()),
            Objective::default(),
            2,
        );*/

        let tree = Target_tree::new(
            self.root.clone(),
            self.selected_target.clone(),
            self.theme.clone(),
            self.working_directory.clone(),
        );
        let header = Header::new(
            self.name.clone(),
            self.settings_open.clone(),
            self.themes.clone(),
        );
        let root = Layout::new(
            Direction::Vertical,
            vec![display!(header), display!(tree)],
            (&self.theme).into(),
            Objective::default(),
            2,
        );

        Ok(vec![display!(root)])
    }
}
