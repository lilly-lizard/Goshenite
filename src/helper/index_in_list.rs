/// Describes an `index` that is guarenteed to be valid in a list with `length` elements.
///
/// Ordering is performed on the index (ignores the length).
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct IndexInList {
    pub index: Option<usize>,
    pub length: usize,
}

impl PartialOrd for IndexInList {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.index.partial_cmp(&other.index)
    }
}

impl Ord for IndexInList {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.index.cmp(&other.index)
    }
}

impl Default for IndexInList {
    fn default() -> Self {
        Self {
            index: None,
            length: 0,
        }
    }
}
