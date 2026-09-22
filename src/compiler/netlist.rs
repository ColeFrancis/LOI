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

//! # netlist
//!
//! Defines netlist structure
//!
//! ## Invariants
//!
//! - 
//!
//! Author: Cole Francis

pub type EntId = usize;

#[derive(PartialEq, Debug)]
pub struct Netlist {
    pub inputs: Vec<(String, EntId)>, // name of port needed for taking inputs in the simulator
    pub outputs: Vec<(String, EntId)>, // name of port needed for reporting outputs in the simulator
    pub relations: Vec<Relation>,
    pub ents: Vec<Entity>,
}

impl Netlist {
    pub fn new() -> Self {
        Self {
            inputs: vec![],
            outputs: vec![],
            relations: vec![],
            ents: vec![],
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct Relation {
    pub idx: usize,
    pub delay: usize,
    pub input_ents: Vec<EntId>,
    pub output_ent: EntId,
}

pub type RelId = usize;

#[derive(PartialEq, Debug)]
pub struct Entity {
    pub val: Option<u64>,
    pub sinks: Vec<RelId>,
}