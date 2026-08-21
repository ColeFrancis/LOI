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

//! # check_constraints
//!
//! Handles final error checking before the compiler enters backend stages
//!
//! ## Invariants
//!
//! - After this point, all errors catchable by the front-end will be caught
//!
//! Author: Cole Francis

use std::collections::HashSet;

use super::SemAnalyzer;

use crate::compiler::{
    ast::*,
    symbol::{SymbolId, SymbolKind},
    diagnostics::{CompilerError, Span},
};

impl <'a> SemAnalyzer<'a> {
    pub(super) fn check_constraints(&mut self) {
        let items = std::mem::take(&mut self.ast.items);
        self.ast.items = Vec::with_capacity(items.len());

        for item in items {
            let checked_item = self.check_constraints_item(item).unwrap_or(Item::Error);
            self.ast.items.push(checked_item);
        }
    }

    fn check_constraints_item(&mut self, item: Item) -> Option<Item> {
        match item {
            Item::Net(net) => self.check_constraints_net(net).map(Item::Net),
            
            other => Some(other),
        }
    }

    // TODO: reason more about if I need to check more than just multiple driver errors
    fn check_constraints_net(&mut self, mut net: Net) -> Option<Net> { 
        let mut has_errors = false; 
        let mut driven_ents: Vec<(SymbolId, Span)> = Vec::new();

        for item in &net.items {
            match item {
                NetItem::Input(input_ent) => {
                    let Ident::Symbol(symbol_id) = input_ent.param.name else {
                        return None; // Not reachable
                    };

                    if let Some((_, old_span)) = driven_ents.iter().find(|(id, _)| *id == symbol_id) {
                        self.diagnostics.error(CompilerError::MultipleEntDrivers {
                            name: self.symbols[symbol_id].name.to_string(),
                            first_span: old_span.clone(),
                            last_span: input_ent.span.clone(),
                        });

                        has_errors = true;
                    }
                    driven_ents.push((symbol_id, input_ent.span));
                }

                NetItem::RelInst(rel_inst) => {
                    let Ident::Symbol(asignee_symbol_id) = rel_inst.asignee else {
                        return None; // Not reachable
                    };

                    if let Some((_, old_span)) = driven_ents.iter().find(|(id, _)| *id == asignee_symbol_id) {
                        self.diagnostics.error(CompilerError::MultipleEntDrivers {
                            name: self.symbols[asignee_symbol_id].name.to_string(),
                            first_span: old_span.clone(),
                            last_span: rel_inst.span.clone(),
                        });

                        has_errors = true;
                    }
                    driven_ents.push((asignee_symbol_id, rel_inst.span));
                }

                NetItem::NetInst(net_inst) => {
                    let Ident::Symbol(net_symbol_id) = net_inst.net else {
                        return None; // not reachable
                    };

                    let SymbolKind::Net{ports} = &self.symbols[net_symbol_id].kind else {
                        return None; // not reachable
                    };

                    for connection in &net_inst.connections {
                        let Ident::Symbol(port_symbol_id) = connection.port else {
                            return None; // not reachable
                        };

                        let Some(net_port) = ports.get(&self.symbols[port_symbol_id].name) else {
                            return None; // not reachable
                        };

                        if !net_port.input {
                            let Ident::Symbol(ent_symbol_id) = connection.ent else {
                                return None; // not reachable
                            };

                            if let Some((_, old_span)) = driven_ents.iter().find(|(id, _)| *id == ent_symbol_id) {
                                self.diagnostics.error(CompilerError::MultipleEntDrivers {
                                    name: self.symbols[ent_symbol_id].name.to_string(),
                                    first_span: old_span.clone(),
                                    last_span: connection.span.clone(),
                                });

                                has_errors = true;
                            }
                            driven_ents.push((ent_symbol_id, connection.span));
                        }
                    }
                }

                _ => {}
            }
        }

        if has_errors {
            None
        }
        else {
            Some(net)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    use crate::compiler::sem_analyzer::scope::Scope;
    use crate::compiler::symbol::{Symbol, NetPort};
    use crate::compiler::diagnostics::Diagnostics;
    use crate::compiler::sem_analyzer::types::Type;

    #[test]
    fn test_net_1() {
        // net NET {
        //     input a: Bool;
        //     output b: Bool;

        //     b := REL(a);
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "REL".to_string(),
                    kind: SymbolKind::Rel_t {
                        input_types: vec![Type::Bool],
                        return_type: Type::Bool,
                    },
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "NET".to_string(),
                    kind: SymbolKind::Net {
                        ports: HashMap::from([
                            ("a".to_string(), NetPort {
                                symbol: 2,
                                input: true,
                            }),
                            ("b".to_string(), NetPort {
                                symbol: 3,
                                input: false,
                            }),
                        ]),
                    },
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Ent(Type::Bool),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "b".to_string(),
                    kind: SymbolKind::Ent(Type::Bool),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("REL".to_string(), 0),
                        ("NET".to_string(), 1),
                        ("a".to_string(), 2),
                        ("b".to_string(), 3),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.check_constraints_net(Net {
            name: Ident::Symbol(1),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(2),
                        param_type: Type::Bool,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Output(OutputEnt {
                    param: Param {
                        name: Ident::Symbol(3),
                        param_type: Type::Bool,
                    },
                }),
                NetItem::RelInst(RelInst {
                    asignee: Ident::Symbol(3),
                    rel: Ident::Symbol(0),
                    args: vec![Ident::Symbol(2)],
                    span: Span{line: 0, col: 0},
                }),
            ],
        });

        assert_eq!(diagnostics.num_errors(), 0);
    }

    #[test]
    fn test_net_2() {
        // net NET {
        //     input a: Bool;
        //     input b: Bool; // b driven twice (input, and return of REL)

        //     b := REL(a);
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "REL".to_string(),
                    kind: SymbolKind::Rel_t {
                        input_types: vec![Type::Bool],
                        return_type: Type::Bool,
                    },
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "NET".to_string(),
                    kind: SymbolKind::Net {
                        ports: HashMap::from([
                            ("a".to_string(), NetPort {
                                symbol: 2,
                                input: true,
                            }),
                            ("b".to_string(), NetPort {
                                symbol: 3,
                                input: true,
                            }),
                        ]),
                    },
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Ent(Type::Bool),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "b".to_string(),
                    kind: SymbolKind::Ent(Type::Bool),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("REL".to_string(), 0),
                        ("NET".to_string(), 1),
                        ("a".to_string(), 2),
                        ("b".to_string(), 3),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.check_constraints_net(Net {
            name: Ident::Symbol(1),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(2),
                        param_type: Type::Bool,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(3),
                        param_type: Type::Bool,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::RelInst(RelInst {
                    asignee: Ident::Symbol(3),
                    rel: Ident::Symbol(0),
                    args: vec![Ident::Symbol(2)],
                    span: Span{line: 0, col: 0},
                }),
            ],
        });

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn test_net_3() {
        // net NET {
        //     input a: Bool;
        //     input b: Bool;

        //     b := REL(a);
        //     NET2 {
        //         B := b, // B is an output, b is driven 3x
        //     }
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "REL".to_string(),
                    kind: SymbolKind::Rel_t {
                        input_types: vec![Type::Bool],
                        return_type: Type::Bool,
                    },
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "NET2".to_string(),
                    kind: SymbolKind::Net {
                        ports: HashMap::from([
                            ("B".to_string(), NetPort {
                                symbol: 2,
                                input: false,
                            }),
                        ]),
                    },
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "B".to_string(),
                    kind: SymbolKind::Ent(Type::Bool),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "NET".to_string(),
                    kind: SymbolKind::Net {
                        ports: HashMap::from([
                            ("a".to_string(), NetPort {
                                symbol: 4,
                                input: true,
                            }),
                            ("b".to_string(), NetPort {
                                symbol: 6,
                                input: true,
                            }),
                        ]),
                    },
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Ent(Type::Bool),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "b".to_string(),
                    kind: SymbolKind::Ent(Type::Bool),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("REL".to_string(), 0),
                        ("NET2".to_string(), 1),
                        ("B".to_string(), 2),
                        ("NET".to_string(), 3),
                        ("a".to_string(), 4),
                        ("b".to_string(), 5),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.check_constraints_net(Net {
            name: Ident::Symbol(3),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(4),
                        param_type: Type::Bool,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(5),
                        param_type: Type::Bool,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::RelInst(RelInst {
                    asignee: Ident::Symbol(5),
                    rel: Ident::Symbol(0),
                    args: vec![Ident::Symbol(4)],
                    span: Span{line: 0, col: 0},
                }),
                NetItem::NetInst(NetInst {
                    net: Ident::Symbol(1),
                    connections: vec![
                        Connection {
                            port: Ident::Symbol(2),
                            ent: Ident::Symbol(5),
                            span: Span{line: 0, col: 0},
                        }
                    ],
                }),
            ],
        });

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 2);
    }
}