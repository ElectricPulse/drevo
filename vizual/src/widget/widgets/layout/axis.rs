use crate::{
    component::Children,
    constraint,
    geometry::Direction,
    id,
    style::Style,
    theme::Theme,
    widget::{IntoWidgets, LayoutInput, Widget, WidgetTrait, widgets::container::Container},
};
use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AxisStyle {
    Gap(f64),
    // TODO: Implement Start, Center, End, SpaceBetween, SpaceAround, and SpaceEvenly.
}

impl From<Theme> for AxisStyle {
    fn from(theme: Theme) -> Self {
        Self::Gap(theme.semantic.axis.gap)
    }
}

#[derive(Clone)]
pub struct Axis {
    direction: Direction,
    elements: Vec<Widget>,
    pub style: Style<AxisStyle>,
    // TODO: Keep priority manual until there is a way to set it automatically.
    priority: usize,
}

impl Axis {
    pub fn new(direction: Direction, elements: impl IntoWidgets) -> Self {
        Self {
            direction,
            elements: elements.into(),
            style: Style::default(),
            priority: 1,
        }
    }
}

#[async_trait]
impl WidgetTrait for Axis {
    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            theme,
            hitbox,
            formula: problem,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let direction = self.direction;
        let mut elements = Vec::with_capacity(self.elements.len());

        let cross = direction.flip();
        let cross_start = hitbox.get_start_position(cross);
        let cross_end = hitbox.get_end_position(cross);

        for (index, element) in self.elements.iter().enumerate() {
            let element = slots.set(index as u64, element).await?;
            let mut element_hitbox = element.get_hitbox().await?;

            element_hitbox.make_start_independent(cross);
            element_hitbox.make_end_independent(cross);

            elements.push(container);
        }

        if elements.len() >= 2 {
            let theme = theme.affect(relayout).await?;
            let AxisStyle::Gap(gap) = self.style.get(&theme);
            // One delta controls every gap belonging to this axis.
            let gap_delta = problem.add_delta(id!(), self.priority)?;
            let gap: crate::layouter::expression::Expression = gap * (1.0 - gap_delta);

            for pair in elements.windows(2) {
                let [previous, current] = pair else {
                    continue;
                };

                let previous_end = {
                    let mut previous = previous.lock().await?;
                    previous.hitbox.make_end_independent(direction);
                    previous.hitbox.get_end_position(direction)
                };
                let current_start = {
                    let mut current = current.lock().await?;
                    current.hitbox.make_start_independent(direction);
                    current.hitbox.get_start_position(direction)
                };

                problem.constrain(
                    id!(),
                    constraint!(current_start - previous_end == gap.clone()),
                )?;
            }
        }

        Ok(elements)
    }
}
