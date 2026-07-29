use async_trait::async_trait;
use color_eyre::eyre::Result;
use good_lp::{Expression, constraint};

use crate::{
    component::Child,
    hitbox::{Direction, Hitbox},
    layouter::{Problem_context, constraints::Objective},
    slot_manager::Slots,
    state::State,
    theme::Theme,
    widget::{Control, Focus_provider, Renderable, Widget_type},
};

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
    elements: Vec<Option<Child>>,
    pub style: Style,
    objective: Objective,
    // TODO: Keep priority manual until there is a way to set it automatically.
    priority: usize,
}

impl Control for Layout {}

impl Layout {
    fn get_elements(&self) -> Vec<&Child> {
        self.elements.iter().flatten().collect()
    }

    pub fn new(
        direction: Direction,
        elements: Vec<Option<Child>>,
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
impl Renderable for Layout {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: Hitbox,
        problem: Problem_context,
        _slots: &mut Slots,
    ) -> Result<Widget_type> {
        let elements = self.get_elements();

        let direction = self.direction;

        if elements.len() >= 1 {
            let flip_direction = direction.flip();

            for element in elements.iter() {
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

        if self.elements.len() >= 2 {
            let Style::Gap(gap) = self.style;

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
                    .apply(&problem, space, gap, gap, self.priority)
                    .await?;
            }
        }

        Ok(Widget_type::Visual(elements.into_iter().cloned().collect()))
    }
}
