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
    pub fn synthesize(nets: Vec<Net>, top_net_idx: usize, mut obj_map: HashMap::<SymbolId, usize>, symbols: &[Symbol], diagnostics: &mut Diagnostics) -> Netlist{
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        let mut ents = Vec::new();
        let mut relations = Vec::new();

        let mut ent_map: HashMap<SymbolId, usize> = HashMap::new();
        for item in &nets[top_net_idx].items {
            match item {
                NetItem::Input(input) => {
                    let Ident::Symbol(id) = input.param.name else {
                        unreachable!("should not be ident string");
                    };

                    let idx = ents.len();

                    inputs.push((symbols[id].name.to_string(), idx));
                    ent_map.insert(id, idx);
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

                    outputs.push((symbols[id].name.to_string(), idx));
                    ent_map.insert(id, idx);
                    ents.push(Entity {
                        val: None,
                        sinks: vec![],
                    });
                }

                _ => {}
            }
        }

        // call synthesize_net

        Netlist {
            inputs,
            outputs,
            relations,
            ents,
        }
    }

    fn synthesize_net(nets: &[Net], target_net_idx: usize, obj_map: &HashMap::<SymbolId, usize>, mut ent_map: HashMap::<SymbolId, usize>, ents: &mut Vec<Entity>, relations: &mut Vec<Relation>) {
        // make obj_map an immutable reference that honly maps for nets and rels
        // make new mutable ent_map that is passed in (not a reference) and a new one is created for each net instantiation
        //  to allow for the interal nets of a net be different ents in the netlist for each seperate instantiation of that net

        for item in &nets[target_net_idx].items {
            match item {
                NetItem::Input(input) => {
                    let Ident::Symbol(id) = input.param.name else {
                        unreachable!("should not be ident string");
                    };

                    Self::insert_or_get_ent(id, &mut ent_map, ents);
                }

                NetItem::Output(input) => {
                    let Ident::Symbol(id) = input.param.name else {
                        unreachable!("should not be ident string");
                    };

                    Self::insert_or_get_ent(id, &mut ent_map, ents);
                }

                NetItem::Init(init) => {
                    let Ident::Symbol(id) = init.param.name else {
                        unreachable!("should not be ident string");
                    };

                    let idx = Self::insert_or_get_ent(id, &mut ent_map, ents);

                    // fold expression
                    let folded_val = 0;

                    ents[idx].val = Some(folded_val);
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

                    let asignee_idx = Self::insert_or_get_ent(asignee_id, &mut ent_map, ents);

                    let mut input_indices = Vec::new();
                    for arg in &rel.args {
                        let Ident::Symbol(arg_id) = arg else {
                            unreachable!("should not be ident string");
                        };

                        let arg_idx = Self::insert_or_get_ent(*arg_id, &mut ent_map, ents);

                        // if the same ent is used as multiple inputs to a relation, this if check prevents 
                        //  the relation from occuring multiple times in the sinks of the ent
                        if !ents[arg_idx].sinks.contains(&relations.len()) {
                            ents[arg_idx].sinks.push(relations.len());
                        }

                        input_indices.push(arg_idx);
                    }

                    relations.push(Relation {
                        idx: rel_idx,
                        input_ents: input_indices,
                        output_ent: asignee_idx,
                    });
                }

                NetItem::NetInst(net) => {
                    let mut sub_net_map: HashMap<SymbolId, usize> = HashMap::new();

                    let Ident::Symbol(net_id) = net.net else {
                        unreachable!("should not be ident string");
                    };
                    let Some(&net_idx) = obj_map.get(&net_id) else {
                        unreachable!("net should be in obj_map");
                    };

                    for connection in &net.connections {
                        let Ident::Symbol(ent_id) = connection.ent else {
                            unreachable!("should not be ident string");
                        };
                        let Ident::Symbol(port_id) = connection.port else {
                            unreachable!("should not be ident string");
                        };

                        let ent_idx = Self::insert_or_get_ent(ent_id, &mut ent_map, ents);

                        // both the port and the connected ent are the same ent in the netlist
                        sub_net_map.insert(port_id, ent_idx);
                    }

                    Self::synthesize_net(nets, net_idx, obj_map, sub_net_map, ents, relations);
                }

                NetItem::Error => {}
            }
        }
    }

    fn insert_or_get_ent(id: SymbolId, ent_map: &mut HashMap::<SymbolId, usize>, ents: &mut Vec<Entity>) -> usize {
        match ent_map.get(&id) {
            Some(&idx) => idx,
            None => {
                let idx = ents.len();

                ent_map.insert(id, idx);

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
    use crate::compiler::symbol::{SymbolKind, NetPort};
    use crate::compiler::sem_analyzer::types::Type;
    use crate::compiler::ast::*;
    use crate::compiler::diagnostics::Span;

    #[test]
    fn rel_inst() {
        // rel_t ADD : (a: Int, b: Int) -> Int = a + b;

        // net ADD_NET {
        //     input A: Int;
        //     input B: Int;
        //     input C: Int;
        //     output S: Int;

        //     S = ADD(A, B);
        // }
        let mut diagnostics = Diagnostics::new();
        let mut ents: Vec<Entity> = Vec::new();
        let mut relations: Vec<Relation> = Vec::new();

        let symbol_table = vec![
            Symbol {
                name: "REL".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Int, Type::Int],
                    return_type: Type::Int,
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "ADD_NET".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([
                        ("A".to_string(), NetPort {
                            symbol: 2,
                            input: true,
                        }),
                        ("B".to_string(), NetPort {
                            symbol: 3,
                            input: true,
                        }),
                        ("C".to_string(), NetPort {
                            symbol: 4,
                            input: true,
                        }),
                        ("S".to_string(), NetPort {
                            symbol: 5,
                            input: false,
                        }),
                    ]),
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "A".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "B".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "C".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "S".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span{line: 0, col: 0},
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
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(3),
                        param_type: Type::Int,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(4),
                        param_type: Type::Int,
                    },
                    span: Span{line: 0, col: 0},
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
                    args: vec![
                        Ident::Symbol(2),
                        Ident::Symbol(3),
                    ],
                    span: Span{line: 0, col: 0},
                }),
            ],
        };
        let obj_map = HashMap::from([
            (0, 0),
            (1, 0),
        ]);
        let ent_map: HashMap<SymbolId, usize> = HashMap::new();

        Synthesis::synthesize_net(&vec![net], 0, &obj_map, ent_map, &mut ents, &mut relations);

        assert_eq!(ents, vec![
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
        ]);
        assert_eq!(relations, vec![
            Relation {
                idx: 0,
                input_ents: vec![0, 1],
                output_ent: 3,
            },
        ]);
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

        let symbol_table = vec![
            Symbol {
                name: "REL".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Int, Type::Int],
                    return_type: Type::Int,
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "ADD_NET".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([
                        ("A".to_string(), NetPort {
                            symbol: 2,
                            input: true,
                        }),
                        ("B".to_string(), NetPort {
                            symbol: 3,
                            input: true,
                        }),
                        ("C".to_string(), NetPort {
                            symbol: 4,
                            input: true,
                        }),
                        ("S".to_string(), NetPort {
                            symbol: 5,
                            input: false,
                        }),
                    ]),
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "A".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "B".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "C".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "S".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "ADD_TO_SELF".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([
                        ("IN".to_string(), NetPort {
                            symbol: 7,
                            input: true,
                        }),
                        ("OUT".to_string(), NetPort {
                            symbol: 8,
                            input: false,
                        }),
                    ]),
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "IN".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "OUT".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span{line: 0, col: 0},
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
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(3),
                        param_type: Type::Int,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(4),
                        param_type: Type::Int,
                    },
                    span: Span{line: 0, col: 0},
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
                    args: vec![
                        Ident::Symbol(2),
                        Ident::Symbol(3),
                    ],
                    span: Span{line: 0, col: 0},
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
                    span: Span{line: 0, col: 0},
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
                            span: Span{line: 0, col: 0},
                        },
                        Connection {
                            port: Ident::Symbol(3),
                            ent: Ident::Symbol(7),
                            span: Span{line: 0, col: 0},
                        },
                        Connection {
                            port: Ident::Symbol(5),
                            ent: Ident::Symbol(8),
                            span: Span{line: 0, col: 0},
                        },
                    ],
                }),
            ],
        };
        
        let obj_map = HashMap::from([
            (0, 0),
            (1, 0),
            (6, 1),
        ]);
        let ent_map: HashMap<SymbolId, usize> = HashMap::new();

        Synthesis::synthesize_net(&vec![net1, net2], 1, &obj_map, ent_map, &mut ents, &mut relations);

        assert_eq!(ents, vec![
            Entity { // IN, A, B
                val: None,
                sinks: vec![0],
            },
            Entity { // OUT, S
                val: None,
                sinks: vec![],
            },
            Entity { // C
                val: None,
                sinks: vec![],
            },
        ]);
        assert_eq!(relations, vec![
            Relation {
                idx: 0,
                input_ents: vec![0, 0],
                output_ent: 1,
            },
        ]);
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

        //     S_inter = MUL(A, B);
        //     S = MUL_2(S_inter);
        // }

        // net OUTER {
        //     input IN1: Int;
        //     input IN2: Int;
        //     input IN3: Int;
        //     output OUT: Int;

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

        let symbol_table = vec![
            Symbol {
                name: "ADD".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Int, Type::Int, Type::Int],
                    return_type: Type::Int,
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "MUL".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Int, Type::Int],
                    return_type: Type::Int,
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "MUL_2".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Int],
                    return_type: Type::Int,
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "INNER".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([
                        ("A".to_string(), NetPort {
                            symbol: 4,
                            input: true,
                        }),
                        ("B".to_string(), NetPort {
                            symbol: 5,
                            input: true,
                        }),
                        ("S".to_string(), NetPort {
                            symbol: 6,
                            input: false,
                        }),
                    ]),
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "A".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "B".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "S".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "S_inter".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "OUTER".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([
                        ("IN1".to_string(), NetPort {
                            symbol: 9,
                            input: true,
                        }),
                        ("IN2".to_string(), NetPort {
                            symbol: 10,
                            input: true,
                        }),
                        ("IN3".to_string(), NetPort {
                            symbol: 11,
                            input: true,
                        }),
                        ("OUT".to_string(), NetPort {
                            symbol: 12,
                            input: false,
                        }),
                    ]),
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "IN1".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "IN2".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "IN3".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "OUT".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "OUT1".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "OUT2".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "OUT3".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span{line: 0, col: 0},
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
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(5),
                        param_type: Type::Int,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Output(OutputEnt {
                    param: Param {
                        name: Ident::Symbol(6),
                        param_type: Type::Int,
                    },
                }),
                NetItem::RelInst(RelInst {
                    asignee: Ident::Symbol(7),
                    rel: Ident::Symbol(1),
                    args: vec![
                        Ident::Symbol(4),
                        Ident::Symbol(5),
                    ],
                    span: Span{line: 0, col: 0},
                }),
                NetItem::RelInst(RelInst {
                    asignee: Ident::Symbol(6),
                    rel: Ident::Symbol(2),
                    args: vec![
                        Ident::Symbol(7),
                    ],
                    span: Span{line: 0, col: 0},
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
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(10),
                        param_type: Type::Int,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(11),
                        param_type: Type::Int,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Output(OutputEnt {
                    param: Param {
                        name: Ident::Symbol(12),
                        param_type: Type::Int,
                    },
                }),
                NetItem::NetInst(NetInst {
                    net: Ident::Symbol(3),
                    connections: vec![
                        Connection {
                            port: Ident::Symbol(4),
                            ent: Ident::Symbol(9),
                            span: Span{line: 0, col: 0},
                        },
                        Connection {
                            port: Ident::Symbol(5),
                            ent: Ident::Symbol(9),
                            span: Span{line: 0, col: 0},
                        },
                        Connection {
                            port: Ident::Symbol(6),
                            ent: Ident::Symbol(13),
                            span: Span{line: 0, col: 0},
                        },
                    ],
                }),
                NetItem::NetInst(NetInst {
                    net: Ident::Symbol(3),
                    connections: vec![
                        Connection {
                            port: Ident::Symbol(4),
                            ent: Ident::Symbol(10),
                            span: Span{line: 0, col: 0},
                        },
                        Connection {
                            port: Ident::Symbol(5),
                            ent: Ident::Symbol(10),
                            span: Span{line: 0, col: 0},
                        },
                        Connection {
                            port: Ident::Symbol(6),
                            ent: Ident::Symbol(14),
                            span: Span{line: 0, col: 0},
                        },
                    ],
                }),
                NetItem::NetInst(NetInst {
                    net: Ident::Symbol(3),
                    connections: vec![
                        Connection {
                            port: Ident::Symbol(4),
                            ent: Ident::Symbol(11),
                            span: Span{line: 0, col: 0},
                        },
                        Connection {
                            port: Ident::Symbol(5),
                            ent: Ident::Symbol(11),
                            span: Span{line: 0, col: 0},
                        },
                        Connection {
                            port: Ident::Symbol(6),
                            ent: Ident::Symbol(15),
                            span: Span{line: 0, col: 0},
                        },
                    ],
                }),
                NetItem::RelInst(RelInst {
                    asignee: Ident::Symbol(12),
                    rel: Ident::Symbol(0),
                    args: vec![
                        Ident::Symbol(13),
                        Ident::Symbol(14),
                        Ident::Symbol(15),
                    ],
                    span: Span{line: 0, col: 0},
                }),
            ],
        };
        
        let obj_map = HashMap::from([
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 0),
            (8, 1),
        ]);
        let ent_map: HashMap<SymbolId, usize> = HashMap::new();

        Synthesis::synthesize_net(&vec![net1, net2], 1, &obj_map, ent_map, &mut ents, &mut relations);

        assert_eq!(ents, vec![
            Entity { // IN1, A(1)
                val: None,
                sinks: vec![0],
            },
            Entity { // IN2, A(2)
                val: None,
                sinks: vec![2],
            },
            Entity { // IN3, A(3)
                val: None,
                sinks: vec![4],
            },
            Entity { // OUT
                val: None,
                sinks: vec![],
            },
            Entity { // OUT1, S(1)
                val: None,
                sinks: vec![6],
            },
            Entity { // S_inter(1)
                val: None,
                sinks: vec![1],
            },
            Entity { // OUT2, S(2)
                val: None,
                sinks: vec![6],
            },
            Entity { // S_inter(2)
                val: None,
                sinks: vec![3],
            },
            Entity { // OUT3, S(3)
                val: None,
                sinks: vec![6],
            },
            Entity { // S_inter(3)
                val: None,
                sinks: vec![5],
            },
        ]);
        assert_eq!(relations, vec![
            Relation {
                idx: 1,
                input_ents: vec![0, 0],
                output_ent: 5,
            },
            Relation {
                idx: 2,
                input_ents: vec![5],
                output_ent: 4,
            },
            Relation {
                idx: 1,
                input_ents: vec![1, 1],
                output_ent: 7
            },
            Relation {
                idx: 2,
                input_ents: vec![7],
                output_ent: 6,
            },
            Relation {
                idx: 1,
                input_ents: vec![2, 2],
                output_ent: 9,
            },
            Relation {
                idx: 2,
                input_ents: vec![9],
                output_ent: 8,
            },
            Relation {
                idx: 0,
                input_ents: vec![4, 6, 8],
                output_ent: 3,
            },
        ]);
    }
}