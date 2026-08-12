use crate::{
    component::{Children, context::Component_context},
    geometry::Direction,
    layouter::{hitbox::Hitbox, objective::minimize},
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
    // TODO: Keep priority manual until there is a way to set it automatically.
    priority: usize,
}

impl Axis {
    pub fn new(direction: Direction, elements: Vec<Widget>) -> Self {
        Self {
            direction,
            elements,
            style: Style::default(),
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
        // direction always remains shared. Intermediate ends need solver variables, while every
        // later start is derived from the previous end and the shared gap expression below.
        let last_index = elements.len().saturating_sub(1);
        for (index, element) in elements.iter().enumerate() {
            let element = &mut element.lock().await?.hitbox;
            if index < last_index {
                element.point_end(
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

        if elements.len() >= 2 {
            let Axis_style::Gap(gap) = self.style.get(&theme);
            // One delta controls every gap belonging to this axis.
            let gap_delta = problem.add_delta("axis-gap-delta", self.priority).await?;
            let gap = gap * (1 - gap_delta);

            for pair in elements.windows(2) {
                let [previous, current] = pair else {
                    continue;
                };

                let previous_hitbox = previous.get_hitbox().await?;
                let current_hitbox = current.get_hitbox().await?;

                current_hitbox.point_start(
                    direction,
                    previous_hitbox.get_end_position(direction) + gap.clone(),
                );
            }
        }

        Ok(elements)
    }
}
