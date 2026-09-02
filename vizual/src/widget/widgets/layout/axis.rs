use crate::{
    component::Children,
    constraint,
    geometry::Direction,
    style::Style,
    theme::Theme,
    widget::{Into_widgets, Layout_input, Widget, Widget_trait, widgets::container::Container},
};
use async_trait::async_trait;
use color_eyre::eyre::Result;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Axis_style {
    Gap(f64),
    // TODO: Implement Start, Center, End, Space_between, Space_around, and Space_evenly.
}

impl From<Theme> for Axis_style {
    fn from(theme: Theme) -> Self {
        Self::Gap(theme.semantic.axis.gap)
    }
}

#[derive(Clone)]
pub struct Axis {
    direction: Direction,
    elements: Vec<Widget>,
    pub style: Style<Axis_style>,
    // TODO: Keep priority manual until there is a way to set it automatically.
    priority: usize,
    pub limit_cross: bool,
}

impl Axis {
    pub fn new(direction: Direction, elements: impl Into_widgets) -> Self {
        Self {
            direction,
            elements: elements.into(),
            style: Style::default(),
            priority: 1,
            limit_cross: false,
        }
    }
}

#[async_trait]
impl Widget_trait for Axis {
    async fn layout(
        &mut self,
        Layout_input {
            relayout,
            theme,
            hitbox,
            problem,
            slots,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        let direction = self.direction;
        let mut elements = Vec::with_capacity(self.elements.len());

        for (index, element) in self.elements.iter().enumerate() {
            let container = Container::new(element.clone());
            let container = slots.set(index as u64, container).await?;
            elements.push(container);
        }

        if self.limit_cross {
            let cross_direction = direction.flip();
            problem
                .minimize(hitbox.get_dimension(cross_direction), 0)
                .await?;
        }

        if elements.len() >= 2 {
            let theme = theme.affect(relayout).await?;
            let Axis_style::Gap(gap) = self.style.get(&theme);
            // One delta controls every gap belonging to this axis.
            let gap_delta = problem.add_delta("axis-gap-delta", self.priority).await?;
            let gap = gap * (1 - gap_delta);

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

                problem
                    .constrain(constraint!(current_start - previous_end == gap.clone()))
                    .await?;
            }
        }

        Ok(elements)
    }
}
