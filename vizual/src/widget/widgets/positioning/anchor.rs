use crate::macros::display;
use crate::{
    component::Children,
    constraint,
    geometry::Direction,
    id,
    layouter::{Formula, hitbox::Hitbox, priorities::POSITIONING},
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

    pub fn top_right(child: impl WidgetTrait) -> Self {
        Self::new(
            child,
            Anchors {
                horizontal: Some(AnchorPosition::End),
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
    fn anchor(
        formula: &mut Formula,
        parent: &Hitbox,
        hitbox: &mut Hitbox,
        position: Option<AnchorPosition>,
        direction: Direction,
    ) -> Result<()> {
        match position {
            Some(AnchorPosition::Start) => {
                hitbox.make_end_independent(direction);
                let end_margin =
                    parent.get_end_position(direction) - hitbox.get_end_position(direction);
                formula.constrain(id!(), constraint!(end_margin.clone() >= 0))?;
                formula.minimize(id!(), end_margin, POSITIONING)?;
            }
            Some(AnchorPosition::Middle) => {
                hitbox.make_start_independent(direction);
                hitbox.make_end_independent(direction);

                let start_margin =
                    hitbox.get_start_position(direction) - parent.get_start_position(direction);

                let end_margin =
                    parent.get_end_position(direction) - hitbox.get_end_position(direction);

                formula.constrain(id!(), constraint!(start_margin.clone() >= 0))?;
                formula.constrain(id!(), constraint!(start_margin.clone() == end_margin))?;
                formula.minimize(id!(), start_margin, POSITIONING)?;
            }
            Some(AnchorPosition::End) => {
                hitbox.make_start_independent(direction);
                let start_margin =
                    hitbox.get_start_position(direction) - parent.get_start_position(direction);
                formula.constrain(id!(), constraint!(start_margin.clone() >= 0))?;
                formula.minimize(id!(), start_margin, POSITIONING)?;
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
            formula,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        Self::anchor(
            formula,
            &parent,
            hitbox,
            self.anchors.horizontal,
            Direction::Horizontal,
        )?;

        Self::anchor(
            formula,
            &parent,
            hitbox,
            self.anchors.vertical,
            Direction::Vertical,
        )?;

        Ok(vec![display!(self.child.clone())])
    }
}
