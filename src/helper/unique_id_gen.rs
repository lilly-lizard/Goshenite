use std::sync::atomic::{AtomicUsize, Ordering};

pub type UniqueId = usize;

pub struct UniqueIdGen {
    counter: AtomicUsize,
}
impl UniqueIdGen {
    pub const fn new() -> Self {
        Self {
            counter: AtomicUsize::new(1),
        }
    }

    pub fn new_id(&mut self) -> UniqueId {
        // todo error when reacing usize::MAX
        self.counter.fetch_add(1, Ordering::Relaxed)
    }
}
unsafe impl Sync for UniqueIdGen {}
