use async_trait::async_trait;
use color_eyre::Result;
use derive_new::new;
use std::path::{Path, PathBuf};
use vizual::{
    component::{Children, context::Component_context},
    geometry::Direction,
    handlers::Retrieve_handler,
    layouter::{hitbox::Hitbox, objective::Objective},
    slot::manager::Slots,
    state::State,
    theme::Theme,
    widget::{
        Focus_provider, Widget_trait,
        custom_widget::Custom_widget_trait,
        widgets::{
            anchor::Anchor,
            full::Full,
            icon::Icon,
            layout::Layout,
            menu::{Menu, Shared_menu_item, get_selector},
            text::Text,
        },
    },
};
use vizual_macros::display;

use crate::target::Dependency;
use crate::utils::get_targets;

#[derive(new)]
struct Target_tree_item {
    target: Dependency,
    theme: State<Theme>,
    working_directory: PathBuf,
}

#[async_trait::async_trait]
impl Retrieve_handler<Dependency> for Target_tree_item {
    async fn on_retrieve(&mut self) -> Result<Dependency> {
        Ok(self.target.clone())
    }
}

#[async_trait::async_trait]
impl Custom_widget_trait for Target_tree_item {
    type Payload = bool;

    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut vizual::text::Text_context,
        slots: &mut Slots,
        _selected: bool,
    ) -> Result<Children> {
        let metadata = self.target.get_metadata().await?;

        let icon = Icon::new(metadata.status.get_icon(), (&self.theme).into());
        let icon = Anchor::center(display!(icon));

        let name = Text::new(metadata.name, (&self.theme).into());
        let mut details = vec![display!(name)];
        if let Some(path) = metadata.path {
            let path = path
                .strip_prefix(&self.working_directory)
                .unwrap_or(path.as_path());
            let path = display_relative_path(path);
            details.push(display!(Text::new(
                format!("Working directory: {path}"),
                (&self.theme).into(),
            )));
        }

        let details = Layout::new(
            Direction::Vertical,
            details,
            (&self.theme).into(),
            Objective::default(),
            2,
        );

        let row = Layout::new(
            Direction::Horizontal,
            vec![display!(details), display!(icon)],
            (&self.theme).into(),
            Objective::default(),
            2,
        );

        let row = Full::new(display!(row));

        Ok(vec![display!(row)])
    }
}

#[derive(new)]
pub struct Target_tree {
    root: Dependency,
    selected: State<Option<Dependency>>,
    theme: State<Theme>,
    working_directory: PathBuf,
}

#[async_trait]
impl Widget_trait for Target_tree {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut vizual::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let targets = get_targets(&self.root)
            .await?
            .into_iter()
            .map(|target| -> Shared_menu_item<Dependency> {
                Target_tree_item::new(target, self.theme.clone(), self.working_directory.clone())
                    .into_shared()
            })
            .collect::<Vec<_>>();

        let default_target = get_selector(
            targets
                .first()
                .expect("target tree must contain its root target"),
        );
        let menu = Menu::new(targets, default_target, None, self.theme.clone());
        let menu = Full::new(display!(menu));

        Ok(vec![display!(menu)])
    }
}

fn display_relative_path(path: &Path) -> String {
    match path.as_os_str().is_empty() {
        true => ".".to_string(),
        false => path.display().to_string(),
    }
}
