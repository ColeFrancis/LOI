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

//! # fold_const
//!
//! Handles folding of compile-time constants in the annotated ast
//!
//! Author: Cole Francis

use super::SemAnalyzer;
use super::symbol::SymbolKind;

use crate::compiler::parser::ast::*;

impl <'a> SemAnalyzer<'a> {
    pub(super) fn fold_const(&mut self) {
        let items = std::mem::take(&mut self.ast.items);
        self.ast.items = Vec::with_capacity(items.len());

        for item in items {
            if let Some(folded_item) = self.fold_item(item) {
                self.ast.items.push(folded_item);
            }
        }
    }

    fn fold_item(&mut self, item: Item) -> Option<Item> {
        match item {
            Item::Let(stmt) => self.fold_let(stmt, true).map(Item::Let),
            Item::Rel(rel_type) => Some(Item::Rel(self.fold_rel(rel_type))),

            // ent_t have no expressions
            // net expressions are evaluated during synthesis 
            other => Some(other),
        }
    }

    // returning None means the statement was sucessfully folded
    pub(super) fn fold_let(&mut self, mut stmt: LetStatement, fold_sample: bool) -> Option<LetStatement> {
        stmt.expr = self.fold_expr(stmt.expr, fold_sample);

        if let Expr::Literal(literal) = stmt.expr {
            if let Ident::Symbol(id) = stmt.name {
                self.symbols[id].kind = SymbolKind::Const(literal);
            }
            
            return None;
        }

        Some(stmt)
    }

    fn fold_rel(&mut self, mut rel_t: RelType) -> RelType {
        // Do not fold samples as these are evaluated compile-time
        rel_t.body = self.fold_expr(rel_t.body, false);
        
        rel_t
    }

    fn fold_net(&mut self, mut net_t: Net) -> Net {
        // Dont fold expressions inside nets here, as samples must be evaluated for each instantiation
        net_t
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::compiler::sem_analyzer::scope::Scope;
    use crate::compiler::sem_analyzer::symbol::{Symbol, NetPort};
    use crate::compiler::diagnostics::{Diagnostics, Span};
    use crate::compiler::sem_analyzer::types::Type;

    #[test]
    fn fold_let_1() {
        // let n = 1;
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "n".to_string(),
                    kind: SymbolKind::Variable(Type::Int),
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

        let result = sem_analyzer.fold_let(LetStatement {
            name: Ident::Symbol(0),
            expr: Expr::Literal(Literal::Int(1)),
        }, true);

        assert_eq!(result, None);
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                name: "n".to_string(),
                kind: SymbolKind::Const(Literal::Int(1)),
                span: Span{line: 0, col: 0},
            },
        ]);
    }

    #[test]
    fn fold_program() {
        // let n = 1;

        // rel_t ADD_N : (a:Int) -> Int = a + n;

        // net NET {
        //     input in: Int;
        //     output out: Int;

        //     out := ADD_N(in);
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: vec![
                Item::Let(LetStatement {
                    name: Ident::Symbol(0),
                    expr: Expr::Literal(Literal::Int(1)),
                }),
                Item::Rel(RelType {
                    name: Ident::Symbol(1),
                    params: vec![
                        Param {
                            name: Ident::Symbol(2),
                            param_type: Type::Int,
                        }
                    ],
                    return_type: Type::Int,
                    body: Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::Ident(Ident::Symbol(2))),
                        right: Box::new(Expr::Ident(Ident::Symbol(0))),
                        op: BinaryOp::Add,
                        op_span: Span{line: 0, col: 0},
                        expr_type: Type::Int,
                    }),
                }),
                Item::Net(Net {
                    name: Ident::Symbol(3),
                    items: vec![
                        NetItem::Input(Param {
                            name: Ident::Symbol(4),
                            param_type: Type::Int,
                        }),
                        NetItem::Output(Param {
                            name: Ident::Symbol(5),
                            param_type: Type::Int,
                        }),
                        NetItem::RelInst(RelInst {
                            asignee: Ident::Symbol(5),
                            rel: Ident::Symbol(1),
                            args: vec![
                                Ident::Symbol(4),
                            ],
                        }),
                    ],
                }),
            ]},
            symbols: vec![
                Symbol {
                    name: "n".to_string(),
                    kind: SymbolKind::Variable(Type::Int),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "ADD_N".to_string(),
                    kind: SymbolKind::Rel_t {
                        input_types: vec![Type::Int],
                        return_type: Type::Int,
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Variable(Type::Int),
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "NET".to_string(),
                    kind: SymbolKind::Net {
                        ports: HashMap::from([
                            ("in".to_string(), NetPort {
                                symbol: 4,
                                input: true,
                            }),
                            ("in".to_string(), NetPort {
                                symbol: 5,
                                input: false,
                            }),
                        ]),
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "in".to_string(),
                    kind: SymbolKind::Ent(Type::Int),
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "out".to_string(),
                    kind: SymbolKind::Ent(Type::Int),
                    span: Span {line: 0, col: 0},
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

        sem_analyzer.fold_const();

        assert_eq!(sem_analyzer.ast, Program {items: vec![
            Item::Rel(RelType {
                name: Ident::Symbol(1),
                params: vec![
                    Param {
                        name: Ident::Symbol(2),
                        param_type: Type::Int,
                    }
                ],
                return_type: Type::Int,
                body: Expr::Binary(BinaryExpr {
                    left: Box::new(Expr::Ident(Ident::Symbol(2))),
                    right: Box::new(Expr::Literal(Literal::Int(1))),
                    op: BinaryOp::Add,
                    op_span: Span{line: 0, col: 0},
                    expr_type: Type::Int,
                }),
            }),
            Item::Net(Net {
                name: Ident::Symbol(3),
                items: vec![
                    NetItem::Input(Param {
                        name: Ident::Symbol(4),
                        param_type: Type::Int,
                    }),
                    NetItem::Output(Param {
                        name: Ident::Symbol(5),
                        param_type: Type::Int,
                    }),
                    NetItem::RelInst(RelInst {
                        asignee: Ident::Symbol(5),
                        rel: Ident::Symbol(1),
                        args: vec![
                            Ident::Symbol(4),
                        ],
                    }),
                ],
            }),
        ]});
    }
}