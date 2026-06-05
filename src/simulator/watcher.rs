use crate::network::entity::EntityId;

pub struct Watcher<T> {
    pub entity: EntityId,
    pub outputs: Vec<T>,
}

impl<T> Watcher<T> {
    pub fn new(entity: EntityId) -> Self {
        Self {
            entity,
            outputs: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.outputs.clear();
    }
}