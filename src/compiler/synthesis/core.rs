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

//! # core
//!
//! syntehsyzes a set of nets into a netlist of entities and relations
//!
//! ## Invariants
//!
//! Author: Cole Francis

use std::collections::HashMap;

use super::Synthesis;
use crate::compiler::{
    symbol::SymbolId,
    ast::{Net, Ident},
    netlist::Netlist,
};

impl Synthesis {
    pub fn synthesize(nets: Vec<Net>, top_net_idx: usize, rel_map: HashMap::<SymbolId, usize>, net_map: HashMap::<SymbolId, usize>) -> Netlist{
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        let mut ents = Vec::new();
        let mut relations = Vec::new();

        // create a hashmap that maps idents (or just ids) to an index of the ent in ents
        //  if the ident is not in the hashmap, then I push a new ent to ents
        //  initilize net statemetns will set the value of the net to be Some(value) rather than None

        Netlist {
            inputs,
            outputs,
            relations,
            ents,
        }
    }
}