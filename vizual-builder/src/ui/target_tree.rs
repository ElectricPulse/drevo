use async_trait::async_trait;
use color_eyre::Result;
use derive_new::new;
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
        selected: bool,
    ) -> Result<Children> {
        let metadata = self.target.get_metadata().await?;

        let icon = Icon::new(metadata.status.get_icon(), (&self.theme).into());
        let icon = Anchor::center(display!(icon));

        let text = Text::new(metadata.name, (&self.theme).into());

        let row = Layout::new(
            Direction::Horizontal,
            vec![display!(text), display!(icon)],
            (&self.theme).into(),
            Objective::default(),
            2,
        );
        let row = Full::width(display!(row));

        Ok(vec![display!(row)])
    }
}

#[derive(new)]
pub struct Target_tree {
    root: Dependency,
    selected: State<Option<Dependency>>,
    theme: State<Theme>,
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
                Target_tree_item::new(target, self.theme.clone()).into_shared()
            })
            .collect::<Vec<_>>();

        let default_target = get_selector(
            targets
                .first()
                .expect("target tree must contain its root target"),
        );
        let menu = Menu::new(targets, default_target, self.theme.clone());
        let menu = Full::new(display!(menu));

        Ok(vec![display!(menu)])
    }
}
