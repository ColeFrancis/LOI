// Copyright 2026 Cole Francis
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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