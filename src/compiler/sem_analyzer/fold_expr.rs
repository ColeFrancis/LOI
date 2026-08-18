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

//! # fold_expr
//!
//! Handles folding of compile-time constants in expressions
//!
//! ## Invariants:
//!
//! - If an expression is folded into a literal, its type must match the original expr_type
//!
//! Author: Cole Francis

use super::SemAnalyzer;
use super::symbol::SymbolKind;
use super::types::Type;
use crate::compiler::diagnostics::{Diagnostics, Span, CompilerError};

use crate::compiler::parser::ast::*;

impl <'a> SemAnalyzer<'a> {
    pub(super) fn fold_expr (&mut self, expr: Expr) -> Expr {
        match expr {
            Expr::Literal(literal) => Expr::Literal(literal),

            Expr::Ident(Ident::Symbol(id)) => match &self.symbols[id].kind {
                SymbolKind::Const(literal) => Expr::Literal(literal.clone()),
                _ => Expr::Ident(Ident::Symbol(id)),
            }

            Expr::Unary(mut unary) => {
                unary.expr = Box::new(self.fold_expr(*unary.expr));

                if let Expr::Literal(literal) = &*unary.expr {
                    if let Some(result) = self.eval_unary(&unary.op, literal) {
                        return Expr::Literal(result);
                    }
                }

                Expr::Unary(unary)
            }

            Expr::Binary(mut binary) => {
                let mut expr_left = self.fold_expr(*binary.left);
                let mut expr_right = self.fold_expr(*binary.right);

                // if you have int + real convert to real + real to simplify eval_binary
                if binary.expr_type == Type::Real {
                    if let Expr::Literal(Literal::Int(val)) = expr_left {
                        expr_left = Expr::Literal(Literal::Real(val as f64));
                    }

                    if let Expr::Literal(Literal::Int(val)) = expr_right {
                        expr_right = Expr::Literal(Literal::Real(val as f64));
                    }
                }

                binary.left = Box::new(expr_left);
                binary.right = Box::new(expr_right);

                if let (Expr::Literal(left), Expr::Literal(right)) = (&*binary.left, &*binary.right) {
                    if let Some(result) = self.eval_binary(&binary.op, left, right,binary.op_span) {
                        return Expr::Literal(result);
                    }
                }

                Expr::Binary(binary)
            }

            Expr::Tuple(mut exprs) => {
                for expr in &mut exprs {
                    let owned_expr = std::mem::replace(expr, Expr::Error);

                    *expr = self.fold_expr(owned_expr);
                }

                Expr::Tuple(exprs)
            }

            Expr::Block(mut block_expr) => {
                let owned_statements = std::mem::take(&mut block_expr.statements);

                for statement in owned_statements {
                    if let Statement::Let(let_statement) = statement {
                        if let Some(folded_let_statement) = self.fold_let(let_statement) {
                            block_expr.statements.push(Statement::Let(folded_let_statement));
                        }
                    }
                }

                match self.fold_expr(*block_expr.expr) {
                    Expr::Literal(literal) => Expr::Literal(literal),

                    other => {
                        block_expr.expr = Box::new(other);

                        Expr::Block(block_expr)
                    }
                }
            }

            Expr::Cases(mut cases_expr) => {
                // Verify case statements dont have duplicate patterns (including defaults).
                // let scrutinee = self.fold_expr(*cases_expr.scrutinee);

                Expr::Cases(cases_expr)
            }

            Expr::Sample(mut sample_expr) => {
                // Verify there is at most 1 default arm and that probabilites add up to 1.

                Expr::Sample(sample_expr)
            }

            _ => Expr::Error,
        }
    }

    fn eval_unary(&self, op: &UnaryOp, literal: &Literal) -> Option<Literal> {
        match (op, literal) {
            (UnaryOp::BitNot, Literal::Bool(x)) => Some(Literal::Bool(!x)),

            (UnaryOp::Neg, Literal::Int(x)) => Some(Literal::Int(-x)),

            (UnaryOp::Neg, Literal::Real(x)) => Some(Literal::Real(-x)),

            _ => None,
        }
    }

    fn eval_binary(&mut self, op: &BinaryOp, left: &Literal, right: &Literal, op_span: Span) -> Option<Literal> {
        match (op, left, right) {
            (BinaryOp::Or, Literal::Bool(a), Literal::Bool(b)) => Some(Literal::Bool(*a || *b)),
            (BinaryOp::And, Literal::Bool(a), Literal::Bool(b)) => Some(Literal::Bool(*a && *b)),

            (BinaryOp::Lt,  Literal::Int(a), Literal::Int(b)) => Some(Literal::Bool(a<b)),
            (BinaryOp::Gt,  Literal::Int(a), Literal::Int(b)) => Some(Literal::Bool(a>b)),
            (BinaryOp::Le,  Literal::Int(a), Literal::Int(b)) => Some(Literal::Bool(a<=b)),
            (BinaryOp::Ge,  Literal::Int(a), Literal::Int(b)) => Some(Literal::Bool(a>=b)),
            (BinaryOp::Add, Literal::Int(a), Literal::Int(b)) => Some(Literal::Int(a+b)),
            (BinaryOp::Sub, Literal::Int(a), Literal::Int(b)) => Some(Literal::Int(a-b)),
            (BinaryOp::Mul, Literal::Int(a), Literal::Int(b)) => Some(Literal::Int(a*b)),
            (BinaryOp::Div, Literal::Int(a), Literal::Int(b)) => 
                if *b != 0 {
                    Some(Literal::Int(a / b))
                } else {
                    self.diagnostics.error(CompilerError::DivideByZero {
                        op_span, 
                    });

                    None
                },
            (BinaryOp::Pow, Literal::Int(a), Literal::Int(b)) => 
                if *b >= 0 {
                    Some(Literal::Int(a.pow(*b as u32)))
                } else { // negative exponent not allowed for ints
                    self.diagnostics.error(CompilerError::NegExpOnInt {
                        op_span,
                    });

                    None
                },

            (BinaryOp::Lt,  Literal::Real(a), Literal::Real(b)) => Some(Literal::Bool(a<b)),
            (BinaryOp::Gt,  Literal::Real(a), Literal::Real(b)) => Some(Literal::Bool(a>b)),
            (BinaryOp::Le,  Literal::Real(a), Literal::Real(b)) => Some(Literal::Bool(a<=b)),
            (BinaryOp::Ge,  Literal::Real(a), Literal::Real(b)) => Some(Literal::Bool(a>=b)),
            (BinaryOp::Add, Literal::Real(a), Literal::Real(b)) => Some(Literal::Real(a+b)),
            (BinaryOp::Sub, Literal::Real(a), Literal::Real(b)) => Some(Literal::Real(a-b)),
            (BinaryOp::Mul, Literal::Real(a), Literal::Real(b)) => Some(Literal::Real(a*b)),
            (BinaryOp::Div, Literal::Real(a), Literal::Real(b)) => 
                if *b != 0.0 {
                    Some(Literal::Real(a / b))
                } else {
                    self.diagnostics.error(CompilerError::DivideByZero {
                        op_span, 
                    });

                    None
                },
            (BinaryOp::Pow, Literal::Real(a), Literal::Real(b)) => Some(Literal::Real(a.powf(*b))),

            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::compiler::sem_analyzer::scope::Scope;
    use crate::compiler::sem_analyzer::symbol::Symbol;

    #[test]
    fn test_const () {
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Const(Literal::Int(1)),
                    span: Span{line: 0, col: 0},
                },
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

        let result = sem_analyzer.fold_expr(Expr::Ident(Ident::Symbol(0)));

        assert_eq!(result, Expr::Literal(Literal::Int(1)));
    }

    #[test]
    fn test_unary_1() {
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer::new(
            Program {items: Vec::new()},
            &mut diagnostics
        );

        let result = sem_analyzer.fold_expr(Expr::Unary(UnaryExpr {
            expr: Box::new(Expr::Literal(Literal::Bool(false))),
            op: UnaryOp::BitNot,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Bool,
        }));

        assert_eq!(result, Expr::Literal(Literal::Bool(true)));
    }

    #[test]
    fn test_unary_2() {
        // -a
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Const(Literal::Real(1.0)),
                    span: Span{line: 0, col: 0},
                },
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

        let result = sem_analyzer.fold_expr(Expr::Unary(UnaryExpr {
            expr: Box::new(Expr::Ident(Ident::Symbol(0))),
            op: UnaryOp::Neg,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Real,
        }));

        assert_eq!(result, Expr::Literal(Literal::Real(-1.0)));
    }

    #[test]
    fn test_binary_1() {
        // 1.0 + 2^3
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer::new(
            Program {items: Vec::new()},
            &mut diagnostics
        );

        let result = sem_analyzer.fold_expr(Expr::Binary(BinaryExpr{
            left: Box::new(Expr::Literal(Literal::Real(1.0))),
            right: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Literal(Literal::Int(2))),
                right: Box::new(Expr::Literal(Literal::Int(3))),
                op: BinaryOp::Pow,
                op_span: Span{line: 0, col: 1},
                expr_type: Type::Int,
            })),
            op: BinaryOp::Add,
            op_span: Span {line: 0, col: 0},
            expr_type: Type::Real,
        }));

        assert_eq!(result, Expr::Literal(Literal::Real(9.0)));
    }

    #[test]
    fn test_binary_2() {
        // 1.0 + 2^(-3) // Neg exp on integer error
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer::new(
            Program {items: Vec::new()},
            &mut diagnostics
        );

        let result = sem_analyzer.fold_expr(Expr::Binary(BinaryExpr{
            left: Box::new(Expr::Literal(Literal::Real(1.0))),
            right: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Literal(Literal::Int(2))),
                right: Box::new(Expr::Unary(UnaryExpr {
                    expr: Box::new(Expr::Literal(Literal::Int(3))),
                    op: UnaryOp::Neg,
                    op_span: Span{line: 0, col:2},
                    expr_type: Type::Int,
                })),
                op: BinaryOp::Pow,
                op_span: Span{line: 0, col: 1},
                expr_type: Type::Int,
            })),
            op: BinaryOp::Add,
            op_span: Span {line: 0, col: 0},
            expr_type: Type::Real,
        }));

        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn test_binary_3() {
        // 1.0 + 2.0^(-3)
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer::new(
            Program {items: Vec::new()},
            &mut diagnostics
        );

        let result = sem_analyzer.fold_expr(Expr::Binary(BinaryExpr{
            left: Box::new(Expr::Literal(Literal::Real(1.0))),
            right: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Literal(Literal::Real(2.0))),
                right: Box::new(Expr::Unary(UnaryExpr {
                    expr: Box::new(Expr::Literal(Literal::Int(3))),
                    op: UnaryOp::Neg,
                    op_span: Span{line: 0, col:2},
                    expr_type: Type::Int,
                })),
                op: BinaryOp::Pow,
                op_span: Span{line: 0, col: 1},
                expr_type: Type::Real,
            })),
            op: BinaryOp::Add,
            op_span: Span {line: 0, col: 0},
            expr_type: Type::Real,
        }));

        assert_eq!(result, Expr::Literal(Literal::Real(1.125)));
    }

    #[test]
    fn test_binary_4() {
        // n + (a + 1) where n is an unknown variable and a is a folded constant
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "n".to_string(),
                    kind: SymbolKind::Variable(Type::Int),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Const(Literal::Int(1)),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("n".to_string(), 0),
                        ("a".to_string(), 1),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.fold_expr(Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Ident(Ident::Symbol(0))),
            right: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Ident(Ident::Symbol(1))),
                right: Box::new(Expr::Literal(Literal::Int(1))),
                op: BinaryOp::Add,
                op_span: Span{line: 0, col: 0},
                expr_type: Type::Int,
            })),
            op: BinaryOp::Add,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Int,
        }));

        assert_eq!(result, Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Ident(Ident::Symbol(0))),
            right: Box::new(Expr::Literal(Literal::Int(2))),
            op: BinaryOp::Add,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Int,
        }));
    }

    #[test]
    fn test_binary_5() {
        // 1 < 2
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer::new(
            Program {items: Vec::new()},
            &mut diagnostics
        );

        let result = sem_analyzer.fold_expr(Expr::Binary(BinaryExpr{
            left: Box::new(Expr::Literal(Literal::Int(1))),
            right: Box::new(Expr::Literal(Literal::Int(2))),
            op: BinaryOp::Lt,
            op_span: Span {line: 0, col: 0},
            expr_type: Type::Int,
        }));

        assert_eq!(result, Expr::Literal(Literal::Bool(true)));
    }

    #[test]
    fn test_block_1() {
        // {
        //     let n = 2;
        //     n-1
        // }

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

        let result = sem_analyzer.fold_expr(Expr::Block(BlockExpr {
            statements: vec![
                Statement::Let(LetStatement {
                    name: Ident::Symbol(0),
                    expr: Expr::Literal(Literal::Int(2)),
                }),
            ],
            expr: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Ident(Ident::Symbol(0))),
                right: Box::new(Expr::Literal(Literal::Int(1))),
                op: BinaryOp::Sub,
                op_span: Span{line: 0, col: 0},
                expr_type: Type::Int,
            })),
            expr_type: Type::Int
        }));

        assert_eq!(result, Expr::Literal(Literal::Int(1)));
    }

    #[test]
    fn test_block_2() {
        // {
        //     let n = 1 + bool; // wrong types result in Expr::Error
        //     n-1
        // }

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

        let result = sem_analyzer.fold_expr(Expr::Block(BlockExpr {
            statements: vec![
                Statement::Let(LetStatement {
                    name: Ident::Symbol(0),
                    expr: Expr::Error,
                }),
            ],
            expr: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Ident(Ident::Symbol(0))),
                right: Box::new(Expr::Literal(Literal::Int(1))),
                op: BinaryOp::Sub,
                op_span: Span{line: 0, col: 0},
                expr_type: Type::Int,
            })),
            expr_type: Type::Int
        }));

        assert_eq!(result, Expr::Block(BlockExpr {
            statements: vec![
                Statement::Let(LetStatement {
                    name: Ident::Symbol(0),
                    expr: Expr::Error,
                }),
            ],
            expr: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Ident(Ident::Symbol(0))),
                right: Box::new(Expr::Literal(Literal::Int(1))),
                op: BinaryOp::Sub,
                op_span: Span{line: 0, col: 0},
                expr_type: Type::Int,
            })),
            expr_type: Type::Int
        }));
    }

    #[test]
    fn test_block_3() {
        // {
        //     let n =  bool; 
        //     n-1 // Expr error
        // }

        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "n".to_string(),
                    kind: SymbolKind::Variable(Type::Bool),
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

        let result = sem_analyzer.fold_expr(Expr::Block(BlockExpr {
            statements: vec![
                Statement::Let(LetStatement {
                    name: Ident::Symbol(0),
                    expr: Expr::Literal(Literal::Bool(true)),
                }),
            ],
            expr: Box::new(Expr::Error),
            expr_type: Type::Int
        }));

        assert_eq!(result, Expr::Block(BlockExpr {
            statements: vec![],
            expr: Box::new(Expr::Error),
            expr_type: Type::Int
        }));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                name: "n".to_string(),
                kind: SymbolKind::Const(Literal::Bool(true)),
                span: Span{line: 0, col: 0},
            },
        ]);
    }
}