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
//! Handles the core of the semantic analyzer
//!
//! ## Invariants
//!
//! - 
//!
//! Author: Cole Francis

use std::collections::HashMap;

use super::SemAnalyzer;
use super::scope::Scope;
use crate::compiler::{
    symbol::Symbol,
    ast::Program,
    diagnostics::Diagnostics,
};

impl <'a> SemAnalyzer<'a> {
    pub fn new(ast: Program, diagnostics: &'a mut Diagnostics) -> Self {
        Self {
            ast,
            symbols: Vec::new(),
            scopes: vec![
                Scope {
                    symbols: HashMap::new(),
                }
            ],
            diagnostics,
        }
    }

    pub fn analyze(mut self) -> (Program, Vec<Symbol>) {
        self.resolve_names();
        self.check_types();
        self.fold_const();
        self.check_constraints();

        (self.ast, self.symbols)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::compiler::lexer::Lexer;
    use crate::compiler::parser::Parser;
    use crate::compiler::ast::*;
    use crate::compiler::diagnostics::Span;
    use crate::compiler::sem_analyzer::types::Type;
    use crate::compiler::symbol::{SymbolKind, NetPort};

    #[test]
    fn integrate_front_end() {
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
    input a: Int;
    output c: Int;
    init a: Int = n;

    FIRST {
        a := a,
        q := c,
    };
}
        ", &mut diagnostics).tokenize();

        let program = Parser::new(tokens, &mut diagnostics).parse();

        let (validated_program, symbols) = SemAnalyzer::new(program, &mut diagnostics).analyze();


        assert_eq!(symbols, vec![
            Symbol {
                name: "a".to_string(),
                kind: SymbolKind::Const(Literal::Int(1)),
                span: Span {line: 3, col: 9},
            },
            Symbol {
                name: "n".to_string(),
                kind: SymbolKind::Const(Literal::Int(2)),
                span: Span {line: 2, col: 5},
            },
            Symbol {
                name: "SINGLE".to_string(),
                kind: SymbolKind::EntType,
                span: Span {line: 7, col: 7},
            },
            Symbol {
                name: "A".to_string(),
                kind: SymbolKind::EntMember{
                    parent: 2,
                    mapping: 0,
                },
                span: Span {line: 7, col: 17},
            },
            Symbol {
                name: "ADD".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Int],
                    return_type: Type::Int,
                },
                span: Span {line: 9, col: 7},
            },
            Symbol {
                name: "b".to_string(),
                kind: SymbolKind::Variable(Type::Int),
                span: Span {line: 9, col: 14},
            },
            Symbol {
                name: "FIRST".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([
                        ("a".to_string(), NetPort {
                            symbol: 7,
                            input: true,
                        }),
                        ("q".to_string(), NetPort {
                            symbol: 8,
                            input: false,
                        }),
                    ])
                },
                span: Span {line: 11, col: 5},
            },
            Symbol {
                name: "a".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span {line: 12, col: 11},
            },
            Symbol {
                name: "q".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span {line: 13, col: 12},
            },
            Symbol {
                name: "SECOND".to_string(),
                kind: SymbolKind::Net {
                    ports: HashMap::from([
                        ("a".to_string(), NetPort {
                            symbol: 10,
                            input: true,
                        }),
                        ("c".to_string(), NetPort {
                            symbol: 11,
                            input: false,
                        }),
                    ])
                },
                span: Span {line: 18, col: 5},
            },
            Symbol {
                name: "a".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span {line: 19, col: 11},
            },
            Symbol {
                name: "c".to_string(),
                kind: SymbolKind::Ent(Type::Int),
                span: Span {line: 20, col: 12},
            },
        ]);
        assert_eq!(validated_program, Program {items: vec![
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
                    left: Box::new(Expr::Literal(Literal::Int(2))),
                    op: BinaryOp::Add,
                    right: Box::new(Expr::Ident(Ident::Symbol(5))),
                    op_span: Span {line: 9, col: 33},
                    expr_type: Type::Int,
                }),
            }),
            Item::Net(Net {
                name: Ident::Symbol(6),
                items: vec![
                    NetItem::Input(InputEnt {
                        param: Param {
                            name: Ident::Symbol(7),
                            param_type: Type::Int,
                        },
                        span: Span{line: 12, col: 5},
                    }),
                    NetItem::Output(OutputEnt {
                        param: Param {
                            name: Ident::Symbol(8),
                            param_type: Type::Int,
                        },
                    }),
                    NetItem::RelInst(RelInst {
                        asignee: Ident::Symbol(8),
                        rel: Ident::Symbol(4),
                        args: vec![
                            Ident::Symbol(7),
                        ],
                        span: Span {line: 15, col: 7},
                    }),
                ],
            }),
            Item::Net(Net {
                name: Ident::Symbol(9),
                items: vec![
                    NetItem::Input(InputEnt {
                        param: Param {
                            name: Ident::Symbol(10),
                            param_type: Type::Int,
                        },
                        span: Span{line: 19, col: 5},
                    }),
                    NetItem::Output(OutputEnt {
                        param: Param {
                            name: Ident::Symbol(11),
                            param_type: Type::Int,
                        },
                    }),
                    NetItem::Init(EntInit {
                        param: Param {
                            name: Ident::Symbol(10),
                            param_type: Type::Int
                        },
                        val: Expr::Literal(Literal::Int(2)),
                    }),
                    NetItem::NetInst(NetInst {
                        net: Ident::Symbol(6),
                        connections: vec![
                            Connection {
                                port: Ident::Symbol(7),
                                ent: Ident::Symbol(10),
                                span: Span {line: 24, col: 11},
                            },
                            Connection {
                                port: Ident::Symbol(8),
                                ent: Ident::Symbol(11),
                                span: Span {line: 25, col: 11},
                            },
                        ],
                    }),
                ],
            })
        ]});
    }
}