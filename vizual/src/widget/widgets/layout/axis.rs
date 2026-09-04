use crate::{
    component::Children,
    constraint,
    geometry::Direction,
    id,
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
    /// Limits the cross-axis size to the space required by the elements.
    ///
    /// Enable this when elements position themselves along the cross axis, such as elements in a
    /// horizontal Axis using `Anchor::v_middle`. Their anchors constrain them within the Axis, and
    /// this objective then shrinks the Axis around them.
    ///
    /// Without it, the shared cross axis grows to fill the available parent space which sometimes isn't desired.
    /// Imagine a toolbar filled with vertically centered icons
    pub limit_cross: bool,
}

impl Axis {
    pub fn new(direction: Direction, elements: impl IntoWidgets) -> Self {
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

        if self.limit_cross {
            // If the performance heavy lexicographic priorites are ever used and priorities become expensive a second algorithm as described here
            // can be used to avoid creating a new priority at the cost of a creating child elements count amount of binary variables:
            // It selects one bounding element with a binary variable and forces that element's
            // cross-axis start and end margins to zero.
            formula.minimize(id!(), hitbox.get_dimension(cross), 0)?;
        }

        for (index, element) in self.elements.iter().enumerate() {
            let element = slots.set(index as u64, element.clone()).await?;
            elements.push(element);
        }

        // Second cross-axis system, kept for comparison. With objective blending enabled and a
        // carefully chosen weight, the priority-0 system above may be faster than this one.
        /*
        let cross_start = hitbox.get_start_position(cross);
        let cross_end = hitbox.get_end_position(cross);
        let mut binaries = Vec::with_capacity(self.elements.len());

        for element in &elements {


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
        }

        let binary_sum = binaries
            .into_iter()
            .fold(Expression::from(0.0), |sum, binary| sum + binary);
        formula.constrain(id!(), constraint!(binary_sum == 1.0))?;
        */

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
