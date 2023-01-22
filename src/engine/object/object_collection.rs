use super::object::Object;
use std::rc::Rc;

pub struct ObjectCollection {
    objects: Vec<Rc<Object>>,
}

impl ObjectCollection {
    pub fn new() -> Self {
        Self {
            objects: Vec::default(),
        }
    }

    pub fn objects(&self) -> &Vec<Rc<Object>> {
        &self.objects
    }

    pub fn get(&self, index: usize) -> Option<&Rc<Object>> {
        self.objects.get(index)
    }

    /// Returns the position `object` was pushed into
    pub fn push(&mut self, object: Object) -> usize {
        self.objects.push(Rc::new(object));
        self.objects.len() - 1
    }

    pub fn remove(&mut self, index: usize) {
        self.objects.remove(index);
    }
}
