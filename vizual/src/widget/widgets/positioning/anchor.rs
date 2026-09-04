use crate::macros::display;
use crate::{
    component::Children,
    constraint,
    geometry::Direction,
    id,
    layouter::{Formula, hitbox::Hitbox},
    widget::{LayoutInput, Widget, WidgetTrait},
};
use async_trait::async_trait;
use color_eyre::Result;

#[derive(Clone, Copy)]
pub enum AnchorPosition {
    Start,
    Middle,
    End,
}

#[derive(Clone)]
pub struct Anchors {
    pub horizontal: Option<AnchorPosition>,
    pub vertical: Option<AnchorPosition>,
}

#[derive(Clone)]
pub struct Anchor {
    child: Widget,
    anchors: Anchors,
}

impl Anchor {
    pub fn new(child: impl WidgetTrait, anchors: Anchors) -> Self {
        Self {
            child: child.as_any(),
            anchors,
        }
    }

    pub fn left(child: impl WidgetTrait) -> Self {
        Self::new(
            child,
            Anchors {
                horizontal: Some(AnchorPosition::Start),
                vertical: None,
            },
        )
    }

    pub fn right(child: impl WidgetTrait) -> Self {
        Self::new(
            child,
            Anchors {
                horizontal: Some(AnchorPosition::End),
                vertical: None,
            },
        )
    }

    pub fn top(child: impl WidgetTrait) -> Self {
        Self::new(
            child,
            Anchors {
                horizontal: None,
                vertical: Some(AnchorPosition::Start),
            },
        )
    }

    pub fn top_left(child: impl WidgetTrait) -> Self {
        Self::new(
            child,
            Anchors {
                horizontal: Some(AnchorPosition::Start),
                vertical: Some(AnchorPosition::Start),
            },
        )
    }

    pub fn middle(child: impl WidgetTrait) -> Self {
        Self::new(
            child,
            Anchors {
                horizontal: Some(AnchorPosition::Middle),
                vertical: Some(AnchorPosition::Middle),
            },
        )
    }

    pub fn v_middle(child: impl WidgetTrait) -> Self {
        Self::new(
            child,
            Anchors {
                horizontal: None,
                vertical: Some(AnchorPosition::Middle),
            },
        )
    }

    /// Applies the selected anchor to this hitbox within its parent.
    async fn anchor(
        formula: &mut Formula,
        parent: &Hitbox,
        hitbox: &mut Hitbox,
        position: Option<AnchorPosition>,
        direction: Direction,
    ) -> Result<()> {
        match position {
            Some(AnchorPosition::Start) => {
                hitbox.make_end_independent(direction);
                formula.constrain(
                    id!(),
                    constraint!(
                        hitbox.get_end_position(direction) <= parent.get_end_position(direction)
                    ),
                )?;
            }
            Some(AnchorPosition::Middle) => {
                hitbox.make_start_independent(direction);
                hitbox.make_end_independent(direction);

                formula.constrain(
                    id!(),
                    constraint!(
                        hitbox.get_start_position(direction)
                            >= parent.get_start_position(direction)
                    ),
                )?;
                formula.constrain(
                    id!(),
                    constraint!(
                        hitbox.get_end_position(direction) <= parent.get_end_position(direction)
                    ),
                )?;

                let start_margin =
                    hitbox.get_start_position(direction) - parent.get_start_position(direction);
                let end_margin =
                    parent.get_end_position(direction) - hitbox.get_end_position(direction);

                formula.constrain(id!(), constraint!(start_margin.clone() == end_margin))?;
            }
            Some(AnchorPosition::End) => {
                hitbox.make_start_independent(direction);
                formula.constrain(
                    id!(),
                    constraint!(
                        hitbox.get_start_position(direction)
                            >= parent.get_start_position(direction)
                    ),
                )?;
            }
            None => {}
        }

        Ok(())
    }
}

#[async_trait]
impl WidgetTrait for Anchor {
    async fn layout(
        &mut self,
        LayoutInput {
            hitbox,
            parent,
            formula: problem,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        Self::anchor(
            problem,
            &parent,
            hitbox,
            self.anchors.horizontal,
            Direction::Horizontal,
        )
        .await?;

        Self::anchor(
            problem,
            &parent,
            hitbox,
            self.anchors.vertical,
            Direction::Vertical,
        )
        .await?;

        Ok(vec![display!(self.child.clone())])
    }
}
