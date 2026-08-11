//! In the case of grid it became clear that align and anchor (furthermore a&a) should not
//! position their content in regards to their own hitbox but their hitbox in regards to the parent
//! hitbox. It is because in grid it would require that by default an item occupy the full size of
//! the grid -> where the item would then choose if it wants to align or anchor. But how would one
//! then implement no overlap behaviour if the only thing grid can see is a&a elements that occupy
//! the whole grid - it cannot access the children of these a&a and hence cannot provide the no
//! overlap behaviour.
//!
//! Positioning widgets repoint the parts of their inherited hitbox which need independent layout.
//!
//! Under normal widget composition, these are the only widgets allowed to create or repoint
//! hitbox variables or to perform manual shrink-wrapping. The layout infrastructure in
//! `widgets::layout` and its internal `Container` are structural exceptions, `Linebreak` owns one
//! intrinsic edge variable, and the header's popup menu is the deliberate UI exception.

pub mod align;
pub mod anchor;
pub mod space;
