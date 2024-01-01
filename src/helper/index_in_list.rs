use super::more_errors::CollectionError;

/// Describes an `index` that is guarenteed to be valid in a list with `length` elements.
///
/// Ordering is performed on the index (ignores the length).
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct IndexInList {
    index: Option<usize>,
    length: usize,
}

impl IndexInList {
    pub fn new(index: Option<usize>, length: usize) -> Self {
        Self { index, length }
    }

    pub fn set_index(&mut self, new_index: Option<usize>) -> Result<(), CollectionError> {
        let Some(some_new_index) = new_index else {
            self.index = None;
            return Ok(());
        };

        if some_new_index < self.length {
            self.index = new_index;
            return Ok(());
        } else {
            return Err(CollectionError::OutOfBounds {
                index: some_new_index,
                size: self.length,
            });
        }
    }

    /// If the new length is less than or equal to the current index, the index is set to `None`.
    pub fn set_length(&mut self, new_length: usize) {
        if let Some(some_index) = self.index {
            if some_index >= new_length {
                // old index is invalid
                self.index = None;
            }
        }
        self.length = new_length;
    }

    pub fn index(&self) -> Option<usize> {
        self.index
    }

    pub fn length(&self) -> usize {
        self.length
    }
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
