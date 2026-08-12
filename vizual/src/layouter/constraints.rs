use super::{hitbox::Hitbox, objective::minimize};
use crate::{
    component::Child, component::context::Component_context, config::MAXIMUM_LAYOUT_VALUE,
    constraint, geometry::Direction,
};
use color_eyre::eyre::Result;
use std::sync::Arc;

/// Shrink-wraps each component edge around the corresponding child edges.
///
/// A child handle which currently points to the component definition still receives a constraint:
/// a positioning widget can repoint that stable handle later in its own layout.
pub async fn shrink_wrap(
    problem: &Component_context,
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

    let mut constrained_start = false;
    let mut constrained_end = false;

    for child_hitbox in child_hitboxes {
        let child_start = child_hitbox.start_expression(direction);
        let hitbox_start = hitbox.start_expression(direction);
        if !Arc::ptr_eq(&child_start, &hitbox_start) {
            problem
                .constrain(
                    constraint!(
                        hitbox.get_start_position(direction)
                            <= child_hitbox.get_start_position(direction)
                    )
                    .set_name(start_bound_name.to_string()),
                )
                .await?;
            constrained_start = true;
        }
        let child_end = child_hitbox.end_expression(direction);
        let hitbox_end = hitbox.end_expression(direction);
        if !Arc::ptr_eq(&child_end, &hitbox_end) {
            problem
                .constrain(
                    constraint!(
                        hitbox.get_end_position(direction)
                            >= child_hitbox.get_end_position(direction)
                    )
                    .set_name(end_bound_name.to_string()),
                )
                .await?;
            constrained_end = true;
        }
    }

    if constrained_start {
        problem
            .lock()
            .await?
            .maximize(hitbox.get_start_position(direction), 0)?;
    }
    if constrained_end {
        minimize(
            &mut *problem.lock().await?,
            hitbox.get_end_position(direction),
            0,
        )?;
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layouter::variables::Variables;

    #[test]
    fn child_edges_are_distinct_handles_which_reference_parent_edges() {
        let variables = Variables::new();
        let parent = Hitbox::new(
            &variables,
            "parent".to_string(),
            "parent".to_string(),
            "test".to_string(),
        );
        let child = Hitbox::shared(&parent);
        let direction = Direction::Horizontal;
        assert!(!Arc::ptr_eq(
            &child.start_expression(direction),
            &parent.start_expression(direction)
        ));
        assert!(!Arc::ptr_eq(
            &child.end_expression(direction),
            &parent.end_expression(direction)
        ));

        assert_eq!(
            child
                .get_start_position(direction)
                .resolved()
                .unwrap()
                .single_variable()
                .unwrap(),
            parent.start_variable(direction)
        );
        assert_eq!(
            child
                .get_end_position(direction)
                .resolved()
                .unwrap()
                .single_variable()
                .unwrap(),
            parent.end_variable(direction)
        );
    }
}
