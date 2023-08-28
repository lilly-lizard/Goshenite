use std::{
    collections::BTreeSet,
    error, fmt,
    sync::atomic::{AtomicUsize, Ordering},
};

pub type UniqueId = usize;

pub struct UniqueIdGen {
    counter: AtomicUsize,
    recycled_ids: BTreeSet<UniqueId>,
}
impl UniqueIdGen {
    pub const fn new() -> Self {
        Self {
            counter: AtomicUsize::new(1),
            recycled_ids: BTreeSet::new(),
        }
    }

    pub fn new_id(&mut self) -> Result<UniqueId, UniqueIdError> {
        // prefer recycling ids
        if let Some(new_id) = self.recycled_ids.pop_first() {
            return Ok(new_id);
        }

        let new_id = self.counter.fetch_add(1, Ordering::Relaxed);
        if new_id == UniqueId::MAX {
            // means the fetch_add new id will wrap around making the ids not unique!
            return Err(UniqueIdError::MaxReached);
        }
        Ok(new_id)
    }

    pub fn recycle_id(&mut self, old_id: UniqueId) -> Result<(), UniqueIdError> {
        if self.recycled_ids.insert(old_id) {
            Ok(())
        } else {
            Err(UniqueIdError::RecycledIdExists(old_id))
        }
    }
}
unsafe impl Sync for UniqueIdGen {}

#[derive(Debug, Clone, Copy)]
pub enum UniqueIdError {
    /// Means that no more unique ids can be generated from this instance. Can be fixed by
    MaxReached,
    /// Id could not be inserted into the `recycled_ids` collection, because it already exists there.
    RecycledIdExists(UniqueId),
}
impl fmt::Display for UniqueIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MaxReached => {
                write!(f, "maximum usize value reached in unique id generator")
            }
            Self::RecycledIdExists(recycled_id) => {
                write!(f, "recycled id {} could not be inserted into recycled_ids collection because it already exists there", recycled_id)
            }
        }
    }
}
impl error::Error for UniqueIdError {}
