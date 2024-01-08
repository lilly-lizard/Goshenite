use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, error, fmt};

// gpu id buffer packed as 16 bits for object and 16 bits for primitive op.
// 32 bit uint images have guarenteed vulkan support
pub type UniqueId = u16;

pub trait UniqueIdType: From<UniqueId> + Ord {
    fn raw_id(&self) -> UniqueId;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UniqueIdGen<T: UniqueIdType> {
    counter: UniqueId,
    recycled_ids: BTreeSet<T>,
}

impl<T: UniqueIdType> UniqueIdGen<T> {
    pub const fn new() -> Self {
        Self {
            counter: 1,
            recycled_ids: BTreeSet::new(),
        }
    }

    pub fn new_id(&mut self) -> Result<T, UniqueIdError> {
        if self.counter == UniqueId::MAX {
            // try recyling. prefer not to do this in case removed ids are mistakenly lingering around...
            if let Some(new_id) = self.recycled_ids.pop_first() {
                return Ok(new_id);
            } else {
                return Err(UniqueIdError::MaxReached);
            }
        }

        let new_id = self.counter;
        self.counter = self.counter + 1;

        Ok(new_id.into())
    }

    pub fn recycle_id(&mut self, old_id: T) -> Result<(), UniqueIdError> {
        let raw_id = old_id.raw_id();
        if self.recycled_ids.insert(old_id) {
            Ok(())
        } else {
            Err(UniqueIdError::RecycledIdExists(raw_id))
        }
    }
}

unsafe impl<T: UniqueIdType> Sync for UniqueIdGen<T> {}

// ~~ UniqueId Error ~~

#[derive(Debug)]
pub enum UniqueIdError {
    /// Means that no more unique ids can be generated from this instance.
    MaxReached,
    /// Id could not be inserted into the `recycled_ids` collection, because it already exists there.
    RecycledIdExists(UniqueId),
}

impl fmt::Display for UniqueIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MaxReached => {
                write!(
                    f,
                    "maximum number of unique ids have been generated for this instance"
                )
            }
            Self::RecycledIdExists(recycled_id) => {
                write!(f, "recycled id {} could not be inserted into recycled_ids collection because it already exists there", recycled_id)
            }
        }
    }
}

impl error::Error for UniqueIdError {}
