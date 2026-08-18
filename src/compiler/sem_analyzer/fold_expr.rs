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
//! Author: Cole Francis

use super::SemAnalyzer;

use crate::compiler::parser::ast::*;

impl <'a> SemAnalyzer<'a> {
    pub fold_expr (&mut self, expr: Expr) -> Expr {
        match expr {
            Expr::Literal(literal) => Expr::Literal(literal),

            Expr::Ident(ident) => {
                if let Ident::Symbol(id) = ident {
                    match &self.symbols[id].kind {
                        SymblKind::Const(literal) => Expr::Literal(literal),

                        _ => Expr::Ident(ident),
                    }
                }
            }

            Expr::Unary(mut unary) => {
                unary.expr = Box::new(self.fold_expr(*unary.expr));

                if let Expr::Literal(literal) = &*unary.expr {
                    if let Some(result) = self.eval_unary(unary.op, literal) {
                        return Expr::Literal(result);
                    }
                }
            }

            Expr::Binary(mut binary) => {
                let mut expr_left = self.fold_expr(*binary.left);
                let mut expr_right = self.fold_expr(*binary.right);

                // if you have int + real convert to real + real to simplify eval_binary
                if binary.expr_type == Type::Real {
                    if let Expr::Literal(Literal::Int(val)) = expr_left {
                        expr_left = Expr::Literal(Literal::Real(val));
                    }

                    if let Expr::Literal(Literal::Int(val)) = expr_left {
                        expr_left = Expr::Literal(Literal::Real(val));
                    }
                }

                binary.left = Box::new(expr_left);
                binary.right = Box::new(expr_right);

                if let (Expr::Literal(left), Expr::Literal(right)) = (&*binary.left, &*binary.right) {
                    if let Some(result) = self.eval_binary(binary.op, left, right) {
                        return Expr::Literal(result);
                    }
                }

                Expr::Binary(binary)
            }

            _ => Expr::Error,
        }
    }

    fn eval_unary(&self, op: UnaryOp, literal: &Literal) -> Option<Literal> {
        match (op, literal) {
            (UnaryOp::BitNot, Literal::Bool(x)) => Some(Literal::Bool(!x)),

            (UnaryOp::Neg, Literal::Int(x)) => Some(Literal::Int(-x)),

            (UnaryOp::Neg, Literal::Real(x)) => Some(Literal::Real(-x)),

            _ => None,
        }
    }

    fn eval_binary(&self, op: BinaryOp, left: &Literal, right: &Literal) -> Option<Literal> {
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
                    // TODO: report divide by zero?
                    None
                },
            (BinaryOp::Pow, Literal::Int(a), Literal::Int(b)) => 
                if *b >= 0 {
                    Some(Literal::Int(a.pow(*b as u32)))
                } else { // negative exponent not allowed for ints
                    // TODO, report error
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
                if *b != 0 {
                    Some(Literal::Real(a / b))
                } else {
                    // TODO: report divide by zero?
                    None
                },
            (BinaryOp::Pow, Literal::Real(a), Literal::Real(b)) => Some(Literal::Real(a.powf(*b)))

            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let result = self.fold_expr(Expr::Ident(Ident::Symbol(0)));

        assert_eq!(result, Expr::Literal(Literal::Int(1)));
    }

    #[test]
    fn test_unary_1() {
        let sem_analyzer = SemAnalyzer::new(
            Program {items: Vec::new()},
            &mut Diagnostics::new()
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
}