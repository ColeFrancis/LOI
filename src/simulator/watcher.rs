//! # watcher
//!
//! This module defines the watcher type which continuosuly records the value of
//! an entity in a network
//!
//! ## Invariants
//!
//! - The watcher must record all values of an entity across all time steps of a simulation
//! - The watcher must be able to be resetted
//!
//! Author: Cole Francis
//!
//! Last Updated: 06/02/2026

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