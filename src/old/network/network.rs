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

//! # network
//!
//! This module defines the network type which forms the basis of nhdl
//!
//! ## Invariants
//!
//! - Netwokrs must be made up of only relations and entities
//!
//! Author: Cole Francis

use super::relation::Relation;
use super::entity::Entity;

pub struct Network<T, O> {
    pub relations: Vec<Relation<O>>,
    pub entities: Vec<Entity<T>>,
}

impl<T, O> Network<T, O> {
    pub fn new() -> Self {
        Self {
            relations: Vec::new(),
            entities: Vec::new(),
        }
    }
}