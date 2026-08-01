/// A stable index into the layout variable registry.
///
/// Index `0` is unused, index `1` is permanently reserved for screen width, and index `2` is
/// permanently reserved for screen height. Dynamic variables always use index `3` or greater.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Variable {
    index: usize,
}

impl Variable {
    pub const fn new(index: usize) -> Self {
        Self { index }
    }

    pub fn index(self) -> usize {
        self.index
    }
}
