use super::{expression::Expression, hitbox::Hitbox};
use crate::{
    component::Child, component::context::Component_context, config::MAXIMUM_LAYOUT_VALUE,
    constraint, geometry::Direction,
};
use color_eyre::eyre::Result;

/// Shrink-wraps each unshared edge of a component around its unshared child edges.
///
/// An edge needs no containment constraint or objective only when it directly reuses the
/// corresponding parent edge variable. A child edge is skipped only when it directly reuses the
/// corresponding component edge variable.
pub async fn shrink_wrap(
    problem: &Component_context,
    hitbox: Hitbox,
    _parent: Hitbox,
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

    // A full child defines the component's normal bounds. Additional children can then overflow
    // those bounds as overlays instead of making the full child grow around them.
    let shrink_start = !child_hitboxes
        .iter()
        .any(|child| child.get_start_position(direction) == hitbox.get_start_position(direction));
    let shrink_end = !child_hitboxes
        .iter()
        .any(|child| child.end.get(direction) == hitbox.end.get(direction));

    let mut constrained_start = false;
    let mut constrained_end = false;

    for child_hitbox in child_hitboxes {
        if shrink_start
            && child_hitbox.get_start_position(direction) != hitbox.get_start_position(direction)
        {
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
        if shrink_end && child_hitbox.end.get(direction) != hitbox.end.get(direction) {
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
            .maximize(Expression::from(hitbox.get_start_position(direction)), 0)
            .await?;
    }
    if constrained_end {
        problem
            .minimize(hitbox.get_end_position(direction), 0)
            .await?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layouter::variables::Variables;

    #[test]
    fn only_corresponding_parent_edges_suppress_shrink_wrap() {
        let variables = Variables::new();
        let parent = Hitbox::new(
            &variables,
            "parent".to_string(),
            "parent".to_string(),
            "test".to_string(),
        );
        let mut child = Hitbox::new(
            &variables,
            "child".to_string(),
            "child".to_string(),
            "test".to_string(),
        );
        let direction = Direction::Horizontal;
        assert_ne!(
            child.get_start_position(direction),
            parent.get_start_position(direction)
        );
        assert_ne!(child.end.get(direction), parent.end.get(direction));

        child.start.x = parent.end.x;
        assert_ne!(
            child.get_start_position(direction),
            parent.get_start_position(direction)
        );

        child.start.x = parent.start.x;
        assert_eq!(
            child.get_start_position(direction),
            parent.get_start_position(direction)
        );

        child.end.x = parent.start.x;
        assert_ne!(child.end.get(direction), parent.end.get(direction));

        child.end.x = parent.end.x;
        assert_eq!(child.end.get(direction), parent.end.get(direction));
    }
}
