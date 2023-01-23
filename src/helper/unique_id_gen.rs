use std::sync::atomic::{AtomicUsize, Ordering};

pub struct UniqueIdGen {
    counter: AtomicUsize,
}

impl UniqueIdGen {
    pub fn new() -> Self {
        Self {
            counter: AtomicUsize::new(1),
        }
    }

    pub fn new_id(&mut self) -> usize {
        // todo error when reacing usize::MAX
        self.counter.fetch_add(1, Ordering::Relaxed)
    }
}
