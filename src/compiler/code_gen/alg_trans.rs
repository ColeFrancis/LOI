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

//! # alg_trans
//!
//! Handles the algebraic transformation of expressions for relation optimization
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
    pub(super) fn algebraic_transform(expr: Expr) -> Expr {
        match expr {
            Expr::Literal(literal) => Expr::Literal(literal),

            Expr::Ident(ident) => Expr::Ident(ident),

            // DONE
            Expr::Unary(mut unary) => {
                unary.expr = Box::new(Self::algebraic_transform(*unary.expr));

                match unary.op {
                    // -(-x) -> x DONE
                    UnaryOp::Neg => {
                        if let Expr::Unary(inner_unary) = &*unary.expr {
                            if let UnaryOp::Neg = inner_unary.op {
                                if let Expr::Unary(inner_unary) = *unary.expr {
                                    return *inner_unary.expr;
                                }
                            }
                        }

                        Expr::Unary(unary)
                    }

                    // ~(~x) -> x DONE
                    UnaryOp::BitNot => {
                        if let Expr::Unary(inner_unary) = &*unary.expr {
                            if let UnaryOp::BitNot = inner_unary.op {
                                if let Expr::Unary(inner_unary) = *unary.expr {
                                    return *inner_unary.expr;
                                }
                            }
                        }

                        Expr::Unary(unary)
                    }
                }
            }

            // NOT DONE
            Expr::Binary(mut binary) => {
                binary.left = Box::new(Self::algebraic_transform(*binary.left));
                binary.right = Box::new(Self::algebraic_transform(*binary.right));

                match binary.op {
                    // x + (-x) -> 0
                    // x + (-y) -> x - y
                    // x + x -> x * 2
                    // x + a * x -> x * (a + 1)
                    // a * x + b * x -> x * (a + b) 
                    // a + (x + b) -> x + (a + b) (a, b are literal)
                    // a + x -> x + a (a is literal)
                    // x + 0 -> x DONE
                    BinaryOp::Add => {
                        match (&*binary.left, &*binary.right) {
                            // 0 + x
                            (&Expr::Literal(Literal::Int(0)), _) => return *binary.right,

                            // 0.0 + x
                            (&Expr::Literal(Literal::Real(0.0)), _) => return *binary.right,

                            // x + 0
                            (_, &Expr::Literal(Literal::Int(0))) => return *binary.left,

                            // x + 0.0
                            (_, &Expr::Literal(Literal::Real(0.0))) => return *binary.left,

                            _ => {},
                        }

                        Expr::Binary(binary)
                    }

                    // x - (-y) -> x + y
                    // x - x -> 0
                    // a * x - b * x -> x * (a - b)
                    // a * x - x -> x * (a - 1)
                    // x - a * x -> x * (1 - a)
                    // 0 - x -> -x DONE
                    // x - 0 -> x DONE
                    BinaryOp::Sub => {
                        match (&*binary.left, &*binary.right) {
                            // 0 - x
                            (&Expr::Literal(Literal::Int(0)), _) => return Expr::Unary(UnaryExpr {
                                expr: binary.right,
                                op: UnaryOp::Neg,
                                op_span: Span{line: 0, col: 0}, // span no longer matters
                                expr_type: binary.expr_type,
                            }),

                            // 0.0 - x
                            (&Expr::Literal(Literal::Real(0.0)), _) => return Expr::Unary(UnaryExpr {
                                expr: binary.right,
                                op: UnaryOp::Neg,
                                op_span: Span{line: 0, col: 0}, // span no longer matters
                                expr_type: binary.expr_type,
                            }),

                            // x - 0
                            (_, &Expr::Literal(Literal::Int(0))) => return *binary.left,

                            // x - 0.0
                            (_, &Expr::Literal(Literal::Real(0.0))) => return *binary.left,

                            _ => {},
                        }

                        // This does not work if spans are different
                        if Self::expr_equal(&*binary.left, &*binary.right) {
                            return Expr::Literal(Literal::Int(0))
                        }

                        Expr::Binary(binary)
                    }

                    // (-x) * (-y) -> x * y
                    // x * (-y) -> -(x * y)
                    // a * (x * b) -> x * (a * b) (a, b are literal)
                    // a * x -> x * a (a is literal)
                    // x * 1 -> x DONE
                    // x * 0 -> 0 DONE
                    BinaryOp::Mul => {
                        match (&*binary.left, &*binary.right) {
                            // 0 * x
                            (&Expr::Literal(Literal::Int(0)), _) => return Expr::Literal(Literal::Int(0)),

                            // 0.0 * x
                            (&Expr::Literal(Literal::Real(0.0)), _) => return Expr::Literal(Literal::Int(0)),

                            // x * 0
                            (_, &Expr::Literal(Literal::Int(0))) => return Expr::Literal(Literal::Int(0)),

                            // x * 0.0
                            (_, &Expr::Literal(Literal::Real(0.0))) => return Expr::Literal(Literal::Int(0)),

                            // 1 * x
                            (&Expr::Literal(Literal::Int(1)), _) => return *binary.right,

                            // 1.0 * x
                            (&Expr::Literal(Literal::Real(1.0)), _) => return *binary.right,

                            // x * 1
                            (_, &Expr::Literal(Literal::Int(1))) => return *binary.left,

                            // x * 1.0
                            (_, &Expr::Literal(Literal::Real(1.0))) => return *binary.left,

                            _ => {},
                        }

                        Expr::Binary(binary)
                    }

                    // (-x) / (-y) -> x / y
                    // x / (-y) -> -(x / y)
                    // (-x) / y -> -(x / y)
                    // 0 / x -> 0 DONE
                    // x / 1 -> x
                    // x / x -> 1 if x is zero
                    BinaryOp::Div => {
                        match (&*binary.left, &*binary.right) {
                            // 0 / x
                            (&Expr::Literal(Literal::Int(0)), _) => return Expr::Literal(Literal::Int(0)),

                            // 0.0 / x
                            (&Expr::Literal(Literal::Real(0.0)), _) => return Expr::Literal(Literal::Int(0)),

                            _ => {},
                        }

                        Expr::Binary(binary)
                    }

                    // 1 ^ x -> 1 DONE
                    // x ^ 1 -> x DONE
                    // x ^ 0 -> 1 DONE
                    BinaryOp::Pow => {
                        match (&*binary.left, &*binary.right) {
                            // x ^ 0
                            (_, &Expr::Literal(Literal::Int(0))) => return Expr::Literal(Literal::Int(1)),

                            // x ^ 0.0
                            (_, &Expr::Literal(Literal::Real(0.0))) => return Expr::Literal(Literal::Int(1)),

                            // 1 ^ x
                            (&Expr::Literal(Literal::Int(1)), _) => return Expr::Literal(Literal::Int(1)),

                            // 1.0 ^ x
                            (&Expr::Literal(Literal::Real(1.0)), _) => return Expr::Literal(Literal::Int(1)),

                            // x ^ 1
                            (_, &Expr::Literal(Literal::Int(1))) => return *binary.left,

                            // x ^ 1.0
                            (_, &Expr::Literal(Literal::Real(1.0))) => return *binary.left,

                            _ => {},
                        }

                        Expr::Binary(binary)
                    }

                    // x & x -> x
                    // x & ~x -> false
                    // (~x) & (~y) -> ~(x | y)
                    // x & true -> x DONE
                    // x & false -> false DONE
                    BinaryOp::And => {
                        match (&*binary.left, &*binary.right) {
                            // true & x
                            (&Expr::Literal(Literal::Bool(true)), _) => return *binary.right,

                            // x & true
                            (_, &Expr::Literal(Literal::Bool(true))) => return *binary.left,
                            
                            // false & x
                            (&Expr::Literal(Literal::Bool(false)), _) => return Expr::Literal(Literal::Bool(false)),

                            // x & true
                            (_, &Expr::Literal(Literal::Bool(false))) => return Expr::Literal(Literal::Bool(false)),

                            _ => {},
                        }

                        Expr::Binary(binary)
                    }

                    // x | x -> x
                    // x | ~x -> true
                    // (~x) | (~y) -> ~(x & y)
                    // x | true -> true DONE
                    // x | false -> x DONE
                    BinaryOp::Or => {
                        match (&*binary.left, &*binary.right) {
                            // true | x
                            (&Expr::Literal(Literal::Bool(true)), _) => return Expr::Literal(Literal::Bool(true)),

                            // x | true
                            (_, &Expr::Literal(Literal::Bool(true))) => return Expr::Literal(Literal::Bool(true)),
                            
                            // false | x
                            (&Expr::Literal(Literal::Bool()), _) => return *binary.right,

                            // x | true
                            (_, &Expr::Literal(Literal::Bool(false))) => return *binary.left,

                            _ => {},
                        }

                        Expr::Binary(binary)
                    }

                    // x xor true -> ~x
                    // x xor false -> x
                    // BinaryOp::Xor

                    // x == x -> true
                    // BinaryOp::Eq

                    // x != x -> false
                    // BinaryOp::Ne

                    _ => Expr::Binary(binary)
                }
            }

            // DONE
            Expr::Tuple(mut tuple) => {
                for expr in &mut tuple {
                    let owned_expr = std::mem::replace(expr, Expr::Error);

                    *expr = Self::algebraic_transform(owned_expr);
                }

                Expr::Tuple(tuple)
            }

            // NOT DONE
            // Expr::Block(block) => {}

            // NOT DONE
            // if two arm expressions are the same, merge them to one arm with a vec of simple patterns
            // if two simple patterns are the same in one arm, merge them
            // Expr::Cases(cases) => {}

            // NOT DONE
            // if two arm expressions are the same, merge them into one arm where the prob is the sum of both
            // Expr::Sample(sample) => {}

            _ => Expr::Error,
        }
    }

    // Compares two expressions to see if their equal.
    // cannot naively compare as spans may be different. 
    //      Maybe you can just set all the spans to zero though?
    fn expr_equal(left: &Expr, right: &Expr) -> bool {
        // match (left, right) {

        // }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::sem_analyzer::types::Type;

    #[test]
    fn unary_cancel() {
        // -(-x)
        let expr = Expr::Unary(UnaryExpr {
            expr: Box::new(Expr::Unary(UnaryExpr {
                expr: Box::new(Expr::Ident(Ident::Symbol(0))),
                op: UnaryOp::Neg,
                op_span: Span{line: 0, col: 0},
                expr_type: Type::Int,
            })),
            op: UnaryOp::Neg,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Int,
        });

        let result = CodeGen::algebraic_transform(expr);

        assert_eq!(result, Expr::Ident(Ident::Symbol(0)));
    }

    #[test]
    fn binary_add() {
        // x + 0
        let expr = Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Ident(Ident::Symbol(0))),
            right: Box::new(Expr::Literal(Literal::Int(0))),
            op: BinaryOp::Add,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Int,
        });

        let result = CodeGen::algebraic_transform(expr);

        assert_eq!(result, Expr::Ident(Ident::Symbol(0)));
    }

    #[test]
    fn binary_sub_1() {
        // -(0.0 - x)
        let expr = Expr::Unary(UnaryExpr {
            expr: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Literal(Literal::Real(0.0))),
                right: Box::new(Expr::Ident(Ident::Symbol(0))),
                op: BinaryOp::Sub,
                op_span: Span{line: 0, col: 0},
                expr_type: Type::Real,
            })),
            op: UnaryOp::Neg,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Real,
        });

        let result = CodeGen::algebraic_transform(expr);

        assert_eq!(result, Expr::Ident(Ident::Symbol(0)));
    }

    // #[test]
    // fn binary_sub_1() {
    //     // 3*x - 3*x
    //     let expr = Expr::Binary(BinaryExpr {
    //         left: Box::new(Expr::Binary(BinaryExpr {
    //             left: Box::new(Expr::Literal(Literal::Int(3))),
    //             right: Box::new(Expr::Ident(Ident::Symbol(0))),
    //             op: BinaryOp::Mul,
    //             op_span: Span{line: 0, col: 0},
    //         })),
    //         right: Box::new(Expr::),
    //         op: BinaryOp::Sub,
    //         op_span: Span{line: 0, col: 0},
    //         expr_type: Type::Int,
    //     });

    //     let result = CodeGen::algebraic_transform(expr);

    //     assert_eq!(result, Expr::Literal(Literal::Int(0)));
    // }

    #[test]
    fn mul_zero() {
        // x * 0
        let expr = Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Ident(Ident::Symbol(0))),
            right: Box::new(Expr::Literal(Literal::Int(0))),
            op: BinaryOp::Mul,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Int,
        });

        let result = CodeGen::algebraic_transform(expr);

        assert_eq!(result, Expr::Literal(Literal::Int(0)));
    }

    #[test]
    fn zero_div() {
        // 0 / x
        let expr = Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Literal(Literal::Int(0))),
            right: Box::new(Expr::Ident(Ident::Symbol(0))),
            op: BinaryOp::Div,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Int,
        });

        let result = CodeGen::algebraic_transform(expr);

        assert_eq!(result, Expr::Literal(Literal::Int(0)));
    }
}