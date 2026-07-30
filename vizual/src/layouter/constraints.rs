use super::{expression::Expression, hitbox::Hitbox};
use crate::{
    component::Child, component::context::Component_context, config::MAXIMUM_LAYOUT_VALUE,
    constraint, geometry::Direction,
};
use color_eyre::eyre::Result;

/// Constrains a component to contain its visual children and minimizes its size on one axis
pub async fn shrink_wrap(
    problem: &Component_context,
    hitbox: Hitbox,
    children: &[Child],
    direction: Direction,
) -> Result<()> {
    let (start_bound_name, end_bound_name) = match direction {
        Direction::Horizontal => ("child_horizontal_start_bound", "child_horizontal_end_bound"),
        Direction::Vertical => ("child_vertical_start_bound", "child_vertical_end_bound"),
    };

    for child in children {
        let child_hitbox = child.get_hitbox().await?;
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
        .minimize(Expression::from(hitbox.get_dimension(direction)), 0)
        .await
}

pub async fn prohibit_overlap(
    problem: &Component_context,
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
            first_left_of_second + second_left_of_first + first_above_second + second_above_first
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
