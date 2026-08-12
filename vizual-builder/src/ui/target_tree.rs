use async_trait::async_trait;
use color_eyre::Result;
use derive_new::new;
use std::path::{Path, PathBuf};
use vizual::{
    component::{Children, context::Component_context},
    geometry::Direction,
    handlers::Retrieve_handler,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::State,
    widget::{
        Focus_provider, Widget, Widget_trait,
        custom_widget::Custom_widget_trait,
        widgets::{
            icon::Icon,
            layout::axis::Axis,
            menu::{Menu, Shared_menu_item, get_selector},
            positioning::anchor::{Anchor, Anchors},
            text::Text,
        },
    },
};
use vizual_macros::display;

use crate::target::Dependency;
use crate::utils::get_targets;

#[derive(Clone, new)]
struct Target_tree_item {
    target: Dependency,
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
        _render: vizual::Render,
        _theme: State<vizual::theme::Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut vizual::text::Text_context,
        slots: &mut Slots,
        _selected: bool,
    ) -> Result<Children> {
        let metadata = self.target.get_metadata().await?;

        let icon = Icon::new(metadata.status.get_icon());
        let icon = Anchor::new(icon, Anchors::middle());

        let name = Text::new(metadata.name);
        let name = Anchor::new(name, Anchors::top_left());
        let mut details: Vec<Widget> = vec![Box::new(name)];
        if let Some(path) = metadata.path {
            let path = path
                .strip_prefix(&self.working_directory)
                .unwrap_or(path.as_path());
            let path = display_relative_path(path);
            let path = Text::new(format!("Working directory: {path}"));
            let path = Anchor::new(path, Anchors::top_left());
            details.push(Box::new(path));
        }

        let details = Axis::new(Direction::Vertical, details);
        let details = Anchor::new(details, Anchors::top_left());

        let row = Axis::new(
            Direction::Horizontal,
            vec![Box::new(details), Box::new(icon)],
        );

        Ok(vec![display!(row)])
    }
}

#[derive(Clone, new)]
pub struct Target_tree {
    root: Dependency,
    selected: State<Option<Dependency>>,
    working_directory: PathBuf,
}

#[async_trait]
impl Widget_trait for Target_tree {
    async fn layout(
        &mut self,
        render: vizual::Render,
        _theme: State<vizual::theme::Theme>,
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
                Target_tree_item::new(target, self.working_directory.clone())
                    .into_shared()
                    .into()
            })
            .collect::<Vec<_>>();

        let default_target = get_selector(
            targets
                .first()
                .expect("target tree must contain its root target"),
        );
        let menu = Menu::new(targets, default_target, render);

        Ok(vec![display!(menu)])
    }
}

fn display_relative_path(path: &Path) -> String {
    match path.as_os_str().is_empty() {
        true => ".".to_string(),
        false => path.display().to_string(),
    }
}
