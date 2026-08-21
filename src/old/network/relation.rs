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

//! # relation
//!
//! This module defines the relation type whcih form the vertices of a network and
//! implements the eval function
//!
//! ## Invariants
//!
//! - Relations must contain an operation field, inputs, and outputs.
//! - Relations must implement eval
//!
//! Author: Cole Francis

use super::entity::EntityId;
use crate::network::network::Network;
use crate::core::operations::Operator;

pub type RelationId = usize;

pub struct Relation<O> {
    pub op: O,
    pub a: EntityId,
    pub b: EntityId,
    pub out: EntityId,
}

impl<O> Relation<O> {
    pub fn eval<T>(&self, network: &Network<T, O>) -> T 
    where
        T: Copy,
        O: Operator<T>,
    {
        let a = network.entities[self.a].value;
        let b = network.entities[self.b].value;

        self.op.eval(a, b)
    }
}