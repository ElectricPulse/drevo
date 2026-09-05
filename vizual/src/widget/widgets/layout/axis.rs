use crate::{
    component::Children,
    constraint,
    geometry::Direction,
    id,
    layouter::priorities::{CROSS_AXIS_LIMIT, GAP},
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
    /// Limits the cross-axis size to the space required by the elements.
    ///
    /// Enable this when elements position themselves along the cross axis, such as elements in a
    /// horizontal Axis using `Anchor::v_middle`. Their anchors constrain them within the Axis, and
    /// this objective then shrinks the Axis around them.
    ///
    /// Without it, the shared cross axis grows to fill the available parent space which sometimes isn't desired.
    /// Imagine a toolbar filled with vertically centered icons
    limit_cross: bool,
}

impl Axis {
    pub fn new(direction: Direction, elements: impl IntoWidgets) -> Self {
        Self {
            direction,
            elements: elements.into(),
            style: Style::default(),
            limit_cross: true,
        }
    }

    // Lets the cross axis grow to fill its available parent space.
    pub fn free_cross(mut self) -> Self {
        self.limit_cross = false;
        self
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
            //  For each child element it creates a binary variable that means true == this is the biggest element, false == this isn't the biggest element
            //  Then it sets the sum of the binaries to be 1 so that only one element is biggest
            //  Then for each element it says that (1 - binary) * very_big_number >= margin between element edge and hitbox edge
            //  It does that constraint for both start and end edge of the cross direction
            //  Which means that both edges need for every element to be independent
            //  Then you say that all other elements must be inside the hitbox
            //  Anchor::middle still works in this system because as of the time of writing this comment it calculates the start and end margin
            //  as a difference of its own hitbox (which in this case it would duplicitly make independent) and the parent hitbox (which would still be axis)
            formula.minimize(id!(), hitbox.get_dimension(cross), CROSS_AXIS_LIMIT)?;
        }

        for (index, element) in self.elements.iter().enumerate() {
            let element = slots.set(index as u64, element.clone()).await?;
            elements.push(element);
        }

        if elements.len() >= 2 {
            let theme = theme.affect(relayout).await?;
            let AxisStyle::Gap(gap) = self.style.get(&theme);
            // One delta controls every gap belonging to this axis.
            let gap_delta = formula.add_delta(id!(), GAP)?;
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
