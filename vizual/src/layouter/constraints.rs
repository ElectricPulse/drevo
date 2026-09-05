use super::{hitbox::Hitbox, priorities::SHRINK_WRAP};
use crate::{
    component::Child, config::MAXIMUM_LAYOUT_VALUE, constraint, geometry::Direction, id,
    layouter::Formula,
};
use color_eyre::eyre::Result;

/// Shrink-wraps each component edge around the corresponding child edges.
pub async fn shrink_wrap(
    formula: &mut Formula,
    hitbox: Hitbox,
    children: &[Child],
    direction: Direction,
) -> Result<()> {
    if children.is_empty() {
        return Ok(());
    }

    let mut child_hitboxes = Vec::with_capacity(children.len());
    for child in children {
        child_hitboxes.push(child.get_hitbox().await?);
    }
    for child_hitbox in child_hitboxes {
        formula.constrain(
            id!(),
            constraint!(
                hitbox.get_start_position(direction) <= child_hitbox.get_start_position(direction)
            ),
        )?;
        formula.constrain(
            id!(),
            constraint!(
                hitbox.get_end_position(direction) >= child_hitbox.get_end_position(direction)
            ),
        )?;
    }
    formula.maximize(id!(), hitbox.get_start_position(direction), SHRINK_WRAP)?;
    formula.minimize(id!(), hitbox.get_end_position(direction), SHRINK_WRAP)?;
    Ok(())
}

pub fn prohibit_overlap(
    formula: &mut Formula,
    first: Hitbox,
    second: Hitbox,
    gap: f64,
) -> Result<()> {
    let first_left = formula.binary_variable("prohibit-overlap-first-left")?;
    let second_left = formula.binary_variable("prohibit-overlap-second-left")?;
    let first_above = formula.binary_variable("prohibit-overlap-first-above")?;
    let second_above = formula.binary_variable("prohibit-overlap-second-above")?;
    formula.constrain(
        id!(),
        constraint!(first_left + second_left + first_above + second_above == 1),
    )?;
    formula.constrain(
        id!(),
        constraint!(
            first.get_end_position(Direction::Horizontal) + gap
                <= second.get_start_position(Direction::Horizontal)
                    + MAXIMUM_LAYOUT_VALUE * (1 - first_left)
        ),
    )?;
    formula.constrain(
        id!(),
        constraint!(
            second.get_end_position(Direction::Horizontal) + gap
                <= first.get_start_position(Direction::Horizontal)
                    + MAXIMUM_LAYOUT_VALUE * (1 - second_left)
        ),
    )?;
    formula.constrain(
        id!(),
        constraint!(
            first.get_end_position(Direction::Vertical) + gap
                <= second.get_start_position(Direction::Vertical)
                    + MAXIMUM_LAYOUT_VALUE * (1 - first_above)
        ),
    )?;
    formula.constrain(
        id!(),
        constraint!(
            second.get_end_position(Direction::Vertical) + gap
                <= first.get_start_position(Direction::Vertical)
                    + MAXIMUM_LAYOUT_VALUE * (1 - second_above)
        ),
    )
}
