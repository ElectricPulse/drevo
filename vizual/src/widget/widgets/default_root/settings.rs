use async_trait::async_trait;
use color_eyre::eyre::Result;
use lucide_icons::Icon as Lucide_icon;
use vizual_macros::display;

use super::{
    super::{
        block::{Block, Block_style},
        button::Button,
        container::Container,
        icon::Icon,
        layer::Layer,
        layout::axis::Axis,
        menu::{Menu, Shared_menu_item, get_selector},
        positioning::anchor::{Anchor, Anchors},
        text::Text,
    },
    Theme_choice,
};
use crate::{
    Render, Vizual_command, Vizual_msg,
    component::{Children, context::Component_context},
    constraint,
    event::Pointer_event,
    geometry::{Direction, Size},
    handlers::Retrieve_handler,
    layouter::{hitbox::Hitbox, objective::minimize},
    slot::manager::Slots,
    state::{State, Store},
    theme::{System_theme, Theme},
    widget::{
        Focus_provider, Widget, Widget_trait,
        custom_widget::Custom_widget_trait,
        widgets::{paper::Paper, positioning::anchor::Position},
    },
};

fn label(choice: Theme_choice, system: System_theme) -> String {
    match choice {
        Theme_choice::System => format!(
            "System ({})",
            match system {
                System_theme::Dark => "Dark",
                System_theme::Light => "Light",
            }
        ),
        Theme_choice::Dark => "Dark".to_owned(),
        Theme_choice::Light => "Light".to_owned(),
    }
}

impl Theme_choice {
    fn resolve(self, theme: &Theme) -> Theme {
        match self {
            Self::System => theme.follow_system(),
            Self::Dark => theme.select(System_theme::Dark),
            Self::Light => theme.select(System_theme::Light),
        }
    }

    fn is_selected(self, theme: &Theme) -> bool {
        match self {
            Self::System => theme.follows_system(),
            Self::Dark => !theme.follows_system() && theme.mode() == System_theme::Dark,
            Self::Light => !theme.follows_system() && theme.mode() == System_theme::Light,
        }
    }
}

#[derive(Clone)]
struct Empty;

#[async_trait]
impl Widget_trait for Empty {
    async fn layout(
        &mut self,
        _render: Render,
        _theme: Store<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::graphics::text::Text_context,
        _slots: &mut Slots,
    ) -> Result<Children> {
        Ok(Vec::new())
    }
}

#[derive(Clone)]
struct Theme_menu_item {
    choice: Theme_choice,
}

#[async_trait]
impl Retrieve_handler<Theme_choice> for Theme_menu_item {
    async fn on_retrieve(&mut self) -> Result<Theme_choice> {
        Ok(self.choice)
    }
}

#[async_trait]
impl Custom_widget_trait for Theme_menu_item {
    type Payload = bool;

    async fn layout(
        &mut self,
        render: Render,
        theme: Store<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::graphics::text::Text_context,
        slots: &mut Slots,
        selected: bool,
    ) -> Result<Children> {
        let current_theme = theme.affect(render).await?;
        let preview_theme = self.choice.resolve(&current_theme);
        let mut text = Text::new(label(self.choice, current_theme.system()));

        text.style.set(match selected {
            true => current_theme.specific.text.selected_subtitle,
            false => current_theme.specific.text.subtitle,
        });

        let border = preview_theme.specific.paper.block.border;
        let swatch = Block::new(
            Empty,
            Block_style {
                padding: 0.0,
                background: preview_theme.semantic.background,
                border,
                focused_border: border,
            },
        );
        let swatch = Container::new(swatch)
            .fixed_size(Size::new(current_theme.units.em, current_theme.units.em));
        let swatch = Anchor::new(
            swatch,
            Anchors {
                horizontal: None,
                vertical: Some(Position::Middle),
            },
        );

        let row = Axis::new(
            Direction::Horizontal,
            vec![Box::new(text), Box::new(swatch)],
        );
        Ok(vec![display!(row)])
    }
}

#[derive(Clone)]
pub(super) struct Settings {
    open: Store<bool>,
    choice: Store<Theme_choice>,
}

impl Settings {
    pub(super) fn new(open: Store<bool>, choice: Store<Theme_choice>) -> Self {
        Self { open, choice }
    }
}

#[derive(Clone)]
struct Positioned_menu {
    child: Widget,
    button: Hitbox,
}

impl Positioned_menu {
    fn new(child: impl Widget_trait, button: Hitbox) -> Self {
        Self {
            child: Box::new(child),
            button,
        }
    }
}

#[async_trait]
impl Widget_trait for Positioned_menu {
    async fn layout(
        &mut self,
        render: Render,
        theme: Store<Theme>,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::graphics::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let horizontal_difference = hitbox.get_end_position(Direction::Horizontal)
            - self.button.get_end_position(Direction::Horizontal);
        // Keep the picker half an ordinary axis gap below the settings button while aligning
        // their right edges.
        let gap = theme.affect(render).await?.semantic.axis.gap;
        let vertical_difference = hitbox.get_start_position(Direction::Vertical)
            - self.button.get_end_position(Direction::Vertical)
            - gap * 0.5;
        let horizontal_distance = problem
            .add_nonnegative_variable("settings-menu-horizontal-distance")
            .await?;
        let vertical_distance = problem
            .add_nonnegative_variable("settings-menu-vertical-distance")
            .await?;

        problem
            .constrain(constraint!(
                horizontal_distance.clone() >= horizontal_difference.clone()
            ))
            .await?;
        problem
            .constrain(constraint!(
                horizontal_distance.clone() >= -horizontal_difference
            ))
            .await?;
        problem
            .constrain(constraint!(
                vertical_distance.clone() >= vertical_difference.clone()
            ))
            .await?;
        problem
            .constrain(constraint!(
                vertical_distance.clone() >= -vertical_difference
            ))
            .await?;

        // The two nonnegative variables model the absolute coordinate differences, so their sum
        // is the Manhattan distance between the requested vertices.
        minimize(
            &mut *problem.lock().await?,
            horizontal_distance + vertical_distance,
            0,
        )?;

        Ok(vec![display!(self.child.clone())])
    }
}

// This is a piece of trash code

#[async_trait]
impl Widget_trait for Settings {
    async fn layout(
        &mut self,
        render: Render,
        theme: Store<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::graphics::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let choice = *self.choice.affect(render.clone()).await?;
        let current_theme = theme.affect(render.clone()).await?;
        if !choice.is_selected(&current_theme) {
            let resolved_theme = choice.resolve(&current_theme);
            drop(current_theme);
            *theme.write().await? = resolved_theme;
        }

        let open = *self.open.affect(render).await?;
        let icon = Icon::new(Lucide_icon::Settings);
        let mut button = Button::around(icon);
        button.highlighted = open;

        let button = Anchor::new(
            button,
            Anchors {
                horizontal: Some(Position::End),
                vertical: Some(Position::Start),
            },
        );
        let button = display!(button);

        if !open {
            return Ok(vec![button]);
        }

        let choices = [
            Theme_choice::System,
            Theme_choice::Dark,
            Theme_choice::Light,
        ];

        let selected_index = choices
            .iter()
            .position(|candidate| *candidate == choice)
            .expect("selected theme choice must be present in the theme menu");

        let items = choices
            .into_iter()
            .map(|choice| -> Shared_menu_item<Theme_choice> {
                Theme_menu_item { choice }.into_shared().into()
            })
            .collect::<Vec<_>>();

        let default_item = get_selector(&items[selected_index]);

        let mut menu = Menu::new(items, default_item);
        menu.set_submit_state(self.choice.clone());
        let menu = Paper::new(menu);

        let menu = Positioned_menu::new(menu, button.get_hitbox().await?);
        let menu = Layer::new(menu, 1);

        Ok(vec![button, display!(menu)])
    }

    async fn on_mouse_click(&mut self, _mouse: &Pointer_event) -> Result<Vizual_msg> {
        let open = *self.open.read().await?;
        *self.open.write().await? = !open;
        Vizual_msg::new(Vizual_command::Layout)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{
        App_problem, geometry::Size, graphics::text::Text_context, layouter::variables::Variables,
        render_manager::Render_manager, slot::Component_slot, theme::dark_theme,
        widget::widgets::root::Root,
    };

    #[test]
    fn system_label_includes_the_resolved_system_theme() {
        assert_eq!(
            label(Theme_choice::System, System_theme::Dark),
            "System (Dark)"
        );
        assert_eq!(
            label(Theme_choice::System, System_theme::Light),
            "System (Light)"
        );
    }

    #[tokio::test]
    async fn open_menu_position_solves() -> Result<()> {
        let render_manager = Render_manager::new();
        let render = render_manager.render;
        let theme = Store::new(dark_theme());
        let settings = Widget_trait::into_shared(Settings::new(
            Store::new(true),
            Store::new(Theme_choice::Dark),
        ));
        let root = Widget_trait::into_shared(Root::new(settings));
        let mut root_slot = Component_slot::new();
        let variables = Arc::new(Variables::new());
        let mut text_context = Text_context::new();
        let mut problem = App_problem::new(root, &mut root_slot, variables).await?;

        problem.layout(render, theme, &mut text_context).await?;
        let _ = problem.minimum_size().await?;
        let _ = problem.solve(Size::new(800.0, 600.0)).await?;

        Ok(())
    }
}
