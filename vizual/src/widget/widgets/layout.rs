use crate::{
    component::{Child, Children, context::Component_context},
    constraint,
    geometry::Direction,
    layouter::{expression::Expression, hitbox::Hitbox, objective::Objective},
    slot::manager::Slots,
    state::State,
    theme::Theme,
    widget::{Control, Focus_provider, Widget_trait},
};
use async_trait::async_trait;
use color_eyre::eyre::Result;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Style {
    Gap(f64),
    // TODO: Implement Start, Center, End, Space_between, Space_around, and Space_evenly.
}

impl Style {
    pub fn default(theme: State<Theme>) -> Self {
        Self::Gap(theme.load().semantic.layout.gap)
    }
}

pub struct Layout {
    direction: Direction,
    elements: Vec<Child>,
    pub style: Style,
    objective: Objective,
    // TODO: Keep priority manual until there is a way to set it automatically.
    priority: usize,
}



impl Layout {
    pub fn new(
        direction: Direction,
        elements: Vec<Child>,
        style: Style,
        objective: Objective,
        priority: usize,
    ) -> Self {
        Self {
            direction,
            elements,
            style,
            objective,
            priority,
        }
    }
}

#[async_trait]
impl Widget_trait for Layout {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        _slots: &mut Slots,
    ) -> Result<Children> {
        let direction = self.direction;

        if self.elements.len() >= 1 {
            let flip_direction = direction.flip();

            for element in self.elements.iter() {
                let element_hitbox = element.get_hitbox().await?;

                // left align
                problem
                    .constrain(constraint!(
                        hitbox.get_start_position(flip_direction)
                            == element_hitbox.get_start_position(flip_direction)
                    ))
                    .await?;
            }
        }

        match (self.elements.first(), self.elements.last()) {
            (Some(first), Some(last)) => {
                let first_hitbox = first.get_hitbox().await?;
                let last_hitbox = last.get_hitbox().await?;
                problem
                    .constrain(constraint!(
                        hitbox.get_start_position(direction)
                            == first_hitbox.get_start_position(direction)
                    ))
                    .await?;
                problem
                    .constrain(constraint!(
                        hitbox.get_end_position(direction)
                            == last_hitbox.get_end_position(direction)
                    ))
                    .await?;
            }
            _ => {
                problem
                    .constrain(constraint!(hitbox.get_dimension(direction) == 0))
                    .await?;
            }
        }

        if self.elements.len() >= 2 {
            let Style::Gap(gap) = self.style;
            let gap_delta = match self.objective {
                Objective::Minimize_difference => {
                    Some(problem.add_delta("layout-gap-delta", self.priority).await?)
                }
                Objective::Maximize | Objective::Minimize => None,
            };

            for pair in self.elements.windows(2) {
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
                    .apply(&problem, space, gap, gap_delta, self.priority)
                    .await?;
            }
        }

        Ok(self.elements.clone())
    }
}
