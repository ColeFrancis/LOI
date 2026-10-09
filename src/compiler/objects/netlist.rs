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

use std::collections::HashMap;

use super::types::Type;

pub type RelId = usize;
pub type EntId = usize;

#[derive(PartialEq, Debug)]
pub struct Netlist {
    pub relations: Vec<Relation>,
    pub ents: Vec<Entity>,
}

impl Netlist {
    pub fn new() -> Self {
        Self {
            relations: vec![],
            ents: vec![],
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct Interface {
    pub inputs: HashMap<String, (EntId, Type)>, // name of port needed for taking inputs in the simulator
    pub outputs: HashMap<EntId, (String, Type)>, // name of port needed for reporting outputs in the simulator
    pub custom_type_maps: HashMap<String, Vec<(String, u64)>>,
}

impl Interface {
    pub fn new() -> Self {
        Self {
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            custom_type_maps: HashMap::new(),
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

#[derive(PartialEq, Debug)]
pub struct Entity {
    pub val: Option<u64>,
    pub sinks: Vec<RelId>,
}
