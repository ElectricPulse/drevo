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
        layout::axis::Axis,
        menu::{Menu, Menu_item},
        positioning::anchor::{Anchor, Anchors},
        text::Text,
    },
    Theme_choice,
};
use crate::{
    Vizual_command, Vizual_msg,
    component::Children,
    constraint,
    event::{Key_code, Key_event},
    geometry::{Direction, Size},
    handlers::Retrieve_handler,
    layouter::hitbox::Hitbox,
    state::{State, Store},
    theme::{System_theme, Theme},
    utils::{get_next_index, get_previous_index},
    widget::{
        Layout_input, Render_input, Widget, Widget_trait,
        custom_widget::Custom_widget_trait,
        widgets::{paper::Paper, positioning::anchor::Anchor_position},
    },
};

#[cfg(test)]
mod tests;

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
    const ALL: [Self; 3] = [Self::System, Self::Dark, Self::Light];

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
impl Widget_trait for Empty {}

#[derive(Clone)]
struct Theme_menu_item {
    choice: Theme_choice,
}

#[async_trait]
impl Retrieve_handler<Theme_choice> for Theme_menu_item {
    async fn on_retrieve(&mut self) -> Result<State<Theme_choice>> {
        Ok(self.choice.into())
    }
}

#[async_trait]
impl Custom_widget_trait for Theme_menu_item {
    type Payload = bool;

    async fn layout(
        &mut self,
        Layout_input {
            render,
            theme,
            slots,
            ..
        }: Layout_input<'_>,
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
                vertical: Some(Anchor_position::Middle),
            },
        );

        let row = Axis::new(
            Direction::Horizontal,
            (text, swatch),
        );
        Ok(vec![display!(row)])
    }
}

#[derive(Clone)]
pub(super) struct Settings {
    choice: Store<Theme_choice>,
}

impl Settings {
    pub(super) fn new(choice: Store<Theme_choice>) -> Self {
        Self { choice }
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
            child: child.as_any(),
            button,
        }
    }
}

#[async_trait]
impl Widget_trait for Positioned_menu {
    async fn layout(
        &mut self,
        Layout_input {
            render,
            theme,
            hitbox,
            problem,
            slots,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        hitbox.make_independent();

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
        problem
            .minimize(horizontal_distance + vertical_distance, 1)
            .await?;

        Ok(vec![display!(self.child.clone())])
    }
}

#[async_trait]
impl Widget_trait for Settings {
    async fn layout(
        &mut self,
        Layout_input {
            render,
            theme,
            focus,
            slots,
            root,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        let focused = focus.get();
        let choice = *self.choice.affect(render.clone()).await?;
        let current_theme = theme.affect(render.clone()).await?;

        if !choice.is_selected(&current_theme) {
            let resolved_theme = choice.resolve(&current_theme);
            drop(current_theme);
            theme.set(resolved_theme).await?;
        }

        let icon = Icon::new(Lucide_icon::Settings);
        let mut button = Button::around(icon);
        button.highlighted = focused;

        let button = Anchor::new(
            button,
            Anchors {
                horizontal: Some(Anchor_position::End),
                vertical: Some(Anchor_position::Start),
            },
        );
        let button = display!(button);

        if !focused {
            return Ok(vec![button]);
        }

        let choices = Theme_choice::ALL;

        let selected_index = choices
            .iter()
            .position(|candidate| *candidate == choice)
            .expect("selected theme choice must be present in the theme menu");

        let items = choices
            .into_iter()
            .map(|choice| -> Menu_item<Theme_choice> { Box::new(Theme_menu_item { choice }) })
            .collect::<Vec<_>>();

        let mut menu = Menu::new(items, selected_index).await?;
        menu.set_submitted(self.choice.clone()).await?;
        let menu = Paper::new(menu);

        let menu = Positioned_menu::new(menu, button.get_hitbox().await?);
        let menu = display!(menu);
        menu.lock().await?.logical = true;

        // Appending the dialog to the root component's children while maintaining settings as its logical layout parent is a neat way to bypass ancestor masking/clipping issues, ensuring it renders on top of the entire tree. In the future, instead of root, dedicated layer containers could be passed into layout.
        root.lock().await?.children.push(menu.clone());

        Ok(vec![button, menu])
    }

    async fn render(
        &mut self,
        Render_input { focus, .. }: Render_input<'_, '_>,
    ) -> Result<()> {
        focus.set_interactive(true);
        Ok(())
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        let index = match key.code {
            Key_code::Arrow_up | Key_code::Arrow_down => {
                let choice = *self.choice.read().await?;
                Theme_choice::ALL
                    .iter()
                    .position(|candidate| *candidate == choice)
                    .expect("selected theme choice must be present in the theme menu")
            }
            _ => return Vizual_msg::none(),
        };
        let index = match key.code {
            Key_code::Arrow_up => get_previous_index(Theme_choice::ALL.len(), index),
            _ => get_next_index(Theme_choice::ALL.len(), index),
        };
        self.choice.set(Theme_choice::ALL[index]).await?;

        Vizual_msg::new(Vizual_command::Layout)
    }
}
