use crate::macros::display;
use crate::{
    component::Children,
    constraint,
    geometry::Direction,
    id,
    layouter::{
        Formula,
        expression::Expression,
        objective::Delta,
        priorities::{EXCESS_SPACE, INTRINSIC_SPACING},
    },
    widget::{LayoutInput, Widget, WidgetTrait},
};
use async_trait::async_trait;
use color_eyre::eyre::{Result, ensure};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Spaces {
    pub left: Option<f64>,
    pub right: Option<f64>,
    pub top: Option<f64>,
    pub bottom: Option<f64>,
}

impl Spaces {
    fn start(self, direction: Direction) -> Option<f64> {
        match direction {
            Direction::Horizontal => self.left,
            Direction::Vertical => self.top,
        }
    }

    fn end(self, direction: Direction) -> Option<f64> {
        match direction {
            Direction::Horizontal => self.right,
            Direction::Vertical => self.bottom,
        }
    }
}

/// Chooses how readily a [`Space`] may shrink when layout is constrained.
///
/// Padding is part of its child's visual surface and is retained ahead of margins. Margins are
/// outer spacing, so they give way before content and padding.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpaceMode {
    #[default]
    Padding,
    Margin,
}

impl SpaceMode {
    fn priority(self) -> usize {
        match self {
            Self::Padding => INTRINSIC_SPACING,
            Self::Margin => EXCESS_SPACE,
        }
    }
}

/// Adds padding or margin between a child and its parent.
///
/// TODO: Widget composition sometimes creates nested `Space` wrappers. Consider detecting and
/// combining adjacent spaces so redundant layout variables and component levels are optimized out.
#[derive(Clone)]
pub struct Space {
    child: Widget,
    spaces: Spaces,
    pub delta: Option<Delta>,
    pub minimum: f64,
    pub mode: SpaceMode,
}

impl Space {
    pub fn new(
        child: impl WidgetTrait,
        left: Option<f64>,
        right: Option<f64>,
        top: Option<f64>,
        bottom: Option<f64>,
        mode: SpaceMode,
    ) -> Self {
        Self {
            child: child.as_any(),
            spaces: Spaces {
                left,
                right,
                top,
                bottom,
            },
            delta: None,
            minimum: 0.0,
            mode,
        }
    }

    pub fn inline(child: impl WidgetTrait, value: f64, mode: SpaceMode) -> Self {
        Self::new(child, Some(value), Some(value), None, None, mode)
    }

    pub fn left(child: impl WidgetTrait, value: f64, mode: SpaceMode) -> Self {
        Self::new(child, Some(value), None, None, None, mode)
    }

    pub fn right(child: impl WidgetTrait, value: f64, mode: SpaceMode) -> Self {
        Self::new(child, None, Some(value), None, None, mode)
    }

    pub fn top(child: impl WidgetTrait, value: f64, mode: SpaceMode) -> Self {
        Self::new(child, None, None, Some(value), None, mode)
    }

    pub fn bottom(child: impl WidgetTrait, value: f64, mode: SpaceMode) -> Self {
        Self::new(child, None, None, None, Some(value), mode)
    }

    pub fn uniform(child: impl WidgetTrait, value: f64, mode: SpaceMode) -> Self {
        Self::new(
            child,
            Some(value),
            Some(value),
            Some(value),
            Some(value),
            mode,
        )
    }

    pub fn full(child: impl WidgetTrait, mode: SpaceMode) -> Self {
        Self::new(child, None, None, None, None, mode)
    }

    async fn expression(
        &self,
        formula: &mut Formula,
        target: f64,
        delta: &mut Option<Delta>,
    ) -> Result<Expression> {
        ensure!(target > 0.0, "space target must be greater than zero");
        let delta = match delta.as_ref() {
            Some(delta) => delta.clone(),
            None => {
                let new_delta = formula.add_delta(id!(), self.mode.priority())?;
                *delta = Some(new_delta.clone());
                new_delta
            }
        };
        let space: Expression = target * (1.0 - delta.clone());

        formula.constrain(id!(), constraint!(space.clone() >= self.minimum))?;
        formula.minimize(id!(), delta, self.mode.priority())?;

        Ok(space)
    }
}

#[async_trait]
impl WidgetTrait for Space {
    async fn layout(
        &mut self,
        LayoutInput {
            hitbox,
            parent,
            formula,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let spaces = self.spaces;
        let mut delta = self.delta.clone();

        for direction in [Direction::Horizontal, Direction::Vertical] {
            if let Some(space) = spaces.start(direction)
                && space != 0.0
            {
                let space = self.expression(formula, space, &mut delta).await?;
                hitbox.make_start_independent(direction);
                formula.constrain(
                    id!(),
                    constraint!(
                        hitbox.get_start_position(direction)
                            == parent.get_start_position(direction) + space
                    ),
                )?;
            }

            if let Some(space) = spaces.end(direction)
                && space != 0.0
            {
                let space = self.expression(formula, space, &mut delta).await?;
                hitbox.make_end_independent(direction);
                formula.constrain(
                    id!(),
                    constraint!(
                        hitbox.get_end_position(direction)
                            == parent.get_end_position(direction) - space
                    ),
                )?;
            }
        }

        Ok(vec![display!(self.child.clone())])
    }
}
