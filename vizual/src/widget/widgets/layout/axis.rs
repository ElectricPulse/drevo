use crate::{
    component::Children,
    config::MAXIMUM_LAYOUT_VALUE,
    constraint,
    geometry::Direction,
    id,
    layouter::expression::Expression,
    style::Style,
    theme::Theme,
    widget::{IntoWidgets, LayoutInput, Widget, WidgetTrait},
};
use async_trait::async_trait;
use color_eyre::eyre::Result;

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
            formula,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let direction = self.direction;
        let mut elements = Vec::with_capacity(self.elements.len());

        if self.elements.is_empty() {
            return Ok(elements);
        }

        let cross = direction.flip();

        // First cross-axis system, kept for comparison:
        //
        // formula.minimize(id!(), hitbox.get_dimension(cross), 0)?;
        //
        // It uses an extra objective priority. The binary-selector system below is faster in
        // practice, even with one binary per child, because it avoids that priority level.
        let cross_start = hitbox.get_start_position(cross);
        let cross_end = hitbox.get_end_position(cross);
        let mut binaries = Vec::with_capacity(self.elements.len());

        for (index, element) in self.elements.iter().enumerate() {
            let element = slots.set(index as u64, element.clone()).await?;
            let mut element_hitbox = element.get_hitbox().await?;

            element_hitbox.make_start_independent(cross);
            element_hitbox.make_end_independent(cross);

            // Exactly one child is selected. Its nonnegative margins are forced to zero, so it
            // is the bounding child: its distance from this axis hitbox (its parent) is zero.
            let binary = formula.binary_variable(id!())?;
            binaries.push(binary);
            let start_margin = formula.variable(id!())?;
            let end_margin = formula.variable(id!())?;
            let start_adjuster = formula.bounded_variable(id!(), 0.0, f64::INFINITY, false)?;
            let end_adjuster = formula.bounded_variable(id!(), 0.0, f64::INFINITY, false)?;
            formula.constrain(id!(), constraint!(start_margin >= 0.0))?;
            formula.constrain(id!(), constraint!(end_margin >= 0.0))?;
            formula.constrain(
                id!(),
                constraint!((1.0 - binary) * MAXIMUM_LAYOUT_VALUE >= start_margin),
            )?;
            formula.constrain(
                id!(),
                constraint!((1.0 - binary) * MAXIMUM_LAYOUT_VALUE >= end_margin),
            )?;
            formula.constrain(
                id!(),
                constraint!(
                    cross_start.clone() - element_hitbox.get_start_position(cross)
                        == start_margin - start_adjuster
                ),
            )?;
            formula.constrain(
                id!(),
                constraint!(
                    cross_end.clone() - element_hitbox.get_end_position(cross)
                        == end_margin - end_adjuster
                ),
            )?;
            elements.push(element);
        }

        let binary_sum = binaries
            .into_iter()
            .fold(Expression::from(0.0), |sum, binary| sum + binary);
        formula.constrain(id!(), constraint!(binary_sum == 1.0))?;

        if elements.len() >= 2 {
            let theme = theme.affect(relayout).await?;
            let AxisStyle::Gap(gap) = self.style.get(&theme);
            // One delta controls every gap belonging to this axis.
            let gap_delta = formula.add_delta(id!(), self.priority)?;
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

                formula.constrain(
                    id!(),
                    constraint!(current_start - previous_end == gap.clone()),
                )?;
            }
        }

        Ok(elements)
    }
}
