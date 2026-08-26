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

//! # check_types
//!
//! Handles checking and assigning of types for annotated ast
//!
//! ## Invariants
//!
//! - All incoming symbols will have Type::Unknown
//!
//! Author: Cole Francis

use super::SemAnalyzer;
use super::types::Type;
use crate::compiler::{
    symbol::{SymbolId, SymbolKind},
    ast::*,
    diagnostics::{CompilerError, Span},
};

impl <'a> SemAnalyzer<'a> {
    // Add type info to symbols
    // Verify all types match
    // Check number of relation arguments
    // Also check only one default in samples/Caseses
    pub(super) fn check_types(&mut self) {
        let items = std::mem::take(&mut self.ast.items);
        self.ast.items = Vec::with_capacity(items.len());

        for item in items {
            let resolved_item = self.check_item(item).unwrap_or(Item::Error);
            self.ast.items.push(resolved_item);
        }
    }

    fn check_item(&mut self, item: Item) -> Option<Item> {
        match item {
            Item::Let(stmt)     => Some(Item::Let(self.check_let(stmt))),
            Item::Ent(ent_type) => Some(Item::Ent(ent_type)),
            Item::Rel(rel_type) => self.check_rel(rel_type).map(Item::Rel),
            Item::Net(net)      => self.check_net(net).map(Item::Net),
            Item::Error         => Some(Item::Error),
        }
    }

    // There is no way checking types on LetStatement results in an error item.
    pub(super) fn check_let(&mut self, mut stmt: LetStatement) -> LetStatement {
        stmt.expr = self.add_types_expr(stmt.expr).unwrap_or(Expr::Error);

        let expr_type = self.get_expr_type(&stmt.expr);

        if let Ident::Symbol(symbol_id) = stmt.name {
            if let SymbolKind::Variable(var_type) = &mut self.symbols[symbol_id].kind {
                *var_type = expr_type;
            }
        }

        stmt
    }

    fn check_rel(&mut self, mut rel_t: RelType) -> Option<RelType> {
        // Annotate types of each parameter's symbol 
        // Annotate types of parameter and return in the relations name symbol
        // Check expression type matches return type

        for param in &mut rel_t.params {
            if let Ident::Symbol(param_id) = param.name {
                if let SymbolKind::Variable(param_symbol_type) = &mut self.symbols[param_id].kind {
                    *param_symbol_type = param.param_type.clone();
                }
            }


            if let Ident::Symbol(rel_name_id) = rel_t.name {
                if let SymbolKind::Rel_t {input_types, ..} = &mut self.symbols[rel_name_id].kind {
                    input_types.push(param.param_type.clone());
                }
            }
        }

        rel_t.body = self.add_types_expr(rel_t.body).unwrap_or(Expr::Error);

        let expr_type = self.get_expr_type(&rel_t.body);

        let span = match &rel_t.name {
            Ident::Str { span, ..} => span.clone(),  // Not reached but needed for compiling
            Ident::Symbol(id) => self.symbols[*id].span.clone(),
        };

        rel_t.return_type = self.verify_return_type(
            &rel_t.return_type, 
            &expr_type,
            &span
        ).unwrap_or(Type::Error);

        if let Ident::Symbol(rel_name_id) = rel_t.name {
            if let SymbolKind::Rel_t {return_type, ..} = &mut self.symbols[rel_name_id].kind {
                *return_type = rel_t.return_type.clone();
            }
        }

        Some(rel_t)
    }

    fn check_net(&mut self, mut net: Net) -> Option<Net> {
        // For each net inst, verify the type
        // Verify type for init
        // Verify type and number of parameters of rel inst
        // Verify types for connections in net inst

        for item in &mut net.items {
            let owned_item = std::mem::replace(item, NetItem::Error);

            *item = self.check_net_item(owned_item).unwrap_or(NetItem::Error);
        }

        Some(net)
    }

    fn check_net_item(&mut self, item: NetItem) -> Option<NetItem> {
        match item {
            NetItem::Input(input_ent) => {
                let Ident::Symbol(id) = input_ent.param.name else {
                    return None; // Not reachable
                };

                let (symbol_type, span) = self.get_ent_type(id)?;

                let new_type = self.compare_ent_types(symbol_type, input_ent.param.param_type.clone(), span)?;

                if let SymbolKind::Ent(ty) = &mut self.symbols[id].kind {
                    *ty = new_type;
                }

                Some(NetItem::Input(input_ent))
            }

            NetItem::Output(output_ent) => {
                let Ident::Symbol(id) = output_ent.param.name else {
                    return None; // Not reachable
                };

                let (symbol_type, span) = self.get_ent_type(id)?;

                let new_type = self.compare_ent_types(symbol_type, output_ent.param.param_type.clone(), span)?;

                if let SymbolKind::Ent(ty) = &mut self.symbols[id].kind {
                    *ty = new_type;
                }

                Some(NetItem::Output(output_ent))
            }

            NetItem::Init(ent_init) => {
                let Ident::Symbol(id) = ent_init.param.name else {
                    return None; // Not reachable
                };

                let (symbol_type, span) = self.get_ent_type(id)?;

                let new_type = self.compare_ent_types(symbol_type, ent_init.param.param_type.clone(), span)?;

                let expr_type = self.get_expr_type(&ent_init.val);

                let new_type = self.verify_return_type(&new_type, &expr_type, &span)?;

                if let SymbolKind::Ent(ty) = &mut self.symbols[id].kind {
                    *ty = new_type;
                }

                Some(NetItem::Init(ent_init))
            }

            NetItem::RelInst(rel_inst) => {
                let Ident::Symbol(rel_id) = rel_inst.rel else {
                    return None; // not reachable
                };

                let (input_types, return_type) = match &self.symbols[rel_id].kind {
                    SymbolKind::Rel_t {input_types, return_type} => (input_types.clone(), return_type.clone()),

                    other => {
                        self.diagnostics.error(CompilerError::UnexpectedIdent {
                            expected: vec![SymbolKind::Rel_t{input_types: Vec::new(), return_type: Type::Unknown}],
                            found: other.clone(),
                            span: self.symbols[rel_id].span.clone(),
                        });

                        return None
                    }
                };

                if rel_inst.args.len() != input_types.len() {
                    self.diagnostics.error(CompilerError::IncorrectNumberOfArgs {
                        expected_len: input_types.len(),
                        actual_len: rel_inst.args.len(),
                        rel_span: self.symbols[rel_id].span.clone(),
                    });

                    return None;
                }

                for (arg, expected_type) in rel_inst.args.iter().zip(input_types) {
                    let Ident::Symbol(arg_id) = arg else {
                        return None; // Not reachable
                    };
                    let (actual_type, span) = self.get_ent_type(*arg_id)?;

                    self.compare_ent_types(actual_type, expected_type, span)?;
                }

                let Ident::Symbol(asignee_id) = rel_inst.asignee else {
                    return None; // not reachable
                };
                let (asignee_type, span) = self.get_ent_type(asignee_id)?;

                self.compare_ent_types(asignee_type, return_type, span)?;

                Some(NetItem::RelInst(rel_inst))
            }

            NetItem::NetInst(net_inst) => {
                // We already verified symbolkind was net inst in resolving names
                let Ident::Symbol(net_id) = net_inst.net else {
                    return None; // not reachable
                };

                let ports = match &self.symbols[net_id].kind {
                    SymbolKind::Net {ports} => ports,
                    _ => return None, // Do not need to check this condition as find_net_port in resolve_net already does.
                };

                for connection in &net_inst.connections {
                    let connection_ent_id = match connection.ent {
                        Ident::Symbol(id) => id,
                        _ => return None, // Unreachable
                    };

                    let (connection_port_name, connection_port_span) = match connection.port {
                        Ident::Symbol(id) => (&self.symbols[id].name, &self.symbols[id].span),
                        _ => return None, // Unreachable
                    };
                    match ports.get(connection_port_name) {
                        Some(inst_port) => {
                            let (inst_port_type, inst_port_span) = self.get_ent_type(inst_port.symbol)?;

                            let (connection_ent_type, _connection_ent_span) = self.get_ent_type(connection_ent_id)?;

                            if inst_port_type != connection_ent_type {
                                self.diagnostics.error(CompilerError::MismatchedEntType {
                                    expected: connection_ent_type,
                                    found: inst_port_type,
                                    span: inst_port_span,
                                });

                                return None;
                            }
                        }
                        _ => {
                            self.diagnostics.error(CompilerError::NonexistantNetPort {
                                name: connection_port_name.to_string(),
                                span: connection_port_span.clone(),
                            });

                            return None
                        }
                    }
                }

                Some(NetItem::NetInst(net_inst))
            }

            NetItem::Error => Some(NetItem::Error),
        }
    }

    fn verify_return_type(&mut self, return_type: &Type, expr_type: &Type, rel_span: &Span) -> Option<Type> {
        match (return_type, expr_type) {
            (Type::Impulse, Type::Impulse) => Some(Type::Impulse),

            (Type::Bool, Type::Impulse) => Some(Type::Bool), // Keep?
            (Type::Bool, Type::Bool) => Some(Type::Bool),

            (Type::Mod(val_l), Type::Mod(val_r)) => {
                if val_l == val_r {
                    Some(Type::Mod(*val_l))
                }
                else {
                    self.diagnostics.error(CompilerError::IncompatibleReturnType {
                        return_type: return_type.clone(),
                        expr_type: expr_type.clone(),
                        rel_span: rel_span.clone(),
                    });

                    None
                }
            }
            (Type::Mod(val), Type::Int) => Some(Type::Mod(*val)),

            (Type::Int,  Type::Mod(_)) => Some(Type::Int),
            (Type::Int,  Type::Int   ) => Some(Type::Int),
            (Type::Real, Type::Mod(_)) => Some(Type::Real),
            (Type::Real, Type::Int   ) => Some(Type::Real),
            (Type::Real, Type::Real  ) => Some(Type::Real),

            (Type::Custom(ident_l), Type::Custom(ident_r)) => {
                let (parent_l, parent_r) = match (ident_l, ident_r) {
                    (Ident::Symbol(id_l), Ident::Symbol(id_r)) => (*id_l, *id_r),
                    
                    _ => return None, // Unreachable after name resolution
                };

                if parent_l == parent_r {
                    Some(Type::Custom(Ident::Symbol(parent_l)))
                }
                else {
                    self.diagnostics.error(CompilerError::IncompatibleReturnType {
                        return_type: return_type.clone(),
                        expr_type: expr_type.clone(),
                        rel_span: rel_span.clone(),
                    });

                    None
                }
            }

            _ => {
                self.diagnostics.error(CompilerError::IncompatibleReturnType {
                    return_type: return_type.clone(),
                    expr_type: expr_type.clone(),
                    rel_span: rel_span.clone(),
                });

                None
            }
        }
    }

    fn get_ent_type(&self, id: SymbolId) -> Option<(Type, Span)> {
        match &self.symbols[id].kind {
            SymbolKind::Ent(ty) => {
                Some((ty.clone(), self.symbols[id].span.clone()))
            }
            _ => None,
        }
    }

    fn compare_ent_types(&mut self, symbol_type: Type, object_type: Type, span: Span) -> Option<Type> {
        if symbol_type == object_type {
            return Some(object_type)
        }

        match symbol_type {
            Type::Unknown => {
                Some(object_type)
            }

            _ => {
                self.diagnostics.error(CompilerError::MismatchedEntType {
                    expected: object_type,
                    found: symbol_type,
                    span,
                });

                None
            }
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
    use crate::compiler::parser::Parser;
    use crate::compiler::lexer::Lexer;

    #[test]
    fn check_let_1() {
        // let n = 1;
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "n".to_string(),
                    kind: SymbolKind::Variable(Type::Unknown),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("n".to_string(), 0),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let _result = sem_analyzer.check_let(LetStatement {
            name: Ident::Symbol(0),
            expr: Expr::Literal(Literal::Int(1)),
        });

        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                name: "n".to_string(),
                kind: SymbolKind::Variable(Type::Int),
                span: Span{line: 0, col: 0},
            },
        ]);
    }

    #[test]
    fn check_rel_1() {
        // rel_t ADD : (a: Real) -> Real = a + 1;
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "ADD".to_string(),
                    kind: SymbolKind::Rel_t {
                        input_types: Vec::new(),
                        return_type: Type::Unknown,
                    },
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Variable(Type::Unknown),
                    span: Span{line: 0, col: 5},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("ADD".to_string(), 0),
                        ("a".to_string(), 1),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.check_rel(RelType {
            name: Ident::Symbol(0),
            params: vec![Param {
                name: Ident::Symbol(1),
                param_type: Type::Real,
            }],
            return_type: Type::Real,
            body: Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Ident(Ident::Symbol(1))),
                right: Box::new(Expr::Literal(Literal::Int(1))),
                op: BinaryOp::Add,
                op_span: Span {line: 0, col: 10},
                expr_type: Type::Unknown,
            }),
        });

        assert_eq!(result, Some(RelType {
            name: Ident::Symbol(0),
            params: vec![Param {
                name: Ident::Symbol(1),
                param_type: Type::Real,
            }],
            return_type: Type::Real,
            body: Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Ident(Ident::Symbol(1))),
                right: Box::new(Expr::Literal(Literal::Int(1))),
                op: BinaryOp::Add,
                op_span: Span {line: 0, col: 10},
                expr_type: Type::Real,
            }),
        }));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                name: "ADD".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Real],
                    return_type: Type::Real,
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "a".to_string(),
                kind: SymbolKind::Variable(Type::Real),
                span: Span{line: 0, col: 5},
            },
        ]);
    }

    #[test]
    fn check_rel_2() {
        // rel_t EX : () -> PARENT = CHILD;
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "PARENT".to_string(),
                    kind: SymbolKind::EntType,
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "CHILD".to_string(),
                    kind: SymbolKind::EntMember{
                        parent: 0,
                        mapping: 0,
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "EX".to_string(),
                    kind: SymbolKind::Rel_t {
                        input_types: Vec::new(),
                        return_type: Type::Unknown,
                    },
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("PARENT".to_string(), 0),
                        ("CHILD".to_string(), 1),
                        ("EX".to_string(), 1),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.check_rel(RelType {
            name: Ident::Symbol(2),
            params: vec![],
            return_type: Type::Custom(Ident::Symbol(0)),
            body: Expr::Ident(Ident::Symbol(1)),
        });

        assert_eq!(result, Some(RelType {
            name: Ident::Symbol(2),
            params: vec![],
            return_type: Type::Custom(Ident::Symbol(0)),
            body: Expr::Ident(Ident::Symbol(1)),
        }));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                name: "PARENT".to_string(),
                kind: SymbolKind::EntType,
                span: Span {line: 0, col: 0},
            },
            Symbol {
                name: "CHILD".to_string(),
                kind: SymbolKind::EntMember{
                    parent: 0,
                    mapping: 0,
                },
                span: Span {line: 0, col: 0},
            },
            Symbol {
                name: "EX".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: Vec::new(),
                    return_type: Type::Custom(Ident::Symbol(0)),
                },
                span: Span{line: 0, col: 0},
            },
        ]);
    }

    #[test]
    fn check_rel_3() {
        // rel_t ADD : (a: Int) -> Int = a + 1.0; //  incompatible return type
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "ADD".to_string(),
                    kind: SymbolKind::Rel_t {
                        input_types: Vec::new(),
                        return_type: Type::Unknown,
                    },
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Variable(Type::Unknown),
                    span: Span{line: 0, col: 5},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("ADD".to_string(), 0),
                        ("a".to_string(), 1),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.check_rel(RelType {
            name: Ident::Symbol(0),
            params: vec![Param {
                name: Ident::Symbol(1),
                param_type: Type::Int,
            }],
            return_type: Type::Int,
            body: Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Ident(Ident::Symbol(1))),
                right: Box::new(Expr::Literal(Literal::Real(1.0))),
                op: BinaryOp::Add,
                op_span: Span {line: 0, col: 10},
                expr_type: Type::Unknown,
            }),
        });

        assert_eq!(result, Some(RelType {
            name: Ident::Symbol(0),
            params: vec![Param {
                name: Ident::Symbol(1),
                param_type: Type::Int,
            }],
            return_type: Type::Error,
            body: Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Ident(Ident::Symbol(1))),
                right: Box::new(Expr::Literal(Literal::Real(1.0))),
                op: BinaryOp::Add,
                op_span: Span {line: 0, col: 10},
                expr_type: Type::Real,
            }),
        }));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                name: "ADD".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Int],
                    return_type: Type::Error,
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "a".to_string(),
                kind: SymbolKind::Variable(Type::Int),
                span: Span{line: 0, col: 5},
            },
        ]);
        
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn check_rel_4() {
        // rel_t ADD : (a: Bool) -> Real = a + 1; // incompatible types
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "ADD".to_string(),
                    kind: SymbolKind::Rel_t {
                        input_types: Vec::new(),
                        return_type: Type::Unknown,
                    },
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Variable(Type::Unknown),
                    span: Span{line: 0, col: 5},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("ADD".to_string(), 0),
                        ("a".to_string(), 1),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.check_rel(RelType {
            name: Ident::Symbol(0),
            params: vec![Param {
                name: Ident::Symbol(1),
                param_type: Type::Bool,
            }],
            return_type: Type::Real,
            body: Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Ident(Ident::Symbol(1))),
                right: Box::new(Expr::Literal(Literal::Int(1))),
                op: BinaryOp::Add,
                op_span: Span {line: 0, col: 10},
                expr_type: Type::Unknown,
            }),
        });

        assert_eq!(result, Some(RelType {
            name: Ident::Symbol(0),
            params: vec![Param {
                name: Ident::Symbol(1),
                param_type: Type::Bool,
            }],
            return_type: Type::Error,
            body: Expr::Error,
        }));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                name: "ADD".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Bool],
                    return_type: Type::Error,
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "a".to_string(),
                kind: SymbolKind::Variable(Type::Bool),
                span: Span{line: 0, col: 5},
            },
        ]);
    }

    #[test]
    fn check_net_1() {
        // net TEST {
        //     input a: Bool;
        //     init a: Bool = true;
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "TEST".to_string(),
                    kind: SymbolKind::Net {ports: HashMap::from([
                        ("a".to_string(), NetPort {
                            symbol: 1,
                            input: true,
                        }),
                    ])},
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
            ],
            scopes: vec![],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.check_net(Net {
            name: Ident::Symbol(0),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(1),
                        param_type: Type::Bool,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Init(EntInit {
                    param: Param {
                        name: Ident::Symbol(1),
                        param_type: Type::Bool,
                    },
                    val: Expr::Literal(Literal::Bool(true)),
                }),
            ],
        });

        assert_eq!(result, Some(Net {
            name: Ident::Symbol(0),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(1),
                        param_type: Type::Bool,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Init(EntInit {
                    param: Param {
                        name: Ident::Symbol(1),
                        param_type: Type::Bool,
                    },
                    val: Expr::Literal(Literal::Bool(true)),
                }),
            ],
        }));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                name: "TEST".to_string(),
                kind: SymbolKind::Net {ports: HashMap::from([
                    ("a".to_string(), NetPort {
                        symbol: 1,
                        input: true,
                    }),
                ])},
                span: Span {line: 0, col: 0},
            },
            Symbol {
                name: "a".to_string(),
                kind: SymbolKind::Ent(Type::Bool),
                span: Span {line: 0, col: 0},
            },
        ]);
        assert_eq!(diagnostics.num_errors(), 0);
    }

    #[test]
    fn check_net_2() {
        // net TEST {
        //     input a: Bool;
        //     init a: Bool = 1; // Incorrect type
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "TEST".to_string(),
                    kind: SymbolKind::Net {ports: HashMap::from([
                        ("a".to_string(), NetPort {
                            symbol: 1,
                            input: true,
                        }),
                    ])},
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
            ],
            scopes: vec![],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.check_net(Net {
            name: Ident::Symbol(0),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(1),
                        param_type: Type::Bool,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Init(EntInit {
                    param: Param {
                        name: Ident::Symbol(1),
                        param_type: Type::Bool,
                    },
                    val: Expr::Literal(Literal::Int(1)),
                }),
            ],
        });

        assert_eq!(result, Some(Net {
            name: Ident::Symbol(0),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(1),
                        param_type: Type::Bool,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Error,
            ],
        }));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                name: "TEST".to_string(),
                kind: SymbolKind::Net {ports: HashMap::from([
                    ("a".to_string(), NetPort {
                        symbol: 1,
                        input: true,
                    }),
                ])},
                span: Span {line: 0, col: 0},
            },
            Symbol {
                name: "a".to_string(),
                kind: SymbolKind::Ent(Type::Bool),
                span: Span {line: 0, col: 0},
            },
        ]);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn check_net_3() {
        // net TEST {
        //     input a: Bool;
        //     init a: Int = 1; // Different type
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "TEST".to_string(),
                    kind: SymbolKind::Net {ports: HashMap::from([
                        ("a".to_string(), NetPort {
                            symbol: 1,
                            input: true,
                        }),
                    ])},
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
            ],
            scopes: vec![],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.check_net(Net {
            name: Ident::Symbol(0),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(1),
                        param_type: Type::Bool,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Init(EntInit {
                    param: Param {
                        name: Ident::Symbol(1),
                        param_type: Type::Int,
                    },
                    val: Expr::Literal(Literal::Int(1)),
                }),
            ],
        });

        assert_eq!(result, Some(Net {
            name: Ident::Symbol(0),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(1),
                        param_type: Type::Bool,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Error,
            ],
        }));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                name: "TEST".to_string(),
                kind: SymbolKind::Net {ports: HashMap::from([
                    ("a".to_string(), NetPort {
                        symbol: 1,
                        input: true,
                    }),
                ])},
                span: Span {line: 0, col: 0},
            },
            Symbol {
                name: "a".to_string(),
                kind: SymbolKind::Ent(Type::Bool),
                span: Span {line: 0, col: 0},
            },
        ]);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn check_net_4() {
        // net TEST {
        //     input a: Real;
        //     input b: COIN;
        //     output c: Real;

        //     c := ADD(a, b);
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "COIN".to_string(),
                    kind: SymbolKind::EntType,
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "H".to_string(),
                    kind: SymbolKind::EntMember{
                        parent: 0,
                        mapping: 0,
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "T".to_string(),
                    kind: SymbolKind::EntMember{
                        parent: 0,
                        mapping: 1,
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "ADD".to_string(),
                    kind: SymbolKind::Rel_t {
                        input_types: vec![Type::Real, Type::Custom(Ident::Symbol(0))],
                        return_type: Type::Real,
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "TEST".to_string(),
                    kind: SymbolKind::Net {ports: HashMap::from([
                        ("a".to_string(), NetPort {
                            symbol: 5,
                            input: true,
                        }),
                        ("b".to_string(), NetPort {
                            symbol: 6,
                            input: true,
                        }),
                        ("c".to_string(), NetPort {
                            symbol: 7,
                            input: false,
                        }),
                    ])},
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "b".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "c".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
            ],
            scopes: vec![],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.check_net(Net {
            name: Ident::Symbol(4),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(5),
                        param_type: Type::Real,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(6),
                        param_type: Type::Custom(Ident::Symbol(0)),
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(7),
                        param_type: Type::Real,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::RelInst(RelInst {
                    asignee: Ident::Symbol(7),
                    rel: Ident::Symbol(3),
                    args: vec![
                        Ident::Symbol(5),
                        Ident::Symbol(6),
                    ],
                    span: Span {line: 0, col: 0},
                }),
            ],
        });

        assert_eq!(result, Some(Net {
            name: Ident::Symbol(4),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(5),
                        param_type: Type::Real,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(6),
                        param_type: Type::Custom(Ident::Symbol(0)),
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(7),
                        param_type: Type::Real,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::RelInst(RelInst {
                    asignee: Ident::Symbol(7),
                    rel: Ident::Symbol(3),
                    args: vec![
                        Ident::Symbol(5),
                        Ident::Symbol(6),
                    ],
                    span: Span {line: 0, col: 0},
                }),
            ],
        }));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                name: "COIN".to_string(),
                kind: SymbolKind::EntType,
                span: Span {line: 0, col: 0},
            },
            Symbol {
                name: "H".to_string(),
                kind: SymbolKind::EntMember{
                    parent: 0,
                    mapping: 0,
                },
                span: Span {line: 0, col: 0},
            },
            Symbol {
                name: "T".to_string(),
                kind: SymbolKind::EntMember{
                    parent: 0,
                    mapping: 1,
                },
                span: Span {line: 0, col: 0},
            },
            Symbol {
                name: "ADD".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Real, Type::Custom(Ident::Symbol(0))],
                    return_type: Type::Real,
                },
                span: Span {line: 0, col: 0},
            },
            Symbol {
                name: "TEST".to_string(),
                kind: SymbolKind::Net {ports: HashMap::from([
                    ("a".to_string(), NetPort {
                        symbol: 5,
                        input: true,
                    }),
                    ("b".to_string(), NetPort {
                        symbol: 6,
                        input: true,
                    }),
                    ("c".to_string(), NetPort {
                        symbol: 7,
                        input: false,
                    }),
                ])},
                span: Span {line: 0, col: 0},
            },
            Symbol {
                name: "a".to_string(),
                kind: SymbolKind::Ent(Type::Real),
                span: Span {line: 0, col: 0},
            },
            Symbol {
                name: "b".to_string(),
                kind: SymbolKind::Ent(Type::Custom(Ident::Symbol(0))),
                span: Span {line: 0, col: 0},
            },
            Symbol {
                name: "c".to_string(),
                kind: SymbolKind::Ent(Type::Real),
                span: Span {line: 0, col: 0},
            },
        ]);
        assert_eq!(diagnostics.num_errors(), 0);
    }

    #[test]
    fn check_net_5() {
        // net TEST {
        //     input a: Real;
        //     input b: COIN;
        //     output c: Int; // Wrong type

        //     c := ADD(a, b);
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "COIN".to_string(),
                    kind: SymbolKind::EntType,
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "H".to_string(),
                    kind: SymbolKind::EntMember{
                        parent: 0,
                        mapping: 0,
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "T".to_string(),
                    kind: SymbolKind::EntMember{
                        parent: 0,
                        mapping: 1,
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "ADD".to_string(),
                    kind: SymbolKind::Rel_t {
                        input_types: vec![Type::Real, Type::Custom(Ident::Symbol(0))],
                        return_type: Type::Real,
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "TEST".to_string(),
                    kind: SymbolKind::Net {ports: HashMap::from([
                        ("a".to_string(), NetPort {
                            symbol: 5,
                            input: true,
                        }),
                        ("b".to_string(), NetPort {
                            symbol: 6,
                            input: true,
                        }),
                        ("c".to_string(), NetPort {
                            symbol: 7,
                            input: false,
                        }),
                    ])},
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "b".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "c".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
            ],
            scopes: vec![],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.check_net(Net {
            name: Ident::Symbol(4),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(5),
                        param_type: Type::Real,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(6),
                        param_type: Type::Custom(Ident::Symbol(0)),
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(7),
                        param_type: Type::Int,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::RelInst(RelInst {
                    asignee: Ident::Symbol(7),
                    rel: Ident::Symbol(3),
                    args: vec![
                        Ident::Symbol(5),
                        Ident::Symbol(6),
                    ],
                    span: Span {line: 0, col: 0},
                }),
            ],
        });

        assert_eq!(result, Some(Net {
            name: Ident::Symbol(4),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(5),
                        param_type: Type::Real,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(6),
                        param_type: Type::Custom(Ident::Symbol(0)),
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(7),
                        param_type: Type::Int,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Error,
            ],
        }));
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn check_net_6() {
        // net TEST {
        //     input a: Real;
        //     input b: COIN;
        //     output c: Real;

        //     c := ADD(a); // Missing argument
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "COIN".to_string(),
                    kind: SymbolKind::EntType,
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "H".to_string(),
                    kind: SymbolKind::EntMember{
                        parent: 0,
                        mapping: 0,
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "T".to_string(),
                    kind: SymbolKind::EntMember{
                        parent: 0,
                        mapping: 1,
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "ADD".to_string(),
                    kind: SymbolKind::Rel_t {
                        input_types: vec![Type::Real, Type::Custom(Ident::Symbol(0))],
                        return_type: Type::Real,
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "TEST".to_string(),
                    kind: SymbolKind::Net {ports: HashMap::from([
                        ("a".to_string(), NetPort {
                            symbol: 5,
                            input: true,
                        }),
                        ("b".to_string(), NetPort {
                            symbol: 6,
                            input: true,
                        }),
                        ("c".to_string(), NetPort {
                            symbol: 7,
                            input: false,
                        }),
                    ])},
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "b".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "c".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
            ],
            scopes: vec![],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.check_net(Net {
            name: Ident::Symbol(4),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(5),
                        param_type: Type::Real,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(6),
                        param_type: Type::Custom(Ident::Symbol(0)),
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(7),
                        param_type: Type::Real,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::RelInst(RelInst {
                    asignee: Ident::Symbol(7),
                    rel: Ident::Symbol(3),
                    args: vec![
                        Ident::Symbol(5),
                    ],
                    span: Span {line: 0, col: 0},
                }),
            ],
        });

        assert_eq!(result, Some(Net {
            name: Ident::Symbol(4),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(5),
                        param_type: Type::Real,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(6),
                        param_type: Type::Custom(Ident::Symbol(0)),
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(7),
                        param_type: Type::Real,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Error,
            ],
        }));
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn check_net_7() {
        // net TEST {
        //     input a: Real;
        //     input b: Bool;
        //     output c: Real;

        //     c := ADD(a, b); // Wrong type for b
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "COIN".to_string(),
                    kind: SymbolKind::EntType,
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "H".to_string(),
                    kind: SymbolKind::EntMember{
                        parent: 0,
                        mapping: 0,
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "T".to_string(),
                    kind: SymbolKind::EntMember{
                        parent: 0,
                        mapping: 1,
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "ADD".to_string(),
                    kind: SymbolKind::Rel_t {
                        input_types: vec![Type::Real, Type::Custom(Ident::Symbol(0))],
                        return_type: Type::Real,
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "TEST".to_string(),
                    kind: SymbolKind::Net {ports: HashMap::from([
                        ("a".to_string(), NetPort {
                            symbol: 5,
                            input: true,
                        }),
                        ("b".to_string(), NetPort {
                            symbol: 6,
                            input: true,
                        }),
                        ("c".to_string(), NetPort {
                            symbol: 7,
                            input: false,
                        }),
                    ])},
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "b".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "c".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
            ],
            scopes: vec![],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.check_net(Net {
            name: Ident::Symbol(4),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(5),
                        param_type: Type::Real,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(6),
                        param_type: Type::Bool,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Output(OutputEnt {
                    param: Param {
                        name: Ident::Symbol(7),
                        param_type: Type::Real,
                    },
                }),
                NetItem::RelInst(RelInst {
                    asignee: Ident::Symbol(7),
                    rel: Ident::Symbol(3),
                    args: vec![
                        Ident::Symbol(5),
                        Ident::Symbol(6),
                    ],
                    span: Span {line: 0, col: 0},
                }),
            ],
        });

        assert_eq!(result, Some(Net {
            name: Ident::Symbol(4),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(5),
                        param_type: Type::Real,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(6),
                        param_type: Type::Bool,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Output(OutputEnt {
                    param: Param {
                        name: Ident::Symbol(7),
                        param_type: Type::Real,
                    },
                }),
                NetItem::Error,
            ],
        }));
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn check_net_8() {
        // net TEST {
        //     input a: Real;
        //     input b: COIN;
        //     output c: Real;

        //     ADD {
        //         A := a,
        //         B := b,
        //         C := c,
        //     };
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "COIN".to_string(),
                    kind: SymbolKind::EntType,
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "H".to_string(),
                    kind: SymbolKind::EntMember{
                        parent: 0,
                        mapping: 0,
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "T".to_string(),
                    kind: SymbolKind::EntMember{
                        parent: 0,
                        mapping: 1,
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "ADD".to_string(),
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
                            ("C".to_string(), NetPort {
                                symbol: 6,
                                input: false,
                            }),
                        ]),
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "A".to_string(),
                    kind: SymbolKind::Ent(Type::Real),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "B".to_string(),
                    kind: SymbolKind::Ent(Type::Custom(Ident::Symbol(0))),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "C".to_string(),
                    kind: SymbolKind::Ent(Type::Real),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "TEST".to_string(),
                    kind: SymbolKind::Net {ports: HashMap::from([
                        ("a".to_string(), NetPort {
                            symbol: 8,
                            input: true,
                        }),
                        ("b".to_string(), NetPort {
                            symbol: 6,
                            input: true,
                        }),
                        ("c".to_string(), NetPort {
                            symbol: 10,
                            input: false,
                        }),
                    ])},
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "b".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "c".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
            ],
            scopes: vec![],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.check_net(Net {
            name: Ident::Symbol(4),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(8),
                        param_type: Type::Real,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(9),
                        param_type: Type::Custom(Ident::Symbol(0)),
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(10),
                        param_type: Type::Real,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::NetInst(NetInst {
                    net: Ident::Symbol(3),
                    connections: vec![
                        Connection {
                            port: Ident::Symbol(4),
                            ent: Ident::Symbol(8), 
                            span: Span {line: 0, col: 0},
                        },
                        Connection {
                            port: Ident::Symbol(5),
                            ent: Ident::Symbol(9), 
                            span: Span {line: 0, col: 0},
                        },
                        Connection {
                            port: Ident::Symbol(6),
                            ent: Ident::Symbol(10), 
                            span: Span {line: 0, col: 0},
                        },
                    ],
                }),
            ],
        });

        assert_eq!(result, Some(Net {
            name: Ident::Symbol(4),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(8),
                        param_type: Type::Real,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(9),
                        param_type: Type::Custom(Ident::Symbol(0)),
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(10),
                        param_type: Type::Real,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::NetInst(NetInst {
                    net: Ident::Symbol(3),
                    connections: vec![
                        Connection {
                            port: Ident::Symbol(4),
                            ent: Ident::Symbol(8), 
                            span: Span {line: 0, col: 0},
                        },
                        Connection {
                            port: Ident::Symbol(5),
                            ent: Ident::Symbol(9), 
                            span: Span {line: 0, col: 0},
                        },
                        Connection {
                            port: Ident::Symbol(6),
                            ent: Ident::Symbol(10), 
                            span: Span {line: 0, col: 0},
                        },
                    ],
                }),
            ],
        }));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                name: "COIN".to_string(),
                kind: SymbolKind::EntType,
                span: Span {line: 0, col: 0},
            },
            Symbol {
                name: "H".to_string(),
                kind: SymbolKind::EntMember{
                    parent: 0,
                    mapping: 0,
                },
                span: Span {line: 0, col: 0},
            },
            Symbol {
                name: "T".to_string(),
                kind: SymbolKind::EntMember{
                    parent: 0,
                    mapping: 1,
                },
                span: Span {line: 0, col: 0},
            },
            Symbol {
                name: "ADD".to_string(),
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
                        ("C".to_string(), NetPort {
                            symbol: 6,
                            input: false,
                        }),
                    ]),
                },
                span: Span {line: 0, col: 0},
            },
            Symbol {
                name: "A".to_string(),
                kind: SymbolKind::Ent(Type::Real),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "B".to_string(),
                kind: SymbolKind::Ent(Type::Custom(Ident::Symbol(0))),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "C".to_string(),
                kind: SymbolKind::Ent(Type::Real),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "TEST".to_string(),
                kind: SymbolKind::Net {ports: HashMap::from([
                    ("a".to_string(), NetPort {
                        symbol: 8,
                        input: true,
                    }),
                    ("b".to_string(), NetPort {
                        symbol: 6,
                        input: true,
                    }),
                    ("c".to_string(), NetPort {
                        symbol: 10,
                        input: false,
                    }),
                ])},
                span: Span {line: 0, col: 0},
            },
            Symbol {
                name: "a".to_string(),
                kind: SymbolKind::Ent(Type::Real),
                span: Span {line: 0, col: 0},
            },
            Symbol {
                name: "b".to_string(),
                kind: SymbolKind::Ent(Type::Custom(Ident::Symbol(0))),
                span: Span {line: 0, col: 0},
            },
            Symbol {
                name: "c".to_string(),
                kind: SymbolKind::Ent(Type::Real),
                span: Span {line: 0, col: 0},
            },
        ]);
        assert_eq!(diagnostics.num_errors(), 0);
    }

    #[test]
    fn check_net_9() {
        // net TEST {
        //     input a: Real;
        //     input b: COIN;
        //     output c: Int; // c is wrong type

        //     ADD {
        //         A := a,
        //         B := b,
        //         C := c,
        //     };
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "COIN".to_string(),
                    kind: SymbolKind::EntType,
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "H".to_string(),
                    kind: SymbolKind::EntMember{
                        parent: 0,
                        mapping: 0,
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "T".to_string(),
                    kind: SymbolKind::EntMember{
                        parent: 0,
                        mapping: 1,
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "ADD".to_string(),
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
                            ("C".to_string(), NetPort {
                                symbol: 6,
                                input: false,
                            }),
                        ]),
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "A".to_string(),
                    kind: SymbolKind::Ent(Type::Real),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "B".to_string(),
                    kind: SymbolKind::Ent(Type::Custom(Ident::Symbol(0))),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "C".to_string(),
                    kind: SymbolKind::Ent(Type::Real),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "TEST".to_string(),
                    kind: SymbolKind::Net {ports: HashMap::from([
                        ("a".to_string(), NetPort {
                            symbol: 8,
                            input: true,
                        }),
                        ("b".to_string(), NetPort {
                            symbol: 6,
                            input: true,
                        }),
                        ("c".to_string(), NetPort {
                            symbol: 10,
                            input: false,
                        }),
                    ])},
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "b".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "c".to_string(),
                    kind: SymbolKind::Ent(Type::Unknown),
                    span: Span {line: 0, col: 0},
                },
            ],
            scopes: vec![],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.check_net(Net {
            name: Ident::Symbol(4),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(8),
                        param_type: Type::Real,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(9),
                        param_type: Type::Custom(Ident::Symbol(0)),
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
                NetItem::NetInst(NetInst {
                    net: Ident::Symbol(3),
                    connections: vec![
                        Connection {
                            port: Ident::Symbol(4),
                            ent: Ident::Symbol(8), 
                            span: Span {line: 0, col: 0},
                        },
                        Connection {
                            port: Ident::Symbol(5),
                            ent: Ident::Symbol(9), 
                            span: Span {line: 0, col: 0},
                        },
                        Connection {
                            port: Ident::Symbol(6),
                            ent: Ident::Symbol(10),
                            span: Span {line: 0, col: 0}, 
                        },
                    ],
                }),
            ],
        });

        assert_eq!(result, Some(Net {
            name: Ident::Symbol(4),
            items: vec![
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(8),
                        param_type: Type::Real,
                    },
                    span: Span{line: 0, col: 0},
                }),
                NetItem::Input(InputEnt {
                    param: Param {
                        name: Ident::Symbol(9),
                        param_type: Type::Custom(Ident::Symbol(0)),
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
                NetItem::Error,
            ],
        }));
        assert_eq!(diagnostics.num_errors(), 1);
    }
}