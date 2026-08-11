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
        // TODO: A separate Container for every item wastes performance, but without its independent
        // hitbox an Align or Anchor cannot be placed directly in a Layout.
        let mut elements = Vec::with_capacity(self.elements.len());
        for (index, element) in self.elements.iter().enumerate() {
            let element = Container::new(element.clone());
            elements.push(slots.set(index as u64, element).await?);
        }

        let cross_direction = direction.flip();
        for element in &elements {
            element
                .share_start(*hitbox, &problem, cross_direction)
                .await?;
            element
                .share_end(*hitbox, &problem, cross_direction)
                .await?;
        }
        minimize(
            &mut *problem.lock().await?,
            hitbox.get_dimension(cross_direction),
            0,
        )?;

        match (elements.first(), elements.last()) {
            (Some(first), Some(last)) => {
                first.share_start(*hitbox, &problem, direction).await?;
                last.share_end(*hitbox, &problem, direction).await?;
            }
            _ => {}
        }

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
                    .apply(&problem, space, gap, gap_delta, self.priority)
                    .await?;
            }
        }

        Ok(elements)
    }
}
