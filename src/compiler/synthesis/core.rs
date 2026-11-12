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

use std::collections::{HashMap, HashSet};

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use super::Synthesis;
use super::union_find::UnionFind;
use crate::compiler::{
    error_handling::diagnostics::Diagnostics,
    error_handling::span::Span,
    objects::{
        ast::{Expr, Ident, Literal, Net, NetItem, Statement},
        netlist::{EntId, Entity, Interface, Netlist, Relation},
        symbol::{Symbol, SymbolId, SymbolKind},
        types::Type,
    },
    sem_analyzer::SemAnalyzer,
};

use crate::simulator::objects::event::Event;

impl<'a> Synthesis<'a> {
    pub fn new(
        nets: Vec<Net>,
        net_map: HashMap<SymbolId, usize>,
        symbols: &'a mut [Symbol],
        seed: Option<u64>,
        diagnostics: &'a mut Diagnostics,
    ) -> Self {
        Self {
            nets,
            net_map,
            rel_map: HashMap::new(),
            symbols,
            seed,
            diagnostics,
            next_compiled_rel_idx: 0,
        }
    }

    pub fn synthesize(mut self, top_net_idx: usize) -> (Netlist, Interface, Vec<Event>, HashMap<SymbolId, usize>) {
        let mut inputs = HashMap::<String, (EntId, Type)>::new();
        let mut outputs = HashMap::<EntId, (String, Type)>::new();
        let mut custom_type_maps = HashMap::<String, Vec<(String, u64)>>::new();
        let mut relations = Vec::new();
        let mut inits = Vec::new();

        let mut union_find = UnionFind::new(0);

        let mut ent_map: HashMap<SymbolId, usize> = HashMap::new();
        for item in &self.nets[top_net_idx].items {
            match item {
                NetItem::Input(input) => {
                    let Ident::Symbol(id) = input.param.name else {
                        unreachable!("should not be ident string");
                    };

                    let ent_type = match input.param.param_type {
                        Type::Bool => Type::Bool,
                        Type::Impulse => Type::Impulse,
                        Type::Int => Type::Int,
                        Type::Float => Type::Float,
                        Type::Mod(n) => Type::Mod(n),
                        Type::Custom(Ident::Symbol(id)) => {
                            let name = self.symbols[id].name.clone();
                            Type::Custom(Ident::Str {
                                val: name,
                                span: Span { line: 0, col: 0 },
                            })
                        }
                        _ => unreachable!("type should match one of above"),
                    };

                    let idx = Self::insert_or_get_ent(id, &mut ent_map, &mut union_find);
                    inputs.insert(self.symbols[id].name.to_string(), (idx, ent_type));
                }

                NetItem::Output(output) => {
                    let Ident::Symbol(id) = output.param.name else {
                        unreachable!("should not be ident string");
                    };

                    let ent_type = match output.param.param_type {
                        Type::Bool => Type::Bool,
                        Type::Impulse => Type::Impulse,
                        Type::Int => Type::Int,
                        Type::Float => Type::Float,
                        Type::Mod(n) => Type::Mod(n),
                        Type::Custom(Ident::Symbol(id)) => {
                            let name = self.symbols[id].name.clone();
                            Type::Custom(Ident::Str {
                                val: name,
                                span: Span { line: 0, col: 0 },
                            })
                        }
                        _ => unreachable!("type should match one of above"),
                    };

                    let idx = Self::insert_or_get_ent(id, &mut ent_map, &mut union_find);
                    outputs.insert(idx, (self.symbols[id].name.to_string(), ent_type));
                }

                _ => {}
            }
        }

        for symbol in self.symbols.iter() {
            if let SymbolKind::EntMember { parent, mapping } = &symbol.kind {
                let symbol_name = symbol.name.clone();
                let parent_name = self.symbols[*parent].name.clone();

                custom_type_maps
                    .entry(parent_name)
                    .or_default()
                    .push((symbol_name, *mapping as u64));
            }
        }

        self.synthesize_net(
            top_net_idx,
            ent_map,
            &mut relations,
            &mut inits,
            &mut union_find
        );
        
        let ents = Self::canonicalize(&mut relations, &mut inits, union_find);

        (
            Netlist { relations, ents },
            Interface {
                inputs,
                outputs,
                custom_type_maps,
            },
            inits,
            self.rel_map,
        )
    }

    #[cfg(test)]
    pub fn test_synthesize_net(
        nets: Vec<Net>,
        target_net_idx: usize,
        net_map: HashMap<SymbolId, usize>,
        ent_map: HashMap<SymbolId, usize>,
        ents: &mut Vec<Entity>,
        relations: &mut Vec<Relation>,
        symbols: &mut [Symbol],
        seed: Option<u64>,
        diagnostics: &mut Diagnostics,
    ){
        let mut union_find = UnionFind::new(0);
        let mut inits = Vec::new();
        Synthesis::new(nets, net_map, symbols, seed, diagnostics).synthesize_net(target_net_idx, ent_map, relations, &mut inits, &mut union_find);

        *ents = Self::canonicalize(relations, &mut inits, union_find);
    }

    fn synthesize_net(
        &mut self,
        target_net_idx: usize,
        mut ent_map: HashMap<SymbolId, usize>,
        relations: &mut Vec<Relation>,
        inits: &mut Vec<Event>,
        union_find: &mut UnionFind,
    ) {
        // make net_map an immutable reference that honly maps for nets and rels
        // make new mutable ent_map that is passed in (not a reference) and a new one is created for each net instantiation
        //  to allow for the interal nets of a net be different ents in the netlist for each seperate instantiation of that net
        let items = std::mem::take(&mut self.nets[target_net_idx].items);

        for item in &items {
            match item {
                NetItem::Stmt(Statement::Var(var)) => {
                    // fold expression, deep clone necessary
                    let mut rng = match self.seed {
                        Some(seed) => ChaCha8Rng::seed_from_u64(seed),
                        None => ChaCha8Rng::from_rng(&mut rand::rng()),
                    };

                    SemAnalyzer::fold_let_inner(var.clone(), true, self.symbols, &mut rng, self.diagnostics);
                }

                NetItem::Input(input) => {
                    let Ident::Symbol(id) = input.param.name else {
                        unreachable!("should not be ident string");
                    };

                    Self::insert_or_get_ent(id, &mut ent_map, union_find);
                }

                NetItem::Output(input) => {
                    let Ident::Symbol(id) = input.param.name else {
                        unreachable!("should not be ident string");
                    };

                    Self::insert_or_get_ent(id, &mut ent_map, union_find);
                }

                NetItem::Alias(alias) => {
                    let Ident::Symbol(left_id) = alias.left else {
                        unreachable!("should not be ident string");
                    };
                    let Ident::Symbol(right_id) = alias.right else {
                        unreachable!("should not be ident string");
                    };

                    let left_idx = Self::insert_or_get_ent(left_id, &mut ent_map, union_find);
                    let right_idx = Self::insert_or_get_ent(right_id, &mut ent_map, union_find);

                    union_find.union(left_idx, right_idx);
                }

                NetItem::RelInst(rel) => {
                    let Ident::Symbol(rel_id) = rel.rel else {
                        unreachable!("should not be ident string");
                    };

                    let rel_idx = self.insert_or_get_rel(rel_id);

                    let Ident::Symbol(asignee_id) = rel.asignee else {
                        unreachable!("should not be ident string");
                    };

                    let asignee_idx = Self::insert_or_get_ent(asignee_id, &mut ent_map, union_find);

                    let mut input_indices = Vec::new();
                    for arg in &rel.args {
                        let Ident::Symbol(arg_id) = arg else {
                            unreachable!("should not be ident string");
                        };

                        let arg_idx = Self::insert_or_get_ent(*arg_id, &mut ent_map, union_find);

                        input_indices.push(arg_idx);
                    }

                    let delay = match rel.delay {
                        Expr::Literal(Literal::Int(val)) => val as usize,

                        _ => unreachable!("type checking already occured and it should be an int literal")
                    };

                    relations.push(Relation {
                        idx: rel_idx,
                        delay,
                        input_ents: input_indices,
                        output_ent: asignee_idx,
                    });
                }

                NetItem::NetInst(net) => {
                    let mut sub_net_map: HashMap<SymbolId, usize> = HashMap::new();

                    let Ident::Symbol(net_id) = net.net else {
                        unreachable!("should not be ident string");
                    };
                    let Some(&net_idx) = self.net_map.get(&net_id) else {
                        unreachable!("net should be in net_map");
                    };

                    for connection in &net.connections {
                        let Ident::Symbol(ent_id) = connection.ent else {
                            unreachable!("should not be ident string");
                        };
                        let Ident::Symbol(port_id) = connection.port else {
                            unreachable!("should not be ident string");
                        };

                        let ent_idx = Self::insert_or_get_ent(ent_id, &mut ent_map, union_find);

                        // both the port and the connected ent are the same ent in the netlist
                        sub_net_map.insert(port_id, ent_idx);
                    }

                    self.synthesize_net(
                        net_idx,
                        sub_net_map,
                        relations,
                        inits,
                        union_find,
                    );
                }

                _ => {}
            }
        }

        // Process inits entirely after the rest so that if instantiated nets also have init values the outer overrides the inner
        for item in &items {
            if let NetItem::Init(init) = item {
                let Ident::Symbol(id) = init.param.name else {
                    unreachable!("should not be ident string");
                };

                let idx = Self::insert_or_get_ent(id, &mut ent_map, union_find);

                // fold expression, deep clone necessary
                let mut rng = match self.seed {
                    Some(seed) => ChaCha8Rng::seed_from_u64(seed),
                    None => ChaCha8Rng::from_rng(&mut rand::rng()),
                };
                let result_expr = SemAnalyzer::fold_expr_inner(init.val.clone(), false, self.symbols, &mut rng, self.diagnostics);

                let folded_val = match result_expr {
                    Expr::Literal(literal) => match (literal, &init.param.param_type) {
                        (Literal::Bool(b), &Type::Bool) => b as u64,
                        (Literal::Bool(b), &Type::Impulse) => match b {
                            true => 0,
                            false => u64::MAX, // typically 0 is chosen when an impulse needs to be set to a known false value but here we need to represent a true value at time 0
                        },

                        (Literal::Int(i), &Type::Mod(n)) => (i % n) as u64,
                        (Literal::Int(i), &Type::Int) => i as u64,
                        (Literal::Int(i), &Type::Float) => (i as f64).to_bits(),

                        (Literal::Float(r), &Type::Float) => r.to_bits(),
                        (Literal::Float(r), &Type::Complex) => (r as f32).to_bits() as u64,

                        (Literal::Complex(a, b), &Type::Complex) => {
                            (a.to_bits() as u64) | ((b.to_bits() as u64) << 32)
                        }

                        _ => unreachable!("type checking already occured"),
                    },

                    Expr::Ident(Ident::Symbol(id)) => match self.symbols[id].kind {
                        SymbolKind::EntMember { mapping, .. } => mapping as u64,

                        _ => unreachable!("ident should be ent member"),
                    },

                    _ => unreachable!("type checking already occured"),
                };

                inits.push(Event {
                    timestep: 0,
                    ent_id: idx,
                    new_val: folded_val,
                });
            }
        }

        self.nets[target_net_idx].items = items;
    }

    fn canonicalize(relations: &mut Vec<Relation>, inits: &mut Vec<Event>, mut union_find: UnionFind) -> Vec<Entity> {
        let mut root_to_new = HashMap::new();
        let mut old_to_new = Vec::with_capacity(union_find.len()); // replaces union_find to get compacted indices of the corresponding entities

        for i in 0..union_find.len() {
            let root = union_find.find(i);

            let next = root_to_new.len();
            let new_idx = *root_to_new.entry(root).or_insert(next);

            old_to_new.push(new_idx);
        }

        let mut ents = (0..root_to_new.len())
            .map(|_| Entity {
                val: None,
                sinks: Vec::new(),
            })
            .collect::<Vec<_>>();

        for (rel_id, relation) in relations.iter_mut().enumerate() {
            for arg in &mut relation.input_ents {
                *arg = old_to_new[*arg];

                if !ents[*arg].sinks.contains(&rel_id) {
                    ents[*arg].sinks.push(rel_id);
                }
            }

            relation.output_ent = old_to_new[relation.output_ent];
        }

        let mut initilaized = HashSet::new();
        let mut canonical_inits = Vec::with_capacity(inits.len());

        for mut event in inits.drain(..).rev() {
            event.ent_id = old_to_new[event.ent_id];

            if initilaized.insert(event.ent_id) {
                canonical_inits.push(event);
            }
        }

        *inits = canonical_inits;

        ents
    }

    fn insert_or_get_rel(&mut self, rel_id: usize) -> usize {
        match self.rel_map.get(&rel_id) {
            Some(&rel_idx) => rel_idx,
            None => {
                let next = self.next_compiled_rel_idx;
                self.next_compiled_rel_idx += 1;

                self.rel_map.insert(rel_id, next);

                next
            }
        }
    }

    fn insert_or_get_ent(
        id: SymbolId,
        ent_map: &mut HashMap<SymbolId, usize>,
        union_find: &mut UnionFind,
    ) -> usize {
        match ent_map.get(&id) {
            Some(&idx) => idx,
            None => {
                let idx = union_find.add();

                ent_map.insert(id, idx);

                idx
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::error_handling::span::Span;
    use crate::compiler::objects::ast::*;
    use crate::compiler::objects::symbol::NetPort;

    #[test]
    fn rel_inst_1() {
        // rel_t ADD : (a: Int, b: Int) -> Int = a + b;

        // net ADD_NET {
        //     input A: Int;
        //     input B: Int;
        //     input C: Int;
        //     output S: Int;

        //     S = ADD(A, B) @ 2;
        // }
        let mut diagnostics = Diagnostics::new();
        let mut ents: Vec<Entity> = Vec::new();
        let mut relations: Vec<Relation> = Vec::new();

        let mut symbol_table = vec![
            Symbol {
                name: "REL".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Int, Type::Int],
                    return_type: Type::Int,
                    
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "ADD_NET".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([
                        (
                            "A".to_string(),
                            NetPort {
                                symbol: 2,
                                input: true,
                            },
                        ),
                        (
                            "B".to_string(),
                            NetPort {
                                symbol: 3,
                                input: true,
                            },
                        ),
                        (
                            "C".to_string(),
                            NetPort {
                                symbol: 4,
                                input: true,
                            },
                        ),
                        (
                            "S".to_string(),
                            NetPort {
                                symbol: 5,
                                input: false,
                            },
                        ),
                    ]),
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "A".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "B".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "C".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "S".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
        ];

        let net = Net {
            name: Ident::Symbol(1),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(2),
                        param_type: Type::Int,
                    },
                    span: Span { line: 0, col: 0 },
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(3),
                        param_type: Type::Int,
                    },
                    span: Span { line: 0, col: 0 },
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(4),
                        param_type: Type::Int,
                    },
                    span: Span { line: 0, col: 0 },
                }),
                NetItem::Output(OutputEnt {
                    param: Param {
                        name: Ident::Symbol(5),
                        param_type: Type::Int,
                    },
                }),
                NetItem::RelInst(RelInst {
                    asignee: Ident::Symbol(5),
                    rel: Ident::Symbol(0),
                    args: vec![Ident::Symbol(2), Ident::Symbol(3)],
                    delay: Expr::Literal(Literal::Int(2)),
                    span: Span { line: 0, col: 0 },
                }),
            ],
        };
        let net_map = HashMap::from([(0, 0), (1, 0)]);
        let ent_map: HashMap<SymbolId, usize> = HashMap::new();

        Synthesis::test_synthesize_net(
            vec![net],
            0,
            net_map,
            ent_map,
            &mut ents,
            &mut relations,
            &mut symbol_table,
            None,
            &mut diagnostics,
        );

        assert_eq!(
            ents,
            vec![
                Entity {
                    val: None,
                    sinks: vec![0],
                },
                Entity {
                    val: None,
                    sinks: vec![0],
                },
                Entity {
                    val: None,
                    sinks: vec![],
                },
                Entity {
                    val: None,
                    sinks: vec![],
                },
            ]
        );
        assert_eq!(
            relations,
            vec![Relation {
                idx: 0,
                delay: 2,
                input_ents: vec![0, 1],
                output_ent: 3,
            },]
        );
    }

    #[test]
    fn rel_inst_2() {
        // rel_t Rel1 : () -> Int = 1;
        // rel_t Rel2 : () -> Int = 2; // Rel2 never instantiated
        // rel_t Rel3 : () -> Int = 3;

        // net NET {

        //     Rel1 := A;
        //     Rel3 := B;
        // }
        let mut diagnostics = Diagnostics::new();
        let mut ents: Vec<Entity> = Vec::new();
        let mut relations: Vec<Relation> = Vec::new();

        let mut symbol_table = vec![
            Symbol {
                name: "REL1".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![],
                    return_type: Type::Int,
                    
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "REL2".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![],
                    return_type: Type::Int,
                    
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "REL3".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![],
                    return_type: Type::Int,
                    
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "NET".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([]),
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "A".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "B".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
        ];

        let net = Net {
            name: Ident::Symbol(1),
            items: vec![
                NetItem::RelInst(RelInst {
                    asignee: Ident::Symbol(4),
                    rel: Ident::Symbol(0),
                    args: vec![],
                    delay: Expr::Literal(Literal::Int(1)),
                    span: Span { line: 0, col: 0 },
                }),
                NetItem::RelInst(RelInst {
                    asignee: Ident::Symbol(5),
                    rel: Ident::Symbol(2),
                    args: vec![],
                    delay: Expr::Literal(Literal::Int(1)),
                    span: Span { line: 0, col: 0 },
                }),
            ],
        };
        let net_map = HashMap::from([(0, 0), (2, 1)]);
        let ent_map: HashMap<SymbolId, usize> = HashMap::new();

        Synthesis::test_synthesize_net(
            vec![net],
            0,
            net_map,
            ent_map,
            &mut ents,
            &mut relations,
            &mut symbol_table,
            None,
            &mut diagnostics,
        );

        assert_eq!(
            ents,
            vec![
                Entity {
                    val: None,
                    sinks: vec![],
                },
                Entity {
                    val: None,
                    sinks: vec![],
                },
            ]
        );
        assert_eq!(
            relations,
            vec![
                Relation {
                    idx: 0,
                    delay: 1,
                    input_ents: vec![],
                    output_ent: 0,
                },
                Relation {
                    idx: 1,
                    delay: 1,
                    input_ents: vec![],
                    output_ent: 1,
                },
            ]
        );
    }

    #[test]
    fn net_inst_1() {
        // rel_t ADD : (a: Int, b: Int) -> Int = a + b;

        // net ADD_NET {
        //     input A: Int;
        //     input B: Int;
        //     input C: Int;
        //     output S: Int;

        //     S = ADD(A, B);
        // }

        // net ADD_TO_SELF {
        //     input IN: Int;
        //     output OUT: Int;

        //     ADD_NET {
        //         A := IN,
        //         B := IN,
        //         S := OUT,
        //     };
        // }
        let mut diagnostics = Diagnostics::new();
        let mut ents: Vec<Entity> = Vec::new();
        let mut relations: Vec<Relation> = Vec::new();

        let mut symbol_table = vec![
            Symbol {
                name: "REL".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Int, Type::Int],
                    return_type: Type::Int,
                    
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "ADD_NET".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([
                        (
                            "A".to_string(),
                            NetPort {
                                symbol: 2,
                                input: true,
                            },
                        ),
                        (
                            "B".to_string(),
                            NetPort {
                                symbol: 3,
                                input: true,
                            },
                        ),
                        (
                            "C".to_string(),
                            NetPort {
                                symbol: 4,
                                input: true,
                            },
                        ),
                        (
                            "S".to_string(),
                            NetPort {
                                symbol: 5,
                                input: false,
                            },
                        ),
                    ]),
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "A".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "B".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "C".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "S".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "ADD_TO_SELF".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([
                        (
                            "IN".to_string(),
                            NetPort {
                                symbol: 7,
                                input: true,
                            },
                        ),
                        (
                            "OUT".to_string(),
                            NetPort {
                                symbol: 8,
                                input: false,
                            },
                        ),
                    ]),
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "IN".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "OUT".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
        ];

        let net1 = Net {
            name: Ident::Symbol(1),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(2),
                        param_type: Type::Int,
                    },
                    span: Span { line: 0, col: 0 },
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(3),
                        param_type: Type::Int,
                    },
                    span: Span { line: 0, col: 0 },
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(4),
                        param_type: Type::Int,
                    },
                    span: Span { line: 0, col: 0 },
                }),
                NetItem::Output(OutputEnt {
                    param: Param {
                        name: Ident::Symbol(5),
                        param_type: Type::Int,
                    },
                }),
                NetItem::RelInst(RelInst {
                    asignee: Ident::Symbol(5),
                    rel: Ident::Symbol(0),
                    args: vec![Ident::Symbol(2), Ident::Symbol(3)],
                    delay: Expr::Literal(Literal::Int(1)),
                    span: Span { line: 0, col: 0 },
                }),
            ],
        };
        let net2 = Net {
            name: Ident::Symbol(6),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(7),
                        param_type: Type::Int,
                    },
                    span: Span { line: 0, col: 0 },
                }),
                NetItem::Output(OutputEnt {
                    param: Param {
                        name: Ident::Symbol(8),
                        param_type: Type::Int,
                    },
                }),
                NetItem::NetInst(NetInst {
                    net: Ident::Symbol(1),
                    connections: vec![
                        Connection {
                            port: Ident::Symbol(2),
                            ent: Ident::Symbol(7),
                            span: Span { line: 0, col: 0 },
                        },
                        Connection {
                            port: Ident::Symbol(3),
                            ent: Ident::Symbol(7),
                            span: Span { line: 0, col: 0 },
                        },
                        Connection {
                            port: Ident::Symbol(5),
                            ent: Ident::Symbol(8),
                            span: Span { line: 0, col: 0 },
                        },
                    ],
                    span: Span{line: 0, col: 0},
                }),
            ],
        };

        let net_map = HashMap::from([(0, 0), (1, 0), (6, 1)]);
        let ent_map: HashMap<SymbolId, usize> = HashMap::new();

        Synthesis::test_synthesize_net(
            vec![net1, net2],
            1,
            net_map,
            ent_map,
            &mut ents,
            &mut relations,
            &mut symbol_table,
            None,
            &mut diagnostics,
        );

        assert_eq!(
            ents,
            vec![
                Entity {
                    // IN, A, B
                    val: None,
                    sinks: vec![0],
                },
                Entity {
                    // OUT, S
                    val: None,
                    sinks: vec![],
                },
                Entity {
                    // C
                    val: None,
                    sinks: vec![],
                },
            ]
        );
        assert_eq!(
            relations,
            vec![Relation {
                idx: 0,
                delay: 1,
                input_ents: vec![0, 0],
                output_ent: 1,
            },]
        );
    }

    #[test]
    fn net_inst_2() {
        // rel_t ADD : (a: Int, b: Int, c: Int) -> Int = a + b + b;
        // rel_t MUL : (a: Int, b: Int) -> Int = a*b;
        // rel_t MUL_2 : (a: Int) -> Int = 2*a;

        // net INNER {
        //     input A: Int;
        //     input B: Int;
        //     output S: Int;

        //     init A: Int = 2;

        //     S_inter = MUL(A, B);
        //     S = MUL_2(S_inter);
        // }

        // net OUTER {
        //     input IN1: Int;
        //     input IN2: Int;
        //     input IN3: Int;
        //     output OUT: Int;

        //     init IN1: Int = 4;

        //     INNER {
        //         A := IN1,
        //         B := IN1,
        //         S := OUT1,
        //     };
        //     INNER {
        //         A := IN2,
        //         B := IN2,
        //         S := OUT2,
        //     };
        //     INNER {
        //         A := IN3,
        //         B := IN3,
        //         S := OUT3,
        //     };

        //     OUT = ADD(OUT1, OUT2, OUT3);
        // }
        let mut diagnostics = Diagnostics::new();
        let mut ents: Vec<Entity> = Vec::new();
        let mut relations: Vec<Relation> = Vec::new();

        let mut symbol_table = vec![
            Symbol {
                name: "ADD".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Int, Type::Int, Type::Int],
                    return_type: Type::Int,
                    
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "MUL".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Int, Type::Int],
                    return_type: Type::Int,
                    
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "MUL_2".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Int],
                    return_type: Type::Int,
                    
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "INNER".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([
                        (
                            "A".to_string(),
                            NetPort {
                                symbol: 4,
                                input: true,
                            },
                        ),
                        (
                            "B".to_string(),
                            NetPort {
                                symbol: 5,
                                input: true,
                            },
                        ),
                        (
                            "S".to_string(),
                            NetPort {
                                symbol: 6,
                                input: false,
                            },
                        ),
                    ]),
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "A".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "B".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "S".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "S_inter".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "OUTER".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([
                        (
                            "IN1".to_string(),
                            NetPort {
                                symbol: 9,
                                input: true,
                            },
                        ),
                        (
                            "IN2".to_string(),
                            NetPort {
                                symbol: 10,
                                input: true,
                            },
                        ),
                        (
                            "IN3".to_string(),
                            NetPort {
                                symbol: 11,
                                input: true,
                            },
                        ),
                        (
                            "OUT".to_string(),
                            NetPort {
                                symbol: 12,
                                input: false,
                            },
                        ),
                    ]),
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "IN1".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "IN2".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "IN3".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "OUT".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "OUT1".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "OUT2".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "OUT3".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
        ];

        let net1 = Net {
            name: Ident::Symbol(3),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(4),
                        param_type: Type::Int,
                    },
                    span: Span { line: 0, col: 0 },
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(5),
                        param_type: Type::Int,
                    },
                    span: Span { line: 0, col: 0 },
                }),
                NetItem::Output(OutputEnt {
                    param: Param {
                        name: Ident::Symbol(6),
                        param_type: Type::Int,
                    },
                }),
                NetItem::Init(EntInit {
                    param: Param {
                        name: Ident::Symbol(4),
                        param_type: Type::Int,
                    },
                    val: Expr::Literal(Literal::Int(2)),
                }),
                NetItem::RelInst(RelInst {
                    asignee: Ident::Symbol(7),
                    rel: Ident::Symbol(1),
                    args: vec![Ident::Symbol(4), Ident::Symbol(5)],
                    delay: Expr::Literal(Literal::Int(1)),
                    span: Span { line: 0, col: 0 },
                }),
                NetItem::RelInst(RelInst {
                    asignee: Ident::Symbol(6),
                    rel: Ident::Symbol(2),
                    args: vec![Ident::Symbol(7)],
                    delay: Expr::Literal(Literal::Int(1)),
                    span: Span { line: 0, col: 0 },
                }),
            ],
        };
        let net2 = Net {
            name: Ident::Symbol(8),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(9),
                        param_type: Type::Int,
                    },
                    span: Span { line: 0, col: 0 },
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(10),
                        param_type: Type::Int,
                    },
                    span: Span { line: 0, col: 0 },
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(11),
                        param_type: Type::Int,
                    },
                    span: Span { line: 0, col: 0 },
                }),
                NetItem::Output(OutputEnt {
                    param: Param {
                        name: Ident::Symbol(12),
                        param_type: Type::Int,
                    },
                }),
                NetItem::Init(EntInit {
                    param: Param {
                        name: Ident::Symbol(9),
                        param_type: Type::Int,
                    },
                    val: Expr::Literal(Literal::Int(4)),
                }),
                NetItem::NetInst(NetInst {
                    net: Ident::Symbol(3),
                    connections: vec![
                        Connection {
                            port: Ident::Symbol(4),
                            ent: Ident::Symbol(9),
                            span: Span { line: 0, col: 0 },
                        },
                        Connection {
                            port: Ident::Symbol(5),
                            ent: Ident::Symbol(9),
                            span: Span { line: 0, col: 0 },
                        },
                        Connection {
                            port: Ident::Symbol(6),
                            ent: Ident::Symbol(13),
                            span: Span { line: 0, col: 0 },
                        },
                    ],
                    span: Span{line: 0, col: 0},
                }),
                NetItem::NetInst(NetInst {
                    net: Ident::Symbol(3),
                    connections: vec![
                        Connection {
                            port: Ident::Symbol(4),
                            ent: Ident::Symbol(10),
                            span: Span { line: 0, col: 0 },
                        },
                        Connection {
                            port: Ident::Symbol(5),
                            ent: Ident::Symbol(10),
                            span: Span { line: 0, col: 0 },
                        },
                        Connection {
                            port: Ident::Symbol(6),
                            ent: Ident::Symbol(14),
                            span: Span { line: 0, col: 0 },
                        },
                    ],
                    span: Span{line: 0, col: 0},
                }),
                NetItem::NetInst(NetInst {
                    net: Ident::Symbol(3),
                    connections: vec![
                        Connection {
                            port: Ident::Symbol(4),
                            ent: Ident::Symbol(11),
                            span: Span { line: 0, col: 0 },
                        },
                        Connection {
                            port: Ident::Symbol(5),
                            ent: Ident::Symbol(11),
                            span: Span { line: 0, col: 0 },
                        },
                        Connection {
                            port: Ident::Symbol(6),
                            ent: Ident::Symbol(15),
                            span: Span { line: 0, col: 0 },
                        },
                    ],
                    span: Span{line: 0, col: 0},
                }),
                NetItem::RelInst(RelInst {
                    asignee: Ident::Symbol(12),
                    rel: Ident::Symbol(0),
                    args: vec![Ident::Symbol(13), Ident::Symbol(14), Ident::Symbol(15)],
                    delay: Expr::Literal(Literal::Int(1)),
                    span: Span { line: 0, col: 0 },
                }),
            ],
        };

        let net_map = HashMap::from([(0, 0), (1, 1), (2, 2), (3, 0), (8, 1)]);
        let ent_map: HashMap<SymbolId, usize> = HashMap::new();

        // fn synthesize_net(
        // &mut self,
        // target_net_idx: usize,
        // mut ent_map: HashMap<SymbolId, usize>,
        // relations: &mut Vec<Relation>,
        // inits: &mut Vec<Event>,
        // union_find: &mut UnionFind,

        Synthesis::test_synthesize_net(
            vec![net1, net2],
            1,
            net_map,
            ent_map,
            &mut ents,
            &mut relations,
            &mut symbol_table,
            None,
            &mut diagnostics,
        );

        assert_eq!(
            ents,
            vec![
                Entity {
                    // IN1, A(1)
                    val: None,
                    sinks: vec![0],
                },
                Entity {
                    // IN2, A(2)
                    val: None,
                    sinks: vec![2],
                },
                Entity {
                    // IN3, A(3)
                    val: None,
                    sinks: vec![4],
                },
                Entity {
                    // OUT
                    val: None,
                    sinks: vec![],
                },
                Entity {
                    // OUT1, S(1)
                    val: None,
                    sinks: vec![6],
                },
                Entity {
                    // S_inter(1)
                    val: None,
                    sinks: vec![1],
                },
                Entity {
                    // OUT2, S(2)
                    val: None,
                    sinks: vec![6],
                },
                Entity {
                    // S_inter(2)
                    val: None,
                    sinks: vec![3],
                },
                Entity {
                    // OUT3, S(3)
                    val: None,
                    sinks: vec![6],
                },
                Entity {
                    // S_inter(3)
                    val: None,
                    sinks: vec![5],
                },
            ]
        );
        assert_eq!(
            relations,
            vec![
                Relation {
                    idx: 0,
                    delay: 1,
                    input_ents: vec![0, 0],
                    output_ent: 5,
                },
                Relation {
                    idx: 1,
                    delay: 1,
                    input_ents: vec![5],
                    output_ent: 4,
                },
                Relation {
                    idx: 0,
                    delay: 1,
                    input_ents: vec![1, 1],
                    output_ent: 7
                },
                Relation {
                    idx: 1,
                    delay: 1,
                    input_ents: vec![7],
                    output_ent: 6,
                },
                Relation {
                    idx: 0,
                    delay: 1,
                    input_ents: vec![2, 2],
                    output_ent: 9,
                },
                Relation {
                    idx: 1,
                    delay: 1,
                    input_ents: vec![9],
                    output_ent: 8,
                },
                Relation {
                    idx: 2,
                    delay: 1,
                    input_ents: vec![4, 6, 8],
                    output_ent: 3,
                },
            ]
        );
    }

    #[test]
    fn ent_init() {
        // rel_t ADD : (a: Int, b: Int) -> Int = a + b;

        // net ADD_NET {
        //     input A: Int;
        //     input B: Int;
        //     init A: Int = 5;
        //     init B: Int = 2 + 2;
        //     output S: Int;

        //     S = ADD(A, B);
        // }
        let mut diagnostics = Diagnostics::new();
        let mut ents: Vec<Entity> = Vec::new();
        let mut relations: Vec<Relation> = Vec::new();

        let mut symbol_table = vec![
            Symbol {
                name: "REL".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Int, Type::Int],
                    return_type: Type::Int,
                    
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "ADD_NET".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([
                        (
                            "A".to_string(),
                            NetPort {
                                symbol: 2,
                                input: true,
                            },
                        ),
                        (
                            "B".to_string(),
                            NetPort {
                                symbol: 3,
                                input: true,
                            },
                        ),
                        (
                            "S".to_string(),
                            NetPort {
                                symbol: 4,
                                input: false,
                            },
                        ),
                    ]),
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "A".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "B".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "S".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
        ];

        let net = Net {
            name: Ident::Symbol(1),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(2),
                        param_type: Type::Int,
                    },
                    span: Span { line: 0, col: 0 },
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(3),
                        param_type: Type::Int,
                    },
                    span: Span { line: 0, col: 0 },
                }),
                NetItem::Init(EntInit {
                    param: Param {
                        name: Ident::Symbol(2),
                        param_type: Type::Int,
                    },
                    val: Expr::Literal(Literal::Int(5)),
                }),
                NetItem::Init(EntInit {
                    param: Param {
                        name: Ident::Symbol(3),
                        param_type: Type::Int,
                    },
                    val: Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::Literal(Literal::Int(2))),
                        right: Box::new(Expr::Literal(Literal::Int(2))),
                        op: BinaryOp::Add,
                        op_span: Span { line: 0, col: 0 },
                        expr_type: Type::Int,
                    }),
                }),
                NetItem::Output(OutputEnt {
                    param: Param {
                        name: Ident::Symbol(4),
                        param_type: Type::Int,
                    },
                }),
                NetItem::RelInst(RelInst {
                    asignee: Ident::Symbol(4),
                    rel: Ident::Symbol(0),
                    args: vec![Ident::Symbol(2), Ident::Symbol(3)],
                    delay: Expr::Literal(Literal::Int(1)),
                    span: Span { line: 0, col: 0 },
                }),
            ],
        };
        let net_map = HashMap::from([(0, 0), (1, 0)]);
        let ent_map: HashMap<SymbolId, usize> = HashMap::new();

        Synthesis::test_synthesize_net(
            vec![net],
            0,
            net_map,
            ent_map,
            &mut ents,
            &mut relations,
            &mut symbol_table,
            None,
            &mut diagnostics,
        );

        assert_eq!(
            ents,
            vec![
                Entity {
                    val: None,
                    sinks: vec![0],
                },
                Entity {
                    val: None,
                    sinks: vec![0],
                },
                Entity {
                    val: None,
                    sinks: vec![],
                },
            ]
        );
        assert_eq!(
            relations,
            vec![Relation {
                idx: 0,
                delay: 1,
                input_ents: vec![0, 1],
                output_ent: 2,
            },]
        );
    }

    #[test]
    fn synthesize() {
        // rel_t ADD : (a: Int, b: Int) -> Int = a + b;

        // net ADD_NET {
        //     input A: Int;
        //     input B: Int;

        //     output S: Int;

        //     init A: Int = 1;

        //     S = ADD(A, B);
        // }
        let mut diagnostics = Diagnostics::new();

        let mut symbol_table = vec![
            Symbol {
                name: "REL".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Int, Type::Int],
                    return_type: Type::Int,
                    
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "ADD_NET".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([
                        (
                            "A".to_string(),
                            NetPort {
                                symbol: 2,
                                input: true,
                            },
                        ),
                        (
                            "B".to_string(),
                            NetPort {
                                symbol: 3,
                                input: true,
                            },
                        ),
                        (
                            "S".to_string(),
                            NetPort {
                                symbol: 4,
                                input: false,
                            },
                        ),
                    ]),
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "A".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "B".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "S".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
        ];

        let net = Net {
            name: Ident::Symbol(1),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(2),
                        param_type: Type::Int,
                    },
                    span: Span { line: 0, col: 0 },
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(3),
                        param_type: Type::Int,
                    },
                    span: Span { line: 0, col: 0 },
                }),
                NetItem::Output(OutputEnt {
                    param: Param {
                        name: Ident::Symbol(4),
                        param_type: Type::Int,
                    },
                }),
                NetItem::Init(EntInit {
                    param: Param {
                        name: Ident::Symbol(2),
                        param_type: Type::Int,
                    },
                    val: Expr::Literal(Literal::Int(1)),
                }),
                NetItem::RelInst(RelInst {
                    asignee: Ident::Symbol(4),
                    rel: Ident::Symbol(0),
                    args: vec![Ident::Symbol(2), Ident::Symbol(3)],
                    delay: Expr::Literal(Literal::Int(1)),
                    span: Span { line: 0, col: 0 },
                }),
            ],
        };
        let net_map = HashMap::from([(0, 0), (1, 0)]);

        let (netlist, interface, inits, _) =
            Synthesis::new(vec![net], net_map, &mut symbol_table, None, &mut diagnostics).synthesize(0);

        assert_eq!(
            netlist,
            Netlist {
                relations: vec![Relation {
                    idx: 0,
                    delay: 1,
                    input_ents: vec![0, 1],
                    output_ent: 2,
                },],
                ents: vec![
                    Entity {
                        val: None,
                        sinks: vec![0],
                    },
                    Entity {
                        val: None,
                        sinks: vec![0],
                    },
                    Entity {
                        val: None,
                        sinks: vec![],
                    },
                ]
            }
        );
        assert_eq!(
            interface,
            Interface {
                inputs: HashMap::from([
                    ("A".to_string(), (0, Type::Int)),
                    ("B".to_string(), (1, Type::Int)),
                ]),
                outputs: HashMap::from([(2, ("S".to_string(), Type::Int)),]),
                custom_type_maps: HashMap::new(),
            }
        );
        assert_eq!(
            inits,
            vec![Event {
                timestep: 0,
                ent_id: 0,
                new_val: 1,
            },]
        );
    }

    #[test]
    fn float_init() {
        // net NET {
        //     output A: Float;

        //     init A: Float = 1;
        // }
        let mut diagnostics = Diagnostics::new();

        let mut symbol_table = vec![
            Symbol {
                name: "NET".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([(
                        "A".to_string(),
                        NetPort {
                            symbol: 2,
                            input: false,
                        },
                    )]),
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "A".to_string(),
                kind: SymbolKind::Ent(Type::Float),
                span: Span { line: 0, col: 0 },
            },
        ];

        let net = Net {
            name: Ident::Symbol(1),
            items: vec![
                NetItem::Output(OutputEnt {
                    param: Param {
                        name: Ident::Symbol(1),
                        param_type: Type::Float,
                    },
                }),
                NetItem::Init(EntInit {
                    param: Param {
                        name: Ident::Symbol(1),
                        param_type: Type::Float,
                    },
                    val: Expr::Literal(Literal::Float(1.0)),
                }),
            ],
        };
        let net_map = HashMap::from([(0, 0), (1, 0)]);

        let (_netlist, _interface, inits, _) =
            Synthesis::new(vec![net], net_map, &mut symbol_table, None, &mut diagnostics).synthesize(0);

        assert_eq!(
            inits,
            vec![Event {
                timestep: 0,
                ent_id: 0,
                new_val: 1.0_f64.to_bits() as u64,
            },]
        );
    }

    #[test]
    fn impulse_init() {
        // net NET {
        //     output A: Impulse;

        //     init A: Impulse = true;
        // }
        let mut diagnostics = Diagnostics::new();

        let mut symbol_table = vec![
            Symbol {
                name: "NET".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([(
                        "A".to_string(),
                        NetPort {
                            symbol: 2,
                            input: false,
                        },
                    )]),
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "A".to_string(),
                kind: SymbolKind::Ent(Type::Impulse),
                span: Span { line: 0, col: 0 },
            },
        ];

        let net = Net {
            name: Ident::Symbol(1),
            items: vec![
                NetItem::Output(OutputEnt {
                    param: Param {
                        name: Ident::Symbol(1),
                        param_type: Type::Impulse,
                    },
                }),
                NetItem::Init(EntInit {
                    param: Param {
                        name: Ident::Symbol(1),
                        param_type: Type::Impulse,
                    },
                    val: Expr::Literal(Literal::Bool(true)),
                }),
            ],
        };
        let net_map = HashMap::from([(0, 0), (1, 0)]);

        let (_netlist, _interface, inits, _) =
            Synthesis::new(vec![net], net_map, &mut symbol_table, None, &mut diagnostics).synthesize(0);

        assert_eq!(
            inits,
            vec![Event {
                timestep: 0,
                ent_id: 0,
                new_val: 0 as u64,
            },]
        );
    }

    #[test]
    fn mod_init() {
        // net NET {
        //     output A: Mod(3);

        //     init A: Mod(3) = 4;
        // }
        let mut diagnostics = Diagnostics::new();

        let mut symbol_table = vec![
            Symbol {
                name: "NET".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([(
                        "A".to_string(),
                        NetPort {
                            symbol: 2,
                            input: false,
                        },
                    )]),
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "A".to_string(),
                kind: SymbolKind::Ent(Type::Mod(3)),
                span: Span { line: 0, col: 0 },
            },
        ];

        let net = Net {
            name: Ident::Symbol(1),
            items: vec![
                NetItem::Output(OutputEnt {
                    param: Param {
                        name: Ident::Symbol(1),
                        param_type: Type::Mod(3),
                    },
                }),
                NetItem::Init(EntInit {
                    param: Param {
                        name: Ident::Symbol(1),
                        param_type: Type::Mod(3),
                    },
                    val: Expr::Literal(Literal::Int(4)),
                }),
            ],
        };
        let net_map = HashMap::from([(0, 0), (1, 0)]);

        let (_netlist, _interface, inits, _) =
            Synthesis::new(vec![net], net_map, &mut symbol_table, None, &mut diagnostics).synthesize(0);

        assert_eq!(
            inits,
            vec![Event {
                timestep: 0,
                ent_id: 0,
                new_val: 1 as u64,
            },]
        );
    }

    #[test]
    fn custom_init() {
        // ent_t COIN = {H, T};
        // net NET {
        //     output A: COIN;

        //     init A: COIN = T;
        // }
        let mut diagnostics = Diagnostics::new();

        let mut symbol_table = vec![
            Symbol {
                name: "COIN".to_string(),
                kind: SymbolKind::EntType,
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "H".to_string(),
                kind: SymbolKind::EntMember {
                    parent: 0,
                    mapping: 0,
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "T".to_string(),
                kind: SymbolKind::EntMember {
                    parent: 0,
                    mapping: 1,
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "NET".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([(
                        "A".to_string(),
                        NetPort {
                            symbol: 4,
                            input: false,
                        },
                    )]),
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "A".to_string(),
                kind: SymbolKind::Ent(Type::Float),
                span: Span { line: 0, col: 0 },
            },
        ];

        let net = Net {
            name: Ident::Symbol(1),
            items: vec![
                NetItem::Output(OutputEnt {
                    param: Param {
                        name: Ident::Symbol(4),
                        param_type: Type::Custom(Ident::Symbol(0)),
                    },
                }),
                NetItem::Init(EntInit {
                    param: Param {
                        name: Ident::Symbol(4),
                        param_type: Type::Custom(Ident::Symbol(0)),
                    },
                    val: Expr::Ident(Ident::Symbol(2)),
                }),
            ],
        };
        let net_map = HashMap::from([(0, 0), (1, 0)]);

        let (_netlist, interface, inits, _) =
            Synthesis::new(vec![net], net_map, &mut symbol_table, None, &mut diagnostics).synthesize(0);

        assert_eq!(
            inits,
            vec![Event {
                timestep: 0,
                ent_id: 0,
                new_val: 1 as u64,
            },]
        );
        assert_eq!(
            interface.custom_type_maps,
            HashMap::from([(
                "COIN".to_string(),
                vec![("H".to_string(), 0), ("T".to_string(), 1),]
            ),])
        );
    }

    #[test]
    fn ent_alias() {
        // rel_t ADD : (a: Int, b: Int) -> Int = a + b;

        // net ADD_NET {
        //     input A: Int;
        //     input B: Int;

        //     output S: Int;

        //     A := D:

        //     let n = sample {1.0: 1, _ : 0};

        //     init D: Int = n;

        //     S = ADD(A, B);
        // }
        let mut diagnostics = Diagnostics::new();

        let mut symbol_table = vec![
            Symbol {
                name: "REL".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Int, Type::Int],
                    return_type: Type::Int,
                    
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "ADD_NET".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([
                        (
                            "A".to_string(),
                            NetPort {
                                symbol: 2,
                                input: true,
                            },
                        ),
                        (
                            "B".to_string(),
                            NetPort {
                                symbol: 3,
                                input: true,
                            },
                        ),
                        (
                            "S".to_string(),
                            NetPort {
                                symbol: 4,
                                input: false,
                            },
                        ),
                    ]),
                },
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "A".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "B".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "S".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "D".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span { line: 0, col: 0 },
            },
            Symbol {
                name: "n".to_string(),
                kind: SymbolKind::Variable(Type::Int),
                span: Span { line: 0, col: 0 },
            },
        ];

        let net = Net {
            name: Ident::Symbol(1),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(2),
                        param_type: Type::Int,
                    },
                    span: Span { line: 0, col: 0 },
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(3),
                        param_type: Type::Int,
                    },
                    span: Span { line: 0, col: 0 },
                }),
                NetItem::Output(OutputEnt {
                    param: Param {
                        name: Ident::Symbol(4),
                        param_type: Type::Int,
                    },
                }),
                NetItem::Alias(EntAlias {
                    left: Ident::Symbol(2),
                    right: Ident::Symbol(5),
                    span: Span { line: 0, col: 0 },
                }),
                NetItem::Init(EntInit {
                    param: Param {
                        name: Ident::Symbol(5),
                        param_type: Type::Int,
                    },
                    val: Expr::Literal(Literal::Int(1)),
                }),
                NetItem::Stmt(Statement::Var(Variable {
                    name: Ident::Symbol(6),
                    expr: Expr::Sample(SampleExpr {
                        arms: vec![
                            SampleArm {
                                prob: Prob::Expr(Expr::Literal(Literal::Float(1.0))),
                                expr: Expr::Literal(Literal::Int(1)),
                                arm_span: Span { line: 0, col: 0 },
                            },
                            SampleArm {
                                prob: Prob::Default,
                                expr: Expr::Literal(Literal::Int(0)),
                                arm_span: Span { line: 0, col: 0 },
                            },
                        ],
                        expr_type: Type::Int,
                        span: Span { line: 0, col: 0 },
                    }),
                })),
                NetItem::RelInst(RelInst {
                    asignee: Ident::Symbol(4),
                    rel: Ident::Symbol(0),
                    args: vec![Ident::Symbol(2), Ident::Symbol(3)],
                    delay: Expr::Literal(Literal::Int(1)),
                    span: Span { line: 0, col: 0 },
                }),
            ],
        };
        let net_map = HashMap::from([(0, 0), (1, 0)]);

        let (netlist, interface, inits, _) =
            Synthesis::new(vec![net], net_map, &mut symbol_table, None, &mut diagnostics).synthesize(0);

        assert_eq!(
            netlist,
            Netlist {
                relations: vec![Relation {
                    idx: 0,
                    delay: 1,
                    input_ents: vec![0, 1],
                    output_ent: 2,
                },],
                ents: vec![
                    Entity {
                        val: None,
                        sinks: vec![0],
                    },
                    Entity {
                        val: None,
                        sinks: vec![0],
                    },
                    Entity {
                        val: None,
                        sinks: vec![],
                    },
                ]
            }
        );
        assert_eq!(
            interface,
            Interface {
                inputs: HashMap::from([
                    ("A".to_string(), (0, Type::Int)),
                    ("B".to_string(), (1, Type::Int)),
                ]),
                outputs: HashMap::from([(2, ("S".to_string(), Type::Int)),]),
                custom_type_maps: HashMap::new(),
            }
        );
        assert_eq!(
            inits,
            vec![Event {
                timestep: 0,
                ent_id: 0,
                new_val: 1,
            },]
        );
    }
}
