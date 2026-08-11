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
use vizual_macros::display;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Layout_style {
    Gap(f64),
    // TODO: Implement Start, Center, End, Space_between, Space_around, and Space_evenly.
}

impl From<Theme> for Layout_style {
    fn from(theme: Theme) -> Self {
        Self::Gap(theme.semantic.layout.gap)
    }
}

#[derive(Clone)]
pub struct Layout {
    direction: Direction,
    elements: Vec<Widget>,
    pub style: Style<Layout_style>,
    objective: Objective,
    // TODO: Keep priority manual until there is a way to set it automatically.
    priority: usize,
}

impl Layout {
    pub fn new<Element>(direction: Direction, elements: Vec<Element>) -> Self
    where
        Element: Widget_trait,
    {
        Self {
            direction,
            elements: elements
                .into_iter()
                .map(|element| Box::new(element) as Widget)
                .collect(),
            style: Style::default(),
            objective: Objective::Minimize_delta,
            priority: 2,
        }
    }
}

#[async_trait]
impl Widget_trait for Layout {
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
                    problem.make_independent_variable("layout-item-start"),
                );
            }
            if index < last_index {
                element.end.point_to_variable(
                    direction,
                    problem.make_independent_variable("layout-item-end"),
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
            let Layout_style::Gap(gap) = self.style.get(&theme);
            let gap_delta = problem.add_delta("layout-gap-delta", self.priority).await?;

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
