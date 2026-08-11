use crate::{
    component::{Children, context::Component_context},
    constraint,
    geometry::Direction,
    layouter::{
        expression::Expression,
        hitbox::Hitbox,
        objective::{Objective, minimize},
    },
    slot::manager::Slots,
    state::State,
    style::Style,
    theme::Theme,
    widget::{Focus_provider, Widget, Widget_trait, widgets::container::Container},
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
    objective: Objective,
    // TODO: Keep priority manual until there is a way to set it automatically.
    priority: usize,
}

impl Axis {
    pub fn new(direction: Direction, elements: Vec<Widget>) -> Self {
        Self {
            direction,
            elements,
            style: Style::default(),
            objective: Objective::Minimize_delta,
            priority: 2,
        }
    }
}

#[async_trait]
impl Widget_trait for Axis {
    async fn layout(
        &mut self,
        _render: crate::Render,
        theme: State<Theme>,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let direction = self.direction;
        let mut elements = Vec::with_capacity(self.elements.len());
        for (index, element) in self.elements.iter().enumerate() {
            let container = Container::new(element.clone());
            let container = slots.set(index as u64, container).await?;
            elements.push(container);
        }

        // A container around each item lets Align, Anchor, and Space position that item. The cross
        // direction always remains shared, so only the main-axis start/end edges need independence.
        let last_index = elements.len().saturating_sub(1);
        for (index, element) in elements.iter().enumerate() {
            let element = &mut element.lock().await?.hitbox;
            if index > 0 {
                element.start.point_to_variable(
                    direction,
                    problem.make_independent_variable("axis-item-start"),
                );
            }
            if index < last_index {
                element.end.point_to_variable(
                    direction,
                    problem.make_independent_variable("axis-item-end"),
                );
            }
        }

        let cross_direction = direction.flip();
        minimize(
            &mut *problem.lock().await?,
            hitbox.get_dimension(cross_direction),
            0,
        )?;

        let gap_delta = problem.add_delta("axis-gap-delta", self.priority).await?;

        if elements.len() >= 2 {
            let Axis_style::Gap(gap) = self.style.get(&theme);

            for pair in elements.windows(2) {
                let [previous, current] = pair else {
                    continue;
                };

                let previous_hitbox = previous.get_hitbox().await?;
                let current_hitbox = current.get_hitbox().await?;

                let space = Expression::from(
                    current_hitbox.get_start_position(direction)
                        - previous_hitbox.get_end_position(direction),
                );
                problem.constrain(constraint!(space.clone() >= 0)).await?;
                self.objective
                    .apply(&problem, space, gap, gap_delta.clone(), self.priority)
                    .await?;
            }
        }

        Ok(elements)
    }
}
