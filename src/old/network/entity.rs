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

//! # entity
//!
//! This module defines the entity type which form the edges of a network
//!
//! ## Invariants
//!
//! - Entity must contain a value field for its current state and a list of
//! the relations it attaches to
//!
//! Author: Cole Francis

use super::relation::RelationId;

pub type EntityId = usize;

pub struct Entity<T> {
    pub value: T,
    pub sinks: Vec<RelationId>,
}