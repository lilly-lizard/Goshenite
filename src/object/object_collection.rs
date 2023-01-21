use super::object::Object;

pub struct ObjectCollection {
    objects: Vec<Object>,
}

impl ObjectCollection {
    pub fn new() -> Self {
        Self {
            objects: Vec::default(),
        }
    }

    pub fn objects(&self) -> &Vec<Object> {
        &self.objects
    }

    pub fn get(&self, index: usize) -> Option<&Object> {
        self.objects.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Object> {
        self.objects.get_mut(index)
    }

    /// Returns the position `object` was pushed into
    pub fn push(&mut self, object: Object) -> usize {
        self.objects.push(object);
        self.objects.len() - 1
    }

    pub fn remove(&mut self, index: usize) {
        self.objects.remove(index);
    }
}
