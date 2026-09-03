use super::hitbox::Hitbox;
use crate::{
    component::Child, component::context::ComponentContext, config::MAXIMUM_LAYOUT_VALUE,
    constraint, geometry::Direction,
};
use color_eyre::eyre::Result;

/// Shrink-wraps each component edge around the corresponding child edges.
///
pub async fn shrink_wrap(
    problem: &ComponentContext,
    hitbox: Hitbox,
    children: &[Child],
    direction: Direction,
) -> Result<()> {
    if children.is_empty() {
        return Ok(());
    }

    let (start_bound_name, end_bound_name) = match direction {
        Direction::Horizontal => ("child_horizontal_start_bound", "child_horizontal_end_bound"),
        Direction::Vertical => ("child_vertical_start_bound", "child_vertical_end_bound"),
    };

    let mut child_hitboxes = Vec::with_capacity(children.len());
    for child in children {
        child_hitboxes.push(child.get_hitbox().await?);
    }

    for child_hitbox in child_hitboxes {
        problem
            .constrain(
                constraint!(
                    hitbox.get_start_position(direction)
                        <= child_hitbox.get_start_position(direction)
                )
                .set_name(start_bound_name.to_string()),
            )
            .await?;
        problem
            .constrain(
                constraint!(
                    hitbox.get_end_position(direction) >= child_hitbox.get_end_position(direction)
                )
                .set_name(end_bound_name.to_string()),
            )
            .await?;
    }

    problem
        .maximize(hitbox.get_start_position(direction), 0)
        .await?;
    problem
        .minimize(hitbox.get_end_position(direction), 0)
        .await?;

    Ok(())
}

pub async fn prohibit_overlap(
    problem: &ComponentContext,
    first: Hitbox,
    second: Hitbox,
    gap: f64,
) -> Result<()> {
    let first_left_of_second = problem
        .add_binary_variable("prohibit-overlap-first-left")
        .await?;
    let second_left_of_first = problem
        .add_binary_variable("prohibit-overlap-second-left")
        .await?;
    let first_above_second = problem
        .add_binary_variable("prohibit-overlap-first-above")
        .await?;
    let second_above_first = problem
        .add_binary_variable("prohibit-overlap-second-above")
        .await?;

    problem
        .constrain(constraint!(
            first_left_of_second.clone()
                + second_left_of_first.clone()
                + first_above_second.clone()
                + second_above_first.clone()
                == 1
        ))
        .await?;

    problem
        .constrain(constraint!(
            first.get_end_position(Direction::Horizontal) + gap
                <= second.get_start_position(Direction::Horizontal)
                    + MAXIMUM_LAYOUT_VALUE * (1 - first_left_of_second)
        ))
        .await?;
    problem
        .constrain(constraint!(
            second.get_end_position(Direction::Horizontal) + gap
                <= first.get_start_position(Direction::Horizontal)
                    + MAXIMUM_LAYOUT_VALUE * (1 - second_left_of_first)
        ))
        .await?;
    problem
        .constrain(constraint!(
            first.get_end_position(Direction::Vertical) + gap
                <= second.get_start_position(Direction::Vertical)
                    + MAXIMUM_LAYOUT_VALUE * (1 - first_above_second)
        ))
        .await?;
    problem
        .constrain(constraint!(
            second.get_end_position(Direction::Vertical) + gap
                <= first.get_start_position(Direction::Vertical)
                    + MAXIMUM_LAYOUT_VALUE * (1 - second_above_first)
        ))
        .await
}
