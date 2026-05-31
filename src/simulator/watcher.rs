use crate::core::types::{NetId, Logic};

pub struct Watcher {
    pub net: NetId,
    pub outputs: Vec<Logic>,
}

impl Watcher {
    pub fn new(net: NetId) -> Self {
        Self {
            net,
            outputs: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.outputs.clear();
    }
}