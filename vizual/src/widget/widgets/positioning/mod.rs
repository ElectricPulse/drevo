//! In the case of grid it became clear that align and anchor (furthermore a&a) should not
//! position their content in regards to their own hitbox but their hitbox in regards to the parent
//! hitbox. It is because in grid it would require that by default an item occupy the full size of
//! the grid -> where the item would then choose if it wants to align or anchor. But how would one
//! then implement no overlap behaviour if the only thing grid can see is a&a elements that occupy
//! the whole grid - it cannot access the children of these a&a and hence cannot provide the no
//! overlap behaviour.
//!
//! TODO: Currently, rendering these widgets using `display!()` internally uses `Full`, which does
//! `hitbox = parent_hitbox` internally and negates any effect these widgets have.

pub mod align;
pub mod anchor;
pub mod space;
