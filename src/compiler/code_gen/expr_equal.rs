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

//! # expr_equal
//!
//! testing two functions for algebraic equivalence
//!
//! ## Invariants
//!
//! - Returned expression must always have the same behavior as inputed function (excepting neglegable differences in outputted floating point values)
//!
//! Author: Cole Francis

use super::CodeGen;
use crate::compiler::ast::*;
    use crate::compiler::diagnostics::Span;
use crate::compiler::compiled_rel::CompiledRel;

impl CodeGen {
    // Compares two expressions to see if their equal.
    // cannot naively compare as spans may be different. 
    //      Maybe you can just set all the spans to zero though?
    pub(super) fn expr_equal(left: &Expr, right: &Expr) -> bool {
        match (left, right) {
            (Expr::Literal(left_literal), Expr::Literal(right_literal)) => {
                // Int(a) == Real(b) is true if  a == b in value
                match (left_literal, right_literal) {
                    (Literal::Int(left_val), Literal::Real(right_val)) => {
                        *left_val as f64 == *right_val
                    }

                    (Literal::Real(left_val), Literal::Int(right_val)) => {
                        *left_val == *right_val as f64
                    }

                    _ => left_literal == right_literal,
                }
            }

            (Expr::Ident(left_ident), Expr::Ident(right_ident)) => left_ident == right_ident,

            (Expr::Unary(left_unary), Expr::Unary(right_unary)) => {
                if left_unary.op != right_unary.op {
                    return false;
                }

                Self::expr_equal(&left_unary.expr, &right_unary.expr)
            }

            (Expr::Binary(left_binary), Expr::Binary(right_binary)) => {
                if left_binary.op != right_binary.op {
                    return false;
                }

                if Self::expr_equal(&left_binary.left, &right_binary.left) && Self::expr_equal(&left_binary.right, &right_binary.right) {
                    return true;
                }

                match left_binary.op {
                    // Commutative
                    // BinaryOp::Eq
                    // BinaryOp::Ne
                    BinaryOp::Add |
                    BinaryOp::Mul |
                    BinaryOp::Or  |
                    BinaryOp::And => {}

                    // Not commutative
                    _ => return false,
                }

                // if the op is commutative, the a + b == b + a
                Self::expr_equal(&left_binary.left, &right_binary.right) && Self::expr_equal(&left_binary.right, &right_binary.left)
            }

            (Expr::Tuple(left_tuple), Expr::Tuple(right_tuple)) => {
                // if any are not equal, the entire expression is not equal
                for (left_expr, right_expr) in left_tuple.iter().zip(right_tuple.iter()) {
                    if !Self::expr_equal(left_expr, right_expr) {
                        return false;
                    }
                }

                true
            }

            // There are more cases that are missed but the point of this function is not 
            // to find exaustive equivalence but a quick check of algebraic equivalence
            
            _ => false,
        }
    }

    fn statements_equal(left: &Statement, right: &Statement) -> bool {
        match (left, right) {
            (Statement::Let(left_let_statement), Statement::Let(right_let_statement)) => {
                if left_let_statement.name != right_let_statement.name {
                    return false;
                }

                Self::expr_equal(&left_let_statement.expr, &right_let_statement.expr)
            }

            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::sem_analyzer::types::Type;

    #[test]
    fn test_binary_1() {
        // x + 3
        let left = Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Ident(Ident::Symbol(0))),
            right: Box::new(Expr::Literal(Literal::Int(3))),
            op: BinaryOp::Add,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Int,
        });

        // 3.0 + x
        let right = Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Literal(Literal::Real(3.0))),
            right: Box::new(Expr::Ident(Ident::Symbol(0))),
            op: BinaryOp::Add,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Int,
        });

        let result = CodeGen::expr_equal(&left, &right);

        assert!(result);
    }

    #[test]
    fn test_binary_2() {
        // x - 3
        let left = Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Ident(Ident::Symbol(0))),
            right: Box::new(Expr::Literal(Literal::Int(3))),
            op: BinaryOp::Sub,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Int,
        });

        // 3.0 - x
        let right = Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Literal(Literal::Real(3.0))),
            right: Box::new(Expr::Ident(Ident::Symbol(0))),
            op: BinaryOp::Sub,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Int,
        });

        let result = CodeGen::expr_equal(&left, &right);

        assert!(!result);
    }
}