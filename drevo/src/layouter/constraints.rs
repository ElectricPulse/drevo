use super::hitbox::Hitbox;
use crate::{config::MAXIMUM_LAYOUT_VALUE, constraint, geometry::Direction, id, layouter::Formula};
use color_eyre::eyre::Result;

#[cfg(test)]
mod tests;

pub fn prohibit_overlap(
    formula: &mut Formula,
    first: Hitbox,
    second: Hitbox,
    gap: f64,
) -> Result<()> {
    // The bits select exactly one separating direction:
    // 00: first is left of second; 01: second is left of first;
    // 10: first is above second; 11: second is above first.
    let horizontal = formula.binary_variable("prohibit-overlap-horizontal")?;
    let vertical = formula.binary_variable("prohibit-overlap-vertical")?;
    formula.constrain(
        id!(),
        constraint!(
            first.get_end_position(Direction::Horizontal) + gap
                <= second.get_start_position(Direction::Horizontal)
                    + MAXIMUM_LAYOUT_VALUE * (horizontal + vertical)
        ),
    )?;
    formula.constrain(
        id!(),
        constraint!(
            second.get_end_position(Direction::Horizontal) + gap
                <= first.get_start_position(Direction::Horizontal)
                    + MAXIMUM_LAYOUT_VALUE * (horizontal + (1 - vertical))
        ),
    )?;
    formula.constrain(
        id!(),
        constraint!(
            first.get_end_position(Direction::Vertical) + gap
                <= second.get_start_position(Direction::Vertical)
                    + MAXIMUM_LAYOUT_VALUE * ((1 - horizontal) + vertical)
        ),
    )?;
    formula.constrain(
        id!(),
        constraint!(
            second.get_end_position(Direction::Vertical) + gap
                <= first.get_start_position(Direction::Vertical)
                    + MAXIMUM_LAYOUT_VALUE * ((1 - horizontal) + (1 - vertical))
        ),
    )
}
