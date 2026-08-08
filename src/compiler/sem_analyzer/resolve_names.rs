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

//! # resolve_names
//!
//! Handles name resolution and building the symbol table of semantic analysis
//!
//! ## Invariants
//!
//! - Must use the same ast as in parsing, just change Idents from Ident::Str to Ident::Symbol
//! - Resolve names sets all types in new symbols as unknown. Type checking handles setting those types
//!
//! Author: Cole Francis

use std::collections::HashMap;

use super::SemAnalyzer;
use super::symbol::{Symbol, SymbolKind, SymbolId, NetPort};
use super::scope::Scope;
use super::types::Type;
use crate::compiler::parser::ast::*;
use crate::compiler::diagnostics::{CompilerError, Span, Expected};

impl <'a> SemAnalyzer<'a> {
    pub(super) fn resolve_names(&mut self) {
        let items = std::mem::take(&mut self.ast.items);

        for item in items {
            let resolved_item = self.resolve_item(item).unwrap_or(Item::Error);
            self.ast.items.push(resolved_item);
        }
    }

    fn resolve_item(&mut self, item: Item) -> Option<Item> {
        match item {
            Item::Let(stmt)     => self.resolve_let(stmt).map(Item::Let),
            Item::Ent(ent_type) => self.resolve_ent(ent_type).map(Item::Ent),
            Item::Rel(rel_type) => self.resolve_rel(rel_type).map(Item::Rel),
            Item::Net(net)      => self.resolve_net(net).map(Item::Net),
            Item::Error         => Some(Item::Error),
        }
    }

    pub(super) fn resolve_let(&mut self, mut stmt: LetStatement) -> Option<LetStatement> {
        let (name, span) = self.extract_ident_str(stmt.name)?; // Should not return None
        
        stmt.name = Ident::Symbol(self.define_symbol(
            name, 
            SymbolKind::Variable(Type::Unknown), 
            span,
        )?);

        stmt.expr = self.resolve_expr(stmt.expr).unwrap_or(Expr::Error);

        Some(stmt)
    }

    fn resolve_ent(&mut self, mut ent_t: EntType) -> Option<EntType> {
        let (name, span) = self.extract_ident_str(ent_t.name)?;
        let ent_t_symbol_id = self.define_symbol(
            name,
            SymbolKind::EntType,
            span,
        )?;

        ent_t.expr = match ent_t.expr {
            EntExpr::Mod(val) => EntExpr::Mod(val),
            EntExpr::SetEnt(idents) => {
                let mut resolved_idents: Vec<Ident> = Vec::new();

                for ident in idents {
                    let (name, span) = self.extract_ident_str(ident)?;
                    let member_symbol_id = self.define_symbol(
                        name,
                        SymbolKind::EntMember{parent: ent_t_symbol_id},
                        span,
                    )?;

                    resolved_idents.push(Ident::Symbol(member_symbol_id));
                }

                EntExpr::SetEnt(resolved_idents)
            }
        };

        ent_t.name = Ident::Symbol(ent_t_symbol_id);
        
        Some(ent_t)
    }

    fn resolve_rel(&mut self, rel_t: RelType) -> Option<RelType> {
        let (name, span) = self.extract_ident_str(rel_t.name)?;
        let rel_symbol_id = self.define_symbol(
            name,
            SymbolKind::Rel_t {
                input_types: Vec::new(),
                return_type: Type::Unknown,
            },
            span,
        )?;

        self.create_scope();

        let mut resolved_params: Vec<Param> = Vec::new();

        for param in rel_t.params {
            let (name, span) = self.extract_ident_str(param.name)?;
            let param_symbol_id = self.define_symbol(
                name,
                SymbolKind::Variable(Type::Unknown),
                span,
            )?;

            let resolved_param_type = self.resolve_type(param.param_type)
                .unwrap_or(Type::Error);

            resolved_params.push(Param {
                name: Ident::Symbol(param_symbol_id),
                param_type: resolved_param_type,
            });
        }

        let resolved_return_type = self.resolve_type(rel_t.return_type)
            .unwrap_or(Type::Error);

        let resolved_body = self.resolve_expr(rel_t.body)
            .unwrap_or(Expr::Error);

        self.exit_scope();

        Some(RelType {
            name: Ident::Symbol(rel_symbol_id),
            params: resolved_params,
            return_type: resolved_return_type,
            body: resolved_body,
        })
    }

    fn resolve_net(&mut self, net: Net) -> Option<Net> {
        let (name, span) = self.extract_ident_str(net.name)?;
        let symbol_id = self.define_symbol(
            name,
            SymbolKind::Net {ports: HashMap::new()},
            span,
        )?;

        self.create_scope();

        let mut resolved_items: Vec<NetItem> = Vec::new();

        for item in net.items {
            resolved_items.push(self.resolve_net_item(item, symbol_id)
                .unwrap_or(NetItem::Error));
        }

        self.exit_scope();

        Some(Net {
            name: Ident::Symbol(symbol_id),
            items: resolved_items,
        })
    }

    fn resolve_net_item(&mut self, item: NetItem, net_id: SymbolId) -> Option<NetItem> {
        match item {
            NetItem::Input(param) => {
                let (name, span) = self.extract_ident_str(param.name)?;

                let symbol_id = self.find_or_define_symbol(
                    &name, 
                    SymbolKind::Ent(Type::Unknown), 
                    span,
                );

                // Ports need to be accessable outside for instantiaing in other nets
                if let SymbolKind::Net { ports } = &mut self.symbols[net_id].kind {
                    ports.insert(name, NetPort {
                        symbol: symbol_id,
                        ty: Type::Unknown,
                    });
                }

                let resolved_param_type = self.resolve_type(param.param_type)
                    .unwrap_or(Type::Error);

                Some(NetItem::Input(Param {
                    name: Ident::Symbol(symbol_id),
                    param_type: resolved_param_type,
                }))
            }

            NetItem::Output(param) => {
                let (name, span) = self.extract_ident_str(param.name)?;
                let symbol_id = self.find_or_define_symbol(
                    &name, 
                    SymbolKind::Ent(Type::Unknown), 
                    span,
                );

                // Ports need to be accessable outside for instantiaing in other nets
                if let SymbolKind::Net { ports } = &mut self.symbols[net_id].kind {
                    ports.insert(name, NetPort {
                        symbol: symbol_id,
                        ty: Type::Unknown,
                    });
                }

                let resolved_param_type = self.resolve_type(param.param_type)
                    .unwrap_or(Type::Error);

                Some(NetItem::Output(Param {
                    name: Ident::Symbol(symbol_id),
                    param_type: resolved_param_type,
                }))
            }

            NetItem::Init(ent_init) => {
                let (name, span) = self.extract_ident_str(ent_init.param.name)?;
                let symbol_id = self.find_or_define_symbol(
                    &name, 
                    SymbolKind::Ent(Type::Unknown), 
                    span,
                );

                let resolved_param_type = self.resolve_type(ent_init.param.param_type)
                    .unwrap_or(Type::Error);

                let resolved_val = self.resolve_expr(ent_init.val)
                    .unwrap_or(Expr::Error);

                Some(NetItem::Init(EntInit {
                    param: Param {
                        name: Ident::Symbol(symbol_id),
                        param_type: resolved_param_type,
                    },
                    val: resolved_val,
                }))
            }

            NetItem::RelInst(rel_inst) => {
                let (name, span) = self.extract_ident_str(rel_inst.asignee)?;
                let asignee_id = self.find_or_define_symbol(
                    &name,
                    SymbolKind::Ent(Type::Unknown),
                    span,
                );

                let (name, span) = self.extract_ident_str(rel_inst.rel)?;
                let rel_id = self.find_symbol(&name, span)?;

                let mut resolved_args: Vec<Ident> = Vec::new();

                for arg in rel_inst.args {
                    let (name, span) = self.extract_ident_str(arg)?;
                    let symbol_id = self.find_or_define_symbol(
                        &name,
                        SymbolKind::Ent(Type::Unknown),
                        span,
                    );

                    resolved_args.push(Ident::Symbol(symbol_id));
                }

                Some(NetItem::RelInst(RelInst {
                    asignee: Ident::Symbol(asignee_id),
                    rel: Ident::Symbol(rel_id),
                    args: resolved_args,
                }))
            }

            NetItem::NetInst(net_inst) => {
                let (name, span) = self.extract_ident_str(net_inst.net)?;
                let inst_net_id = self.find_symbol(&name, span)?;

                let mut resolved_connections: Vec<Connection> = Vec::new();

                for connection in net_inst.connections {
                    // Port symbols have to be checked specialy
                    let (name, span) = self.extract_ident_str(connection.port)?;
                    let port_id = self.find_net_port(inst_net_id, &name, span)?;

                    let (name, span) = self.extract_ident_str(connection.net)?;
                    let connection_net_id = self.find_or_define_symbol(
                        &name, 
                        SymbolKind::Ent(Type::Unknown), 
                        span,
                    );

                    resolved_connections.push(Connection {
                        port: Ident::Symbol(port_id),
                        net: Ident::Symbol(connection_net_id),
                    })
                }

                Some(NetItem::NetInst(NetInst {
                    net: Ident::Symbol(inst_net_id),
                    connections: resolved_connections,
                }))
            }

            NetItem::Error => {
                Some(NetItem::Error)
            }
        }
    }

    fn find_net_port(&mut self, net_id: SymbolId, name: &str, span: Span) -> Option<SymbolId> {
        match &self.symbols[net_id].kind {
            SymbolKind::Net { ports } =>  match ports.get(name) {
                Some(port) => Some(port.symbol),
                None => {
                    self.diagnostics.error(CompilerError::UndefinedPort {
                        name: name.to_string(),
                        span,
                    });

                    None
                }
            },
            other => {
                self.diagnostics.error(CompilerError::UnexpectedIdent {
                    expected: vec![SymbolKind::Net { ports: HashMap::new() }],
                    found: other.clone(),
                    span,
                });

                None
            },
        }
    }

    fn resolve_type(&mut self, ty: Type) -> Option<Type> {
        match ty {
            Type::Bool    => Some(Type::Bool),
            Type::Impulse => Some(Type::Impulse),
            Type::Int     => Some(Type::Int),
            Type::Real    => Some(Type::Real),

            Type::Mod(val) => {
                Some(Type::Mod(val))
            }

            Type::Custom(ident) => {
                let (name, span) = self.extract_ident_str(ident)?;

                let symbol_id = self.find_symbol(&name, span)?;

                Some(Type::Custom(Ident::Symbol(symbol_id)))
            }

            Type::Error => Some(Type::Error),
            Type::Unknown => Some(Type::Unknown), // Should not reach
        }
    }

    pub(super) fn extract_ident_str(&self, ident: Ident) -> Option<(String, Span)> {
        match ident {
            Ident::Str {val, span} => Some((val, span)),
            Ident::Symbol(_) => None,
        }
    }

    pub(super) fn find_symbol(&mut self, name: &str, span: Span) -> Option<SymbolId> {
        for scope in self.scopes.iter().rev() {
            if let Some(id) = scope.symbols.get(name) {
                return Some(*id);
            }
        }
        self.diagnostics.error(CompilerError::UndefinedIdent {
            name: name.to_string(),
            span,
        });

        None
    }

    fn define_symbol(&mut self, name: String, kind: SymbolKind, span: Span) -> Option<SymbolId> {
        let current = self.scopes.last_mut().unwrap();

        // Duplicate definition
        if let Some(&old_id) = current.symbols.get(&name) {
            let old_span = self.symbols[old_id].span;

            self.diagnostics.error(CompilerError::DuplicateDefinition {
                name,
                old_span,
                new_span: span,
            });

            return None;
        }

        let id = self.symbols.len();

        self.symbols.push(Symbol {
            id,
            name: name.clone(),
            kind,
            span,
        });

        current.symbols.insert(name, id);

        Some(id)
    }

    fn find_or_define_symbol(&mut self, name: &str, kind: SymbolKind, span: Span) -> SymbolId {
        let current = self.scopes.last_mut().unwrap();

        if let Some(id) = current.symbols.get(name) {
            return *id;
        }

        let id = self.symbols.len();

        self.symbols.push(Symbol {
            id,
            name: name.to_string(),
            kind,
            span,
        });

        current.symbols.insert(name.to_string(), id);

        id
    }

    pub(super) fn create_scope(&mut self) {
        self.scopes.push(Scope {
            symbols: HashMap::new(),
        });
    }

    pub(super) fn exit_scope(&mut self) {
        assert!(self.scopes.len() > 1);
        self.scopes.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::diagnostics::Diagnostics;
    use crate::compiler::sem_analyzer::scope::Scope;
    use crate::compiler::lexer::Lexer;
    use crate::compiler::parser::Parser;

    #[test]
    fn test_find() {
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    id: 0,
                    name: "a".to_string(),
                    kind: SymbolKind::Variable(Type::Unknown),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    id: 1,
                    name: "b".to_string(),
                    kind: SymbolKind::Variable(Type::Unknown),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    id: 2,
                    name: "c".to_string(),
                    kind: SymbolKind::Variable(Type::Unknown),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("a".to_string(), 0),
                    ])
                },
                Scope {
                    symbols: HashMap::from([
                        ("b".to_string(), 1),
                    ])
                },
            ],

            diagnostics: &mut Diagnostics::new(),
        };

        assert_eq!(sem_analyzer.find_symbol("a", Span{line: 1,col: 0}), Some(0));
        assert_eq!(sem_analyzer.find_symbol("b", Span{line: 2,col: 0}), Some(1));
        assert_eq!(sem_analyzer.find_symbol("c", Span{line: 3,col: 0}), None);
        assert_eq!(sem_analyzer.diagnostics.num_errors(), 1);
    }

    #[test]
    fn test_define_1() {
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    id: 0,
                    name: "a".to_string(),
                    kind: SymbolKind::Variable(Type::Unknown),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    id: 1,
                    name: "b".to_string(),
                    kind: SymbolKind::Variable(Type::Unknown),
                    span: Span{line: 0, col: 0},
                }
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("a".to_string(), 0),
                    ])
                },
                Scope {
                    symbols: HashMap::from([
                        ("b".to_string(), 1),
                    ])
                }
            ],

            diagnostics: &mut Diagnostics::new(),
        };

        let result = sem_analyzer.define_symbol("a".to_string(), SymbolKind::Variable(Type::Unknown), Span{line:0,col:0});

        assert_eq!(result, Some(2));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                id: 0,
                name: "a".to_string(),
                kind: SymbolKind::Variable(Type::Unknown),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                id: 1,
                name: "b".to_string(),
                kind: SymbolKind::Variable(Type::Unknown),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                id: 2,
                name: "a".to_string(),
                kind: SymbolKind::Variable(Type::Unknown),
                span: Span{line: 0, col: 0},
            }
        ]);
        assert_eq!(sem_analyzer.scopes, vec![
            Scope {
                symbols: HashMap::from([
                    ("a".to_string(), 0),
                ])
            },
            Scope {
                symbols: HashMap::from([
                    ("b".to_string(), 1),
                    ("a".to_string(), 2),
                ])
            }
        ]);
    }
    
    #[test]
    fn test_define_2() {
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    id: 0,
                    name: "a".to_string(),
                    kind: SymbolKind::Variable(Type::Unknown),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    id: 1,
                    name: "b".to_string(),
                    kind: SymbolKind::Variable(Type::Unknown),
                    span: Span{line: 0, col: 0},
                }
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("a".to_string(), 0),
                    ])
                },
            ],

            diagnostics: &mut Diagnostics::new(),
        };

        let result = sem_analyzer.define_symbol("a".to_string(), SymbolKind::Variable(Type::Unknown), Span{line:0,col:0});

        assert_eq!(result, None);
        assert_eq!(sem_analyzer.diagnostics.num_errors(), 1);
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                id: 0,
                name: "a".to_string(),
                kind: SymbolKind::Variable(Type::Unknown),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                id: 1,
                name: "b".to_string(),
                kind: SymbolKind::Variable(Type::Unknown),
                span: Span{line: 0, col: 0},
            },
        ]);
        assert_eq!(sem_analyzer.scopes, vec![
            Scope {
                symbols: HashMap::from([
                    ("a".to_string(), 0),
                ])
            },
        ]);
    }

    #[test]
    fn enter_exit_scope() {
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer::new(
            Program {items: Vec::new()},
            &mut diagnostics,
        );

        sem_analyzer.create_scope();

        assert_eq!(sem_analyzer.scopes, vec![
            Scope {
                symbols: HashMap::from([
                ])
            },
            Scope {
                symbols: HashMap::from([
                ])
            }
        ]);

        sem_analyzer.exit_scope();

        assert_eq!(sem_analyzer.scopes, vec![
            Scope {
                symbols: HashMap::from([
                ])
            },
        ]);
    }

    #[test]
    fn resolve_let() {
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer::new(
            Program{items: vec![]},
            &mut diagnostics,
        );

        let result = sem_analyzer.resolve_let(LetStatement {
            name: Ident::Str {
                val: "a".to_string(),
                span: Span{line: 1, col: 2},
            },
            expr: Expr::Literal(Literal::Bool(false)),
        });

        assert_eq!(result, Some(LetStatement {
            name: Ident::Symbol(0),
            expr: Expr::Literal(Literal::Bool(false)),
        }));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                id: 0,
                name: "a".to_string(),
                kind: SymbolKind::Variable(Type::Unknown),
                span: Span{line: 1, col: 2},
            },
        ]);
    }

    #[test]
    fn resolve_ent_1() {
        // ent_t z3 = Mod(3);
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer::new(
            Program{items: vec![]},
            &mut diagnostics,
        );

        let result = sem_analyzer.resolve_ent(EntType {
            name: Ident::Str {
                val: "z3".to_string(),
                span: Span{line: 0, col: 0}
            },
            expr: EntExpr::Mod(3),
        });

        assert_eq!(result, Some(EntType {
            name: Ident::Symbol(0),
            expr: EntExpr::Mod(3),
        }));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                id: 0,
                name: "z3".to_string(),
                kind: SymbolKind::EntType,
                span: Span{line: 0, col: 0},
            },
        ]);
    }

    #[test]
    fn resolve_ent_2() {
        // ent_t COIN = {H, T};
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer::new(
            Program{items: vec![]},
            &mut diagnostics,
        );

        let result = sem_analyzer.resolve_ent(EntType {
            name: Ident::Str {
                val: "COIN".to_string(),
                span: Span{line: 0, col: 0}
            },
            expr: EntExpr::SetEnt(vec![
                Ident::Str {
                    val: "H".to_string(),
                    span: Span{line: 0, col: 1}
                },
                Ident::Str {
                    val: "T".to_string(),
                    span: Span{line: 0, col: 2}
                },
            ]),
        });

        assert_eq!(result, Some(EntType {
            name: Ident::Symbol(0),
            expr: EntExpr::SetEnt(vec![
                Ident::Symbol(1),
                Ident::Symbol(2),
            ]),
        }));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                id: 0,
                name: "COIN".to_string(),
                kind: SymbolKind::EntType,
                span: Span{line: 0, col: 0},
            },
            Symbol {
                id: 1,
                name: "H".to_string(),
                kind: SymbolKind::EntMember{ parent: 0 },
                span: Span{line: 0, col: 1},
            },
            Symbol {
                id: 2,
                name: "T".to_string(),
                kind: SymbolKind::EntMember{ parent: 0 },
                span: Span{line: 0, col: 2},
            },
        ]);
    }

    #[test]
    fn rel_1() {
        // rel_t is_heads (c: COIN) -> Bool = match c {
        //     H => true,
        //     _ => false,
        // };
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    id: 0,
                    name: "COIN".to_string(),
                    kind: SymbolKind::EntType,
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    id: 1,
                    name: "H".to_string(),
                    kind: SymbolKind::EntMember{ parent: 0 },
                    span: Span{line: 0, col: 1},
                },
                Symbol {
                    id: 2,
                    name: "T".to_string(),
                    kind: SymbolKind::EntMember{ parent: 0 },
                    span: Span{line: 0, col: 2},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("COIN".to_string(), 0),
                        ("H".to_string(), 1),
                        ("T".to_string(), 2),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.resolve_rel(RelType {
            name: Ident::Str {
                val: "is_heads".to_string(),
                span: Span{line: 0, col: 3}
            },
            params: vec![
                Param {
                    name: Ident::Str {
                        val: "c".to_string(),
                        span: Span{line: 0, col: 4}
                    },
                    param_type: Type::Custom(Ident::Str {
                        val: "COIN".to_string(),
                        span: Span{line: 0, col: 2}
                    })
                }
            ],
            return_type: Type::Bool,
            body: Expr::Match(MatchExpr {
                scrutinee: Box::new(Expr::Ident(Ident::Str {
                    val: "c".to_string(),
                    span: Span{line: 1, col: 4}
                })),
                arms: vec![
                    MatchArm {
                        pattern: vec![SimplePattern::Ident(Ident::Str {
                            val: "H".to_string(),
                            span: Span{line: 1, col: 0},
                        })],
                        expr: Expr::Literal(Literal::Bool(true)),
                    },
                    MatchArm {
                        pattern: vec![SimplePattern::Default],
                        expr: Expr::Literal(Literal::Bool(false)),
                    }
                ],
            }),
        });

        assert_eq!(result, Some(RelType {
            name: Ident::Symbol(3),
            params: vec![
                Param {
                    name: Ident::Symbol(4),
                    param_type: Type::Custom(Ident::Symbol(0)),
                }
            ],
            return_type: Type::Bool,
            body: Expr::Match(MatchExpr {
                scrutinee: Box::new(Expr::Ident(Ident::Symbol(4))),
                arms: vec![
                    MatchArm {
                        pattern: vec![SimplePattern::Ident(Ident::Symbol(1))],
                        expr: Expr::Literal(Literal::Bool(true)),
                    },
                    MatchArm {
                        pattern: vec![SimplePattern::Default],
                        expr: Expr::Literal(Literal::Bool(false)),
                    }
                ],
            }),
        }));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                id: 0,
                name: "COIN".to_string(),
                kind: SymbolKind::EntType,
                span: Span{line: 0, col: 0},
            },
            Symbol {
                id: 1,
                name: "H".to_string(),
                kind: SymbolKind::EntMember{ parent: 0 },
                span: Span{line: 0, col: 1},
            },
            Symbol {
                id: 2,
                name: "T".to_string(),
                kind: SymbolKind::EntMember{ parent: 0 },
                span: Span{line: 0, col: 2},
            },
            Symbol {
                id: 3,
                name: "is_heads".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![],
                    return_type: Type::Unknown,
                },
                span: Span{line: 0, col: 3},
            },
            Symbol {
                id: 4,
                name: "c".to_string(),
                kind: SymbolKind::Variable(Type::Unknown),
                span: Span{line: 0, col: 4},
            },
        ]);
        assert_eq!(sem_analyzer.scopes, vec![
            Scope {
                symbols: HashMap::from([
                    ("COIN".to_string(), 0),
                    ("H".to_string(), 1),
                    ("T".to_string(), 2),
                    ("is_heads".to_string(), 3),
                ])
            },
        ]);
    }

    #[test]
    fn net_1() {
        // net TEST {
        //     input a: Bool;
        //     output b: Real;
        //     init c: Int = 3;

        //     d = REL(a, c);

        //     NET {
        //         A := d,
        //         B := b,
        //     };
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    id: 0,
                    name: "REL".to_string(),
                    kind: SymbolKind::Rel_t {
                        input_types: vec![],
                        return_type: Type::Unknown,
                    },
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    id: 1,
                    name: "NET".to_string(),
                    kind: SymbolKind::Net {
                        ports: HashMap::from([
                            ("A".to_string(), NetPort {
                                symbol: 2,
                                ty: Type::Unknown,
                            }),
                            ("B".to_string(), NetPort {
                                symbol: 3,
                                ty: Type::Unknown,
                            }),
                        ])
                    },
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    id: 2,
                    name: "A".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    id: 3,
                    name: "B".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("REL".to_string(), 0),
                        ("NET".to_string(), 1),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.resolve_net(Net {
            name: Ident::Str {
                val: "TEST".to_string(),
                span: Span {line: 0, col: 0},
            },
            items: vec![
                NetItem::Input(Param {
                    name: Ident::Str {
                        val: "a".to_string(),
                        span: Span {line: 0, col: 0},
                    },
                    param_type: Type::Bool,
                }),
                NetItem::Output(Param {
                    name: Ident::Str {
                        val: "b".to_string(),
                        span: Span {line: 0, col: 0},
                    },
                    param_type: Type::Real,
                }),
                NetItem::Init(EntInit {
                    param: Param {
                        name: Ident::Str {
                            val: "c".to_string(),
                            span: Span {line: 0, col: 0},
                        },
                        param_type: Type::Int,
                    },
                    val: Expr::Literal(Literal::Int(3)),
                }),
                NetItem::RelInst(RelInst {
                    asignee: Ident::Str {
                        val: "d".to_string(),
                        span: Span {line: 0, col: 0},
                    },
                    rel: Ident::Str {
                        val: "REL".to_string(),
                        span: Span {line: 0, col: 0},
                    },
                    args: vec![
                        Ident::Str {
                            val: "a".to_string(),
                            span: Span {line: 0, col: 0},
                        },
                        Ident::Str {
                            val: "c".to_string(),
                            span: Span {line: 0, col: 0},
                        },
                    ],
                }),
                NetItem::NetInst(NetInst {
                    net: Ident::Str {
                        val: "NET".to_string(),
                        span: Span {line: 0, col: 0},
                    },
                    connections: vec![
                        Connection {
                            port: Ident::Str {
                                val: "A".to_string(),
                                span: Span {line: 0, col: 0},
                            },
                            net: Ident::Str {
                                val: "d".to_string(),
                                span: Span {line: 0, col: 0},
                            },
                        },
                        Connection {
                            port: Ident::Str {
                                val: "B".to_string(),
                                span: Span {line: 0, col: 0},
                            },
                            net: Ident::Str {
                                val: "b".to_string(),
                                span: Span {line: 0, col: 0},
                            },
                        },
                    ],
                }),
            ],
        });

        assert_eq!(result, Some(Net {
            name: Ident::Symbol(4),
            items: vec![
                NetItem::Input(Param {
                    name: Ident::Symbol(5),
                    param_type: Type::Bool,
                }),
                NetItem::Output(Param {
                    name: Ident::Symbol(6),
                    param_type: Type::Real,
                }),
                NetItem::Init(EntInit {
                    param: Param {
                        name: Ident::Symbol(7),
                        param_type: Type::Int,
                    },
                    val: Expr::Literal(Literal::Int(3)),
                }),
                NetItem::RelInst(RelInst {
                    asignee: Ident::Symbol(8),
                    rel: Ident::Symbol(0),
                    args: vec![
                        Ident::Symbol(5),
                        Ident::Symbol(7),
                    ],
                }),
                NetItem::NetInst(NetInst {
                    net: Ident::Symbol(1),
                    connections: vec![
                        Connection {
                            port: Ident::Symbol(2),
                            net: Ident::Symbol(8),
                        },
                        Connection {
                            port: Ident::Symbol(3),
                            net: Ident::Symbol(6),
                        },
                    ],
                }),
            ],
        }));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                id: 0,
                name: "REL".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![],
                    return_type: Type::Unknown,
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                id: 1,
                name: "NET".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([
                        ("A".to_string(), NetPort {
                            symbol: 2,
                            ty: Type::Unknown,
                        }),
                        ("B".to_string(), NetPort {
                            symbol: 3,
                            ty: Type::Unknown,
                        }),
                    ])
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                id: 2,
                name: "A".to_string(),
                kind: SymbolKind::Ent(Type::Unknown),
                span: Span {line: 0, col: 0},
            },
            Symbol {
                id: 3,
                name: "B".to_string(),
                kind: SymbolKind::Ent(Type::Unknown),
                span: Span {line: 0, col: 0},
            },
            Symbol {
                id: 4,
                name: "TEST".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([
                        ("a".to_string(), NetPort {
                            symbol: 5,
                            ty: Type::Unknown,
                        }),
                        ("b".to_string(), NetPort {
                            symbol: 6,
                            ty: Type::Unknown,
                        }),
                    ])
                },
                span: Span {line: 0, col: 0},
            },
            Symbol {
                id: 5,
                name: "a".to_string(),
                kind: SymbolKind::Ent(Type::Unknown),
                span: Span {line: 0, col: 0}
            },
            Symbol {
                id: 6,
                name: "b".to_string(),
                kind: SymbolKind::Ent(Type::Unknown),
                span: Span {line: 0, col: 0}
            },
            Symbol {
                id: 7,
                name: "c".to_string(),
                kind: SymbolKind::Ent(Type::Unknown),
                span: Span {line: 0, col: 0}
            },
            Symbol {
                id: 8,
                name: "d".to_string(),
                kind: SymbolKind::Ent(Type::Unknown),
                span: Span {line: 0, col: 0}
            },
        ]);
        assert_eq!(sem_analyzer.scopes, vec![
            Scope {
                symbols: HashMap::from([
                    ("REL".to_string(), 0),
                    ("NET".to_string(), 1),
                    ("TEST".to_string(), 4),
                ])
            },
        ]);
    }

    #[test]
    fn bad_net_1() {
        // net TEST {
        //     input a: Bool;
        //     output b: Real;
        //     init c Int = 3; // Error net member (already reported)

        //     d = REL(a, c); // REL is not defined

        //     NET {
        //         A := d,
        //         B := b, // Net has no port B
        //     };
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    id: 0,
                    name: "NET".to_string(),
                    kind: SymbolKind::Net {
                        ports: HashMap::from([
                            ("A".to_string(), NetPort {
                                symbol: 1,
                                ty: Type::Unknown,
                            }),
                        ])
                    },
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    id: 1,
                    name: "A".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("NET".to_string(), 0),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.resolve_net(Net {
            name: Ident::Str {
                val: "TEST".to_string(),
                span: Span {line: 0, col: 0},
            },
            items: vec![
                NetItem::Input(Param {
                    name: Ident::Str {
                        val: "a".to_string(),
                        span: Span {line: 0, col: 0},
                    },
                    param_type: Type::Bool,
                }),
                NetItem::Output(Param {
                    name: Ident::Str {
                        val: "b".to_string(),
                        span: Span {line: 0, col: 0},
                    },
                    param_type: Type::Real,
                }),
                NetItem::Error,
                NetItem::RelInst(RelInst {
                    asignee: Ident::Str {
                        val: "d".to_string(),
                        span: Span {line: 0, col: 0},
                    },
                    rel: Ident::Str {
                        val: "REL".to_string(),
                        span: Span {line: 0, col: 0},
                    },
                    args: vec![
                        Ident::Str {
                            val: "a".to_string(),
                            span: Span {line: 0, col: 0},
                        },
                        Ident::Str {
                            val: "c".to_string(),
                            span: Span {line: 0, col: 0},
                        },
                    ],
                }),
                NetItem::NetInst(NetInst {
                    net: Ident::Str {
                        val: "NET".to_string(),
                        span: Span {line: 0, col: 0},
                    },
                    connections: vec![
                        Connection {
                            port: Ident::Str {
                                val: "A".to_string(),
                                span: Span {line: 0, col: 0},
                            },
                            net: Ident::Str {
                                val: "d".to_string(),
                                span: Span {line: 0, col: 0},
                            },
                        },
                        Connection {
                            port: Ident::Str {
                                val: "B".to_string(),
                                span: Span {line: 0, col: 0},
                            },
                            net: Ident::Str {
                                val: "b".to_string(),
                                span: Span {line: 0, col: 0},
                            },
                        },
                    ],
                }),
            ],
        });

        assert_eq!(result, Some(Net {
            name: Ident::Symbol(2),
            items: vec![
                NetItem::Input(Param {
                    name: Ident::Symbol(3),
                    param_type: Type::Bool,
                }),
                NetItem::Output(Param {
                    name: Ident::Symbol(4),
                    param_type: Type::Real,
                }),
                NetItem::Error,
                NetItem::Error,
                NetItem::Error,
            ],
        }));

        diagnostics.debug_print();
        assert_eq!(diagnostics.num_errors(), 2);
    }

    #[test]
    fn program() {
        let mut diagnostics = Diagnostics::new();

        let tokens = Lexer::new("
let n = {
    let a = 1;
    a+1
};

ent_t SINGLE = {A};

rel_t ADD : (b: Int) -> Int = n + b;

net FIRST {
    input a: Int;
    output q: Int;

    q := ADD(a);
}

net SECOND {
    input a: SINGLE;
    output c: SINGLE;

    FIRST {
        a := a,
        q := c,
    };
}
        ", &mut diagnostics).tokenize();

        let program = Parser::new(tokens, &mut diagnostics).parse();

        let mut sem_analyzer = SemAnalyzer::new(program, &mut diagnostics);

        sem_analyzer.resolve_names();

        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                id: 0,
                name: "n".to_string(),
                kind: SymbolKind::Variable(Type::Unknown),
                span: Span {line: 2, col: 5},
            },
            Symbol {
                id: 1,
                name: "a".to_string(),
                kind: SymbolKind::Variable(Type::Unknown),
                span: Span {line: 3, col: 9},
            },
            Symbol {
                id: 2,
                name: "SINGLE".to_string(),
                kind: SymbolKind::EntType,
                span: Span {line: 7, col: 7},
            },
            Symbol {
                id: 3,
                name: "A".to_string(),
                kind: SymbolKind::EntMember{ parent: 2 },
                span: Span {line: 7, col: 17},
            },
            Symbol {
                id: 4,
                name: "ADD".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![],
                    return_type: Type::Unknown,
                },
                span: Span {line: 9, col: 7},
            },
            Symbol {
                id: 5,
                name: "b".to_string(),
                kind: SymbolKind::Variable(Type::Unknown),
                span: Span {line: 9, col: 14},
            },
            Symbol {
                id: 6,
                name: "FIRST".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([
                        ("a".to_string(), NetPort {
                            symbol: 7,
                            ty: Type::Unknown,
                        }),
                        ("q".to_string(), NetPort {
                            symbol: 8,
                            ty: Type::Unknown,
                        }),
                    ])
                },
                span: Span {line: 11, col: 5},
            },
            Symbol {
                id: 7,
                name: "a".to_string(),
                kind: SymbolKind::Ent(Type::Unknown),
                span: Span {line: 12, col: 11},
            },
            Symbol {
                id: 8,
                name: "q".to_string(),
                kind: SymbolKind::Ent(Type::Unknown),
                span: Span {line: 13, col: 12},
            },
            Symbol {
                id: 9,
                name: "SECOND".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([
                        ("a".to_string(), NetPort {
                            symbol: 10,
                            ty: Type::Unknown,
                        }),
                        ("c".to_string(), NetPort {
                            symbol: 11,
                            ty: Type::Unknown,
                        }),
                    ])
                },
                span: Span {line: 18, col: 5},
            },
            Symbol {
                id: 10,
                name: "a".to_string(),
                kind: SymbolKind::Ent(Type::Unknown),
                span: Span {line: 19, col: 11},
            },
            Symbol {
                id: 11,
                name: "c".to_string(),
                kind: SymbolKind::Ent(Type::Unknown),
                span: Span {line: 20, col: 12},
            },
        ]);
        assert_eq!(sem_analyzer.ast, Program {items: vec![
            Item::Let(LetStatement {
                name: Ident::Symbol(0),
                expr: Expr::Block(BlockExpr {
                    statements: vec![
                        Statement::Let(LetStatement {
                            name: Ident::Symbol(1),
                            expr: Expr::Literal(Literal::Int(1)),
                        })
                    ],
                    expr: Box::new(Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::Ident(Ident::Symbol(1))),
                        op: BinaryOp::Add,
                        right: Box::new(Expr::Literal(Literal::Int(1))),
                    })),
                }),
            }),
            Item::Ent(EntType {
                name: Ident::Symbol(2),
                expr: EntExpr::SetEnt(vec![
                    Ident::Symbol(3),
                ]),
            }),
            Item::Rel(RelType {
                name: Ident::Symbol(4),
                params: vec![
                    Param {
                        name: Ident::Symbol(5),
                        param_type: Type::Int,
                    },
                ],
                return_type: Type::Int,
                body: Expr::Binary(BinaryExpr {
                    left: Box::new(Expr::Ident(Ident::Symbol(0))),
                    op: BinaryOp::Add,
                    right: Box::new(Expr::Ident(Ident::Symbol(5))),
                }),
            }),
            Item::Net(Net {
                name: Ident::Symbol(6),
                items: vec![
                    NetItem::Input(Param {
                        name: Ident::Symbol(7),
                        param_type: Type::Int,
                    }),
                    NetItem::Output(Param {
                        name: Ident::Symbol(8),
                        param_type: Type::Int,
                    }),
                    NetItem::RelInst(RelInst {
                        asignee: Ident::Symbol(8),
                        rel: Ident::Symbol(4),
                        args: vec![
                            Ident::Symbol(7),
                        ],
                    }),
                ],
            }),
            Item::Net(Net {
                name: Ident::Symbol(9),
                items: vec![
                    NetItem::Input(Param {
                        name: Ident::Symbol(10),
                        param_type: Type::Custom(Ident::Symbol(2)),
                    }),
                    NetItem::Output(Param {
                        name: Ident::Symbol(11),
                        param_type: Type::Custom(Ident::Symbol(2)),
                    }),
                    NetItem::NetInst(NetInst {
                        net: Ident::Symbol(6),
                        connections: vec![
                            Connection {
                                port: Ident::Symbol(7),
                                net: Ident::Symbol(10),
                            },
                            Connection {
                                port: Ident::Symbol(8),
                                net: Ident::Symbol(11),
                            },
                        ],
                    }),
                ],
            })
        ]});
    }
}