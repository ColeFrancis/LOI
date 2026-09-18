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
    symbol::{Symbol, SymbolId},
    ast::{Net, NetItem, Ident},
    netlist::{Netlist, Entity, Relation},
    diagnostics::Diagnostics,
};

impl Synthesis {
    pub fn synthesize(nets: Vec<Net>, top_net_idx: usize, mut obj_map: HashMap::<SymbolId, usize>, symbol_table: &[Symbol], diagnostics: &mut Diagnostics) -> Netlist{
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        let mut ents = Vec::new();
        let mut relations = Vec::new();

        //  if the ident is not in the hashmap, then I push a new ent to ents
        //  initilize net statemetns will set the value of the net to be Some(value) rather than None

        // The top level net's ports need to be added to inputs and outputs. (somehow we need their names too)

        for item in &nets[top_net_idx].items {
            match item {
                NetItem::Input(input) => {
                    let Ident::Symbol(id) = input.param.name else {
                        unreachable!("should not be ident string");
                    };

                    let idx = ents.len();

                    inputs.push((symbol_table[id].name.to_string(), idx));
                    obj_map.insert(id, idx);
                    ents.push(Entity {
                        val: None,
                        sinks: vec![],
                    });
                }

                NetItem::Output(output) => {
                    let Ident::Symbol(id) = output.param.name else {
                        unreachable!("should not be ident string");
                    };

                    let idx = ents.len();

                    outputs.push((symbol_table[id].name.to_string(), idx));
                    obj_map.insert(id, idx);
                    ents.push(Entity {
                        val: None,
                        sinks: vec![],
                    });
                }

                _ => {}
            }
        }

        Netlist {
            inputs,
            outputs,
            relations,
            ents,
        }
    }

    fn synthesize_net(net: &Net, obj_map: &mut HashMap::<SymbolId, usize>, ents: &mut Vec<Entity>, relations: &mut Vec<Relation>) {
        // fold expressions. 

        for item in &net.items {
            match item {
                NetItem::Input(input) => {
                    let Ident::Symbol(id) = input.param.name else {
                        unreachable!("should not be ident string");
                    };

                    Self::insert_or_get_ent(id, obj_map, ents);
                }

                NetItem::Output(input) => {
                    let Ident::Symbol(id) = input.param.name else {
                        unreachable!("should not be ident string");
                    };

                    Self::insert_or_get_ent(id, obj_map, ents);
                }

                NetItem::Init(init) => {
                    let Ident::Symbol(id) = init.param.name else {
                        unreachable!("should not be ident string");
                    };

                    // fold expression

                    // ents[idx].val = Some(folded_val);
                }

                NetItem::RelInst(rel) => {
                    let Ident::Symbol(rel_id) = rel.rel else {
                        unreachable!("should not be ident string");
                    };

                    let Some(&rel_idx) = obj_map.get(&rel_id) else {
                        unreachable!("rel should be in obj_map");
                    };

                    let Ident::Symbol(asignee_id) = rel.asignee else {
                        unreachable!("should not be ident string");
                    };

                    let asignee_idx = Self::insert_or_get_ent(asignee_id, obj_map, ents);

                    let mut input_indices = Vec::new();
                    for arg in &rel.args {
                        let Ident::Symbol(arg_id) = arg else {
                            unreachable!("should not be ident string");
                        };

                        let arg_idx = Self::insert_or_get_ent(*arg_id, obj_map, ents);

                        ents[arg_idx].sinks.push(rel_idx);

                        input_indices.push(arg_idx);
                    }

                    relations.push(Relation {
                        idx: rel_idx,
                        input_ents: input_indices,
                        output_ent: asignee_idx,
                    });
                }

                NetItem::NetInst(net) => {

                }

                NetItem::Error => {}
            }
        }
    }

    fn insert_or_get_ent(id: SymbolId, obj_map: &mut HashMap::<SymbolId, usize>, ents: &mut Vec<Entity>) -> usize {
        match obj_map.get(&id) {
            Some(&idx) => idx,
            None => {
                let idx = ents.len();

                obj_map.insert(id, idx);

                ents.push(Entity {
                    val: None,
                    sinks: vec![],
                });

                idx
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // fn input_item() {
    //     let mut diagnostics = Diagnostics::new();
    //     let mut obj_map = HashMap::<SymbolId, usize>::new();
    //     let mut ents: Vec<Entity> = Vec::new();
    // }
}