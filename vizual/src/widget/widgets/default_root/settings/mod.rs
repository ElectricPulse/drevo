use crate::macros::display;
use async_trait::async_trait;
use color_eyre::eyre::Result;
use lucide_icons::Icon as LucideIcon;

use super::{
    super::{
        block::{Block, BlockStyle},
        button::Button,
        container::Container,
        icon::Icon,
        layout::axis::Axis,
        menu::{Menu, MenuItem},
        positioning::anchor::{Anchor, Anchors},
        text::Text,
    },
    ThemeChoice,
};
use crate::{
    VizualMsg,
    component::Children,
    constraint,
    event::KeyCode,
    geometry::{Direction, Size},
    handlers::RetrieveHandler,
    layouter::hitbox::Hitbox,
    state::{State, Store},
    theme::{SystemTheme, Theme},
    utils::{get_next_index, get_previous_index},
    widget::{
        LayoutInput, RenderInput, Widget, WidgetTrait,
        custom_widget::CustomWidgetTrait,
        widgets::{paper::Paper, positioning::anchor::AnchorPosition},
    },
};

#[cfg(test)]
mod tests;

fn label(choice: ThemeChoice, system: SystemTheme) -> String {
    match choice {
        ThemeChoice::System => format!(
            "System ({})",
            match system {
                SystemTheme::Dark => "Dark",
                SystemTheme::Light => "Light",
            }
        ),
        ThemeChoice::Dark => "Dark".to_owned(),
        ThemeChoice::Light => "Light".to_owned(),
    }
}

impl ThemeChoice {
    const ALL: [Self; 3] = [Self::System, Self::Dark, Self::Light];

    fn resolve(self, theme: &Theme) -> Theme {
        match self {
            Self::System => theme.follow_system(),
            Self::Dark => theme.select(SystemTheme::Dark),
            Self::Light => theme.select(SystemTheme::Light),
        }
    }

    fn is_selected(self, theme: &Theme) -> bool {
        match self {
            Self::System => theme.follows_system(),
            Self::Dark => !theme.follows_system() && theme.mode() == SystemTheme::Dark,
            Self::Light => !theme.follows_system() && theme.mode() == SystemTheme::Light,
        }
    }
}

#[derive(Clone)]
struct Empty;

#[async_trait]
impl WidgetTrait for Empty {}

#[derive(Clone)]
struct ThemeMenuItem {
    choice: ThemeChoice,
}

#[async_trait]
impl RetrieveHandler<ThemeChoice> for ThemeMenuItem {
    async fn on_retrieve(&mut self) -> Result<State<ThemeChoice>> {
        Ok(self.choice.into())
    }
}

#[async_trait]
impl CustomWidgetTrait for ThemeMenuItem {
    type Payload = bool;

    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            theme,
            slots,
            ..
        }: LayoutInput<'_>,
        selected: bool,
    ) -> Result<Children> {
        let current_theme = theme.affect(relayout).await?;
        let preview_theme = self.choice.resolve(&current_theme);
        let mut text = Text::new(label(self.choice, current_theme.system()));
        let mut style = current_theme.specific.text.button;
        if !selected {
            style.color = current_theme.semantic.text.muted;
        }
        text.style.set(style);

        let border = preview_theme.specific.paper.block.border;
        let swatch = Block::new(
            Empty,
            BlockStyle {
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
                vertical: Some(AnchorPosition::Middle),
            },
        );

        let row = Axis::new(Direction::Horizontal, (text, swatch));
        Ok(vec![display!(row)])
    }
}

#[derive(Clone)]
pub(super) struct Settings {
    choice: Store<ThemeChoice>,
}

impl Settings {
    pub(super) fn new(choice: Store<ThemeChoice>) -> Self {
        Self { choice }
    }
}

#[derive(Clone)]
struct PositionedMenu {
    child: Widget,
    button: Hitbox,
}

impl PositionedMenu {
    fn new(child: impl WidgetTrait, button: Hitbox) -> Self {
        Self {
            child: child.as_any(),
            button,
        }
    }
}

#[async_trait]
impl WidgetTrait for PositionedMenu {
    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            theme,
            hitbox,
            problem,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        hitbox.make_independent();

        let horizontal_difference = hitbox.get_end_position(Direction::Horizontal)
            - self.button.get_end_position(Direction::Horizontal);
        // Keep the picker half an ordinary axis gap below the settings button while aligning
        // their right edges.
        let gap = theme.affect(relayout).await?.semantic.axis.gap;
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
impl WidgetTrait for Settings {
    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            theme,
            focus,
            slots,
            root,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let focused = focus.get();
        let choice = *self.choice.affect(relayout.clone()).await?;
        let current_theme = theme.affect(relayout.clone()).await?;

        if !choice.is_selected(&current_theme) {
            let resolved_theme = choice.resolve(&current_theme);
            drop(current_theme);
            theme.set(resolved_theme).await?;
        }

        let icon = Icon::new(LucideIcon::Settings);
        let mut button = Button::around(icon);
        button.highlighted = focused;

        let button = Anchor::new(
            button,
            Anchors {
                horizontal: Some(AnchorPosition::End),
                vertical: Some(AnchorPosition::Start),
            },
        );
        let button = display!(button);

        if !focused {
            return Ok(vec![button]);
        }

        let choices = ThemeChoice::ALL;

        let selected_index = choices
            .iter()
            .position(|candidate| *candidate == choice)
            .expect("selected theme choice must be present in the theme menu");

        let items = choices
            .into_iter()
            .map(|choice| -> MenuItem<ThemeChoice> { Box::new(ThemeMenuItem { choice }) })
            .collect::<Vec<_>>();

        let mut menu = Menu::new(items, selected_index).await?;
        menu.set_submitted(self.choice.clone()).await?;
        let menu = Paper::new(menu);

        let menu = PositionedMenu::new(menu, button.get_hitbox().await?);
        let menu = display!(menu);
        menu.lock().await?.logical = true;

        // Appending the dialog to the root component's children while maintaining settings as its logical layout parent is a neat way to bypass ancestor masking/clipping issues, ensuring it renders on top of the entire tree. In the future, instead of root, dedicated layer containers could be passed into layout.
        root.lock().await?.children.push(menu.clone());

        Ok(vec![button, menu])
    }

    async fn render(&mut self, RenderInput { .. }: RenderInput<'_, '_>) -> Result<()> {
        Ok(())
    }

    async fn on_key_press(&mut self, input: crate::widget::KeyPress<'_>) -> Result<VizualMsg> {
        let key = input.key;
        let relayout = input.relayout;
        let index = match key.code {
            KeyCode::ArrowUp | KeyCode::ArrowDown => {
                let choice = *self.choice.read().await?;
                ThemeChoice::ALL
                    .iter()
                    .position(|candidate| *candidate == choice)
                    .expect("selected theme choice must be present in the theme menu")
            }
            _ => return VizualMsg::none(),
        };
        let index = match key.code {
            KeyCode::ArrowUp => get_previous_index(ThemeChoice::ALL.len(), index),
            _ => get_next_index(ThemeChoice::ALL.len(), index),
        };
        self.choice.set(ThemeChoice::ALL[index]).await?;

        relayout.send();
        VizualMsg::none()
    }
}
