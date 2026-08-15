use crate::{
    component::{Children, context::Component_context},
    constraint,
    geometry::Direction,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::{State, Store},
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
    // TODO: Keep priority manual until there is a way to set it automatically.
    priority: usize,
}

impl Axis {
    pub fn new(direction: Direction, elements: Vec<Widget>) -> Self {
        Self {
            direction,
            elements,
            style: Style::default(),
            priority: 1,
        }
    }
}

#[async_trait]
impl Widget_trait for Axis {
    async fn layout(
        &mut self,
        render: crate::Render,
        theme: Store<Theme>,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::graphics::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let direction = self.direction;
        let mut elements = Vec::with_capacity(self.elements.len());

        for (index, element) in self.elements.iter().enumerate() {
            let container = Container::new(element.clone());
            let container = slots.set(index as u64, container).await?;
            elements.push(container);
        }

        if elements.len() >= 2 {
            let cross_direction = direction.flip();
            problem
                .minimize(hitbox.get_dimension(cross_direction), 0)
                .await?;

            let theme = theme.affect(render).await?;
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
