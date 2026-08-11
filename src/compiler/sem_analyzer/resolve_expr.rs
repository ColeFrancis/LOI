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

//! # resolve_expression
//!
//! Handles name resolution and building the symbol table of for expressions only
//!
//! ## Invariants
//!
//! - Must use the same ast as in parsing, just change Idents from Ident::Str to Ident::Symbol
//!
//! Author: Cole Francis

use std::collections::HashMap;

use super::SemAnalyzer;
use super::symbol::{Symbol, SymbolKind, SymbolId};
use super::scope::Scope;
use super::types::Type;
use crate::compiler::parser::ast::*;
use crate::compiler::diagnostics::{CompilerError, Span};

impl <'a> SemAnalyzer<'a> {
    // Unlike parsing expressions, if any part of an expression is an error (undefined ident),
    //  then the whole expression does not become an error, only that portion. This allows for
    //  more helpful diagnostics
    pub(super) fn resolve_expr(&mut self, expr: Expr) -> Option<Expr> {
        match expr { 
            Expr::Literal(literal) => Some(Expr::Literal(literal)),

            Expr::Ident(ident) => {
                let (name, span) = self.extract_ident_str(ident)?;

                let symbol_id = self.find_symbol(&name, span)?;

                Some(Expr::Ident(Ident::Symbol(symbol_id)))
            },

            Expr::Unary(mut unary_expr) => {
                unary_expr.expr = Box::new(self.resolve_expr(*unary_expr.expr)
                    .unwrap_or(Expr::Error));
                    
                Some(Expr::Unary(unary_expr))
            }

            Expr::Binary(mut binary_expr) => {
                binary_expr.left = Box::new(self.resolve_expr(*binary_expr.left)
                    .unwrap_or(Expr::Error));
                
                binary_expr.right = Box::new(self.resolve_expr(*binary_expr.right)
                    .unwrap_or(Expr::Error));
                    
                Some(Expr::Binary(binary_expr))
            }

            Expr::Tuple(mut tuple_expr) => {
                for expr in &mut tuple_expr {
                    let owned_expr = std::mem::replace(expr, Expr::Error);

                    *expr = self.resolve_expr(owned_expr)
                        .unwrap_or(Expr::Error);
                }

                Some(Expr::Tuple(tuple_expr))
            }

            Expr::Block(mut block_expr) => { // TODO: Refactor
                self.create_scope();

                for stmt in &mut block_expr.statements {
                    let owned_stmt = std::mem::replace(stmt, Statement::Error);

                    *stmt = match owned_stmt {
                        Statement::Let(let_stmt) => {
                            match self.resolve_let(let_stmt) {
                                Some(resolved_let) => Statement::Let(resolved_let),
                                None => Statement::Error,
                            }
                        }
                        Statement::Error => Statement::Error
                    }
                }

                block_expr.expr = Box::new(self.resolve_expr(*block_expr.expr)
                    .unwrap_or(Expr::Error));

                self.exit_scope();
                
                Some(Expr::Block(block_expr))
            }

            Expr::Match(match_expr) => {
                match self.resolve_match_expr(match_expr) {
                    Some(expr) => Some(Expr::Match(expr)),
                    None => Some(Expr::Error),
                }
            }

            Expr::Sample(sample_expr) => {
                match self.resolve_sample_expr(sample_expr) {
                    Some(expr) => Some(Expr::Sample(expr)),
                    None => Some(Expr::Error),
                }
            }

            Expr::Error => Some(Expr::Error),
        }
    }

    fn resolve_match_expr(&mut self, mut match_expr: MatchExpr) -> Option<MatchExpr> {
        match_expr.scrutinee = Box::new(self.resolve_expr(*match_expr.scrutinee)
            .unwrap_or(Expr::Error));

        for arm in &mut match_expr.arms {
            for simple_pattern in &mut arm.pattern {
                let owned_pattern = std::mem::replace(simple_pattern, SimplePattern::Error);

                *simple_pattern = self.resolve_simple_pattern(owned_pattern)
                    .unwrap_or(SimplePattern::Error);
            }

            let owned_expr = std::mem::replace(&mut arm.expr, Expr::Error);

            arm.expr = self.resolve_expr(owned_expr).unwrap_or(Expr::Error);
        }

        Some(match_expr)
    }

    fn resolve_simple_pattern(&mut self, simple_pattern: SimplePattern) -> Option<SimplePattern> {
        match simple_pattern {
            SimplePattern::Default => Some(SimplePattern::Default),

            SimplePattern::Literal(literal) => Some(SimplePattern::Literal(literal)),

            SimplePattern::Ident(ident) => {
                let (name, span) = self.extract_ident_str(ident)?;

                let symbol = self.find_symbol(&name, span)?;

                Some(SimplePattern::Ident(Ident::Symbol(symbol)))
            }

            SimplePattern::Tuple(mut tuple_pattern) => {
                for pattern in &mut tuple_pattern {
                    let owned_pattern = std::mem::replace(pattern, SimplePattern::Error);

                    *pattern = self.resolve_simple_pattern(owned_pattern)
                        .unwrap_or(SimplePattern::Error);
                }

                Some(SimplePattern::Tuple(tuple_pattern))
            }

            SimplePattern::Comparison(mut comparison_pattern) => {
                comparison_pattern.expr = Box::new(self.resolve_expr(*comparison_pattern.expr)
                    .unwrap_or(Expr::Error));

                Some(SimplePattern::Comparison(comparison_pattern))
            }

            SimplePattern::Error => Some(SimplePattern::Error),
        }
    }

    fn resolve_sample_expr(&mut self, mut sample_expr: SampleExpr) -> Option<SampleExpr> {
        for arm in &mut sample_expr.arms {
            let owned_prob = std::mem::replace(&mut arm.prob, Prob::Default);
            arm.prob = match owned_prob {
                Prob::Default => Prob::Default,
                Prob::Expr(expr) => Prob::Expr(self.resolve_expr(expr)
                    .unwrap_or(Expr::Error)),
            };

            let owned_expr = std::mem::replace(&mut arm.expr, Expr::Error);

            arm.expr = self.resolve_expr(owned_expr).unwrap_or(Expr::Error);
        }
        
        Some(sample_expr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::diagnostics::Diagnostics;
    use crate::compiler::sem_analyzer::scope::Scope;

    #[test]
    fn expr_literal() {
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![],
            scopes: vec![
                Scope {
                    symbols: HashMap::new(),
                },
            ],

            diagnostics: &mut Diagnostics::new(),
        };

        let result = sem_analyzer.resolve_expr(Expr::Literal(Literal::Bool(false)));

        assert_eq!(result, Some(Expr::Literal(Literal::Bool(false))));
    }
    
    #[test]
    fn expr_ident_fail() {
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![],
            scopes: vec![
                Scope {
                    symbols: HashMap::new(),
                },
            ],

            diagnostics: &mut Diagnostics::new(),
        };

        let result = sem_analyzer.resolve_expr(Expr::Ident(Ident::Str {
            val: "a".to_string(),
            span: Span{line: 1, col: 0},
        }));

        assert_eq!(result, None);
    }

    #[test]
    fn expr_ident() {
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("a".to_string(), 1),
                    ]),
                },
            ],

            diagnostics: &mut Diagnostics::new(),
        };

        let result = sem_analyzer.resolve_expr(Expr::Ident(Ident::Str {
            val: "a".to_string(),
            span: Span{line: 2, col: 0},
        }));

        assert_eq!(result, Some(Expr::Ident(Ident::Symbol(1))));
    }

    #[test]
    fn expr_binary() {
        // a + 1  // a undefined
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                    ]),
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.resolve_expr(Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Ident(Ident::Str {
                val: "a".to_string(),
                span: Span{line: 0, col: 0},
            })),
            op: BinaryOp::Add,
            right: Box::new(Expr::Literal(Literal::Int(1))),
            expr_type: Type::Unknown,
        }));

        assert_eq!(result, Some(Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Error),
            op: BinaryOp::Add,
            right: Box::new(Expr::Literal(Literal::Int(1))),
            expr_type: Type::Unknown,
        })));
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn expr_tuple() {
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("a".to_string(), 1),
                    ]),
                },
            ],

            diagnostics: &mut Diagnostics::new(),
        };

        let result = sem_analyzer.resolve_expr(Expr::Tuple(vec![
            Expr::Ident(Ident::Str {
                val: "a".to_string(),
                span: Span{line: 2, col: 0},
            }),
            Expr::Ident(Ident::Str {
                val: "b".to_string(),
                span: Span{line: 2, col: 1},
            }),
        ]));

        assert_eq!(result, Some(Expr::Tuple(vec![Expr::Ident(Ident::Symbol(1)), Expr::Error])));
    }

    #[test]
    fn expr_block_1() {
        // {
        //     let n = 1;
        //     n
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer::new(
            Program{items: vec![]},
            &mut diagnostics,
        );

        let result = sem_analyzer.resolve_expr(Expr::Block(BlockExpr {
            statements: vec![
                Statement::Let(LetStatement {
                    name: Ident::Str {
                        val: "n".to_string(),
                        span: Span{line: 2, col: 0},
                    },
                    expr: Expr::Literal(Literal::Int(1)),
                }),
            ],
            expr: Box::new(Expr::Ident(Ident::Str {
                val: "n".to_string(),
                span: Span{line: 3, col: 0},
            })),
            expr_type: Type::Unknown,
        }));

        assert_eq!(result, Some(Expr::Block(BlockExpr {
            statements: vec![
                Statement::Let(LetStatement {
                    name: Ident::Symbol(0),
                    expr: Expr::Literal(Literal::Int(1)),
                })
            ],
            expr: Box::new(Expr::Ident(Ident::Symbol(0))),
            expr_type: Type::Unknown,
        })));
        assert_eq!(sem_analyzer.scopes, vec![
            Scope {
                symbols: HashMap::new(),
            },
        ]);
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                id: 0,
                name: "n".to_string(),
                kind: SymbolKind::Variable(Type::Unknown),
                span: Span{line: 2, col: 0},
            }
        ]);
    }

    #[test]
    fn expr_block_2() {
        // {
        //     let n = {
        //         let n = {
        //             let n = {
        //                 let n = 1;
        //                 n
        //             };
        //             n
        //         };
        //         n
        //     };
        //     n
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer::new(
            Program{items: vec![]},
            &mut diagnostics,
        );

        let result = sem_analyzer.resolve_expr(Expr::Block(BlockExpr {
            statements: vec![
                Statement::Let(LetStatement {
                    name: Ident::Str {
                        val: "n".to_string(),
                        span: Span{line: 0, col: 0},
                    },
                    expr: Expr::Block(BlockExpr {
                        statements: vec![
                            Statement::Let(LetStatement {
                                name: Ident::Str {
                                    val: "n".to_string(),
                                    span: Span{line: 0, col: 0},
                                },
                                expr: Expr::Block(BlockExpr {
                                    statements: vec![
                                        Statement::Let(LetStatement {
                                            name: Ident::Str {
                                                val: "n".to_string(),
                                                span: Span{line: 0, col: 0},
                                            },
                                            expr: Expr::Block(BlockExpr {
                                                statements: vec![
                                                    Statement::Let(LetStatement {
                                                        name: Ident::Str {
                                                            val: "n".to_string(),
                                                            span: Span{line: 0, col: 0},
                                                        },
                                                        expr: Expr::Literal(Literal::Int(1)),
                                                    }),
                                                ],
                                                expr: Box::new(Expr::Ident(Ident::Str {
                                                    val: "n".to_string(),
                                                    span: Span{line: 0, col: 0},
                                                })),
                                                expr_type: Type::Unknown,
                                            }),
                                        }),
                                    ],
                                    expr: Box::new(Expr::Ident(Ident::Str {
                                        val: "n".to_string(),
                                        span: Span{line: 0, col: 0},
                                    })),
                                    expr_type: Type::Unknown,
                                }),
                            }),
                        ],
                        expr: Box::new(Expr::Ident(Ident::Str {
                            val: "n".to_string(),
                            span: Span{line: 0, col: 0},
                        })),
                        expr_type: Type::Unknown,
                    }),
                }),
            ],
            expr: Box::new(Expr::Ident(Ident::Str {
                val: "n".to_string(),
                span: Span{line: 0, col: 0},
            })),
            expr_type: Type::Unknown,
        }));

        assert_eq!(result, Some(Expr::Block(BlockExpr {
            statements: vec![
                Statement::Let(LetStatement {
                    name: Ident::Symbol(0),
                    expr: Expr::Block(BlockExpr {
                        statements: vec![
                            Statement::Let(LetStatement {
                                name: Ident::Symbol(1),
                                expr: Expr::Block(BlockExpr {
                                    statements: vec![
                                        Statement::Let(LetStatement {
                                            name: Ident::Symbol(2),
                                            expr: Expr::Block(BlockExpr {
                                                statements: vec![
                                                    Statement::Let(LetStatement {
                                                        name: Ident::Symbol(3),
                                                        expr: Expr::Literal(Literal::Int(1)),
                                                    }),
                                                ],
                                                expr: Box::new(Expr::Ident(Ident::Symbol(3))),
                                                expr_type: Type::Unknown,
                                            }),
                                        }),
                                    ],
                                    expr: Box::new(Expr::Ident(Ident::Symbol(2))),
                                    expr_type: Type::Unknown,
                                }),
                            }),
                        ],
                        expr: Box::new(Expr::Ident(Ident::Symbol(1))),
                        expr_type: Type::Unknown,
                    }),
                }),
            ],
            expr: Box::new(Expr::Ident(Ident::Symbol(0))),
            expr_type: Type::Unknown,
        })));
        assert_eq!(sem_analyzer.scopes, vec![
            Scope {
                symbols: HashMap::new(),
            },
        ]);
    }

    #[test]
    fn bad_expr_block() {
        // {
        //     let n = 1;
        //     m   // undefined
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer::new(
            Program{items: vec![]},
            &mut diagnostics,
        );

        let result = sem_analyzer.resolve_expr(Expr::Block(BlockExpr {
            statements: vec![
                Statement::Let(LetStatement {
                    name: Ident::Str {
                        val: "n".to_string(),
                        span: Span{line: 2, col: 0},
                    },
                    expr: Expr::Literal(Literal::Int(1)),
                }),
            ],
            expr: Box::new(Expr::Ident(Ident::Str {
                val: "m".to_string(),
                span: Span{line: 3, col: 0},
            })),
            expr_type: Type::Unknown,
        }));

        assert_eq!(result, Some(Expr::Block(BlockExpr {
            statements: vec![
                Statement::Let(LetStatement {
                    name: Ident::Symbol(0),
                    expr: Expr::Literal(Literal::Int(1)),
                })
            ],
            expr: Box::new(Expr::Error),
            expr_type: Type::Unknown,
        })));
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn expr_match_1() {
        // match a {
        //     >= 0.5 => b, // b undefined
        //     _ => 0
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("a".to_string(), 1),
                    ]),
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.resolve_expr(Expr::Match(MatchExpr {
            scrutinee: Box::new(Expr::Ident(Ident::Str {
                val: "a".to_string(),
                span: Span{line: 1, col: 0}
            })),
            arms: vec![
                MatchArm {
                    pattern: vec![SimplePattern::Comparison(ComparisonPattern {
                        op: CompOp::Ge,
                        expr: Box::new(Expr::Literal(Literal::Real(0.5))),
                    })],
                    expr: Expr::Ident(Ident::Str {
                        val: "b".to_string(),
                        span: Span{line: 2, col: 0},
                    }),
                },
                MatchArm {
                    pattern: vec![SimplePattern::Default],
                    expr: Expr::Literal(Literal::Int(0)),
                }
            ],
            expr_type: Type::Unknown,
        }));

        assert_eq!(result, Some(Expr::Match(MatchExpr {
            scrutinee: Box::new(Expr::Ident(Ident::Symbol(1))),
            arms: vec![
                MatchArm {
                    pattern: vec![SimplePattern::Comparison(ComparisonPattern {
                        op: CompOp::Ge,
                        expr: Box::new(Expr::Literal(Literal::Real(0.5))),
                    })],
                    expr: Expr::Error,
                },
                MatchArm {
                    pattern: vec![SimplePattern::Default],
                    expr: Expr::Literal(Literal::Int(0)),
                }
            ],
            expr_type: Type::Unknown,
        })));
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn expr_match_2() {
        // match a {  // a undefined
        //     >= 0.5 => b,
        //     _ => 0
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("b".to_string(), 10),
                    ]),
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.resolve_expr(Expr::Match(MatchExpr {
            scrutinee: Box::new(Expr::Ident(Ident::Str {
                val: "a".to_string(),
                span: Span{line: 1, col: 0}
            })),
            arms: vec![
                MatchArm {
                    pattern: vec![SimplePattern::Comparison(ComparisonPattern {
                        op: CompOp::Ge,
                        expr: Box::new(Expr::Literal(Literal::Real(0.5))),
                    })],
                    expr: Expr::Ident(Ident::Str {
                        val: "b".to_string(),
                        span: Span{line: 2, col: 0},
                    }),
                },
                MatchArm {
                    pattern: vec![SimplePattern::Default],
                    expr: Expr::Literal(Literal::Int(0)),
                }
            ],
            expr_type: Type::Unknown,
        }));

        assert_eq!(result, Some(Expr::Match(MatchExpr {
            scrutinee: Box::new(Expr::Error),
            arms: vec![
                MatchArm {
                    pattern: vec![SimplePattern::Comparison(ComparisonPattern {
                        op: CompOp::Ge,
                        expr: Box::new(Expr::Literal(Literal::Real(0.5))),
                    })],
                    expr: Expr::Ident(Ident::Symbol(10)),
                },
                MatchArm {
                    pattern: vec![SimplePattern::Default],
                    expr: Expr::Literal(Literal::Int(0)),
                }
            ],
            expr_type: Type::Unknown,
        })));
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn expr_sample() {
        // sample {
        //     a => b, // b undefined
        //     _ => false
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                    ]),
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.resolve_expr(Expr::Sample(SampleExpr {
            arms: vec![
                SampleArm {
                    prob: Prob::Expr(Expr::Ident(Ident::Str{
                        val: "a".to_string(),
                        span: Span{line: 0, col: 0}
                    })),
                    expr: Expr::Ident(Ident::Str{
                        val: "b".to_string(),
                        span: Span{line: 1, col: 0},
                    })
                },
                SampleArm {
                    prob: Prob::Default,
                    expr: Expr::Literal(Literal::Bool(false)),
                }
            ],
            expr_type: Type::Unknown,
        }));

        assert_eq!(result, Some(Expr::Sample(SampleExpr {
            arms: vec![
                SampleArm {
                    prob: Prob::Expr(Expr::Error),
                    expr: Expr::Error,
                },
                SampleArm {
                    prob: Prob::Default,
                    expr: Expr::Literal(Literal::Bool(false)),
                }
            ],
            expr_type: Type::Unknown,
        })));
        assert_eq!(diagnostics.num_errors(), 2);
    }
}