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
    pub(super) fn algebraic_transform(expr: Expr) -> (Expr, bool) {
        match expr {
            Expr::Literal(literal) => (Expr::Literal(literal), false),

            Expr::Ident(ident) => (Expr::Ident(ident), false),

            // DONE
            Expr::Unary(mut unary) => {
                let (transformed_expr, modified) = Self::algebraic_transform(*unary.expr);
                unary.expr = Box::new(transformed_expr);

                match unary.op {
                    // -(-x) -> x DONE
                    // -(x + y) -> -x - y
                    // -(x - y) -> y - x
                    UnaryOp::Neg => {
                        if let Expr::Unary(inner_unary) = &*unary.expr {
                            if let UnaryOp::Neg = inner_unary.op {
                                if let Expr::Unary(inner_unary) = *unary.expr {
                                    return (*inner_unary.expr, true);
                                }
                            }
                        }

                        (Expr::Unary(unary), modified)
                    }

                    // ~(~x) -> x DONE
                    UnaryOp::BitNot => {
                        if let Expr::Unary(inner_unary) = &*unary.expr {
                            if let UnaryOp::BitNot = inner_unary.op {
                                if let Expr::Unary(inner_unary) = *unary.expr {
                                    return (*inner_unary.expr, true);
                                }
                            }
                        }

                        (Expr::Unary(unary), modified)
                    }
                }
            }

            // NOT DONE
            Expr::Binary(mut binary) => {
                let (transformed_left, left_modified) = Self::algebraic_transform(*binary.left);
                let (transformed_right, right_modified) = Self::algebraic_transform(*binary.right);
                binary.left = Box::new(transformed_left);
                binary.right = Box::new(transformed_right);

                let modified = left_modified | right_modified;

                match binary.op {
                    // x + 0 -> x DONE
                    // x + x -> x * 2 DONE
                    // x + (x + y) -> x*2 + y
                    // x + (x - y) -> x*2 - y
                    // x + (y - x) -> y
                    // x + x * y -> x * (y + 1) DONE (NOT included for now)
                    // x + x/y -> x *(1/y + 1)
                    // x * y + x * z -> x * (y + z) 
                    // x + (-x) -> 0 DONE
                    // x + (-y) -> x - y DONE
                    // a + (x + b) -> x + (a + b) (a, b are literal) 
                    // a + (x - b) -> x + (a - b) (a, b ar literal)
                    // a + x -> x + a (a is literal) DONE
                    BinaryOp::Add => {
                        match (&*binary.left, &*binary.right) {
                            // 0 + x
                            (&Expr::Literal(Literal::Int(0)), _) => return (*binary.right, true),

                            // 0.0 + x
                            (&Expr::Literal(Literal::Real(0.0)), _) => return (*binary.right, true),

                            // x + 0
                            (_, &Expr::Literal(Literal::Int(0))) => return (*binary.left, true),

                            // x + 0.0
                            (_, &Expr::Literal(Literal::Real(0.0))) => return (*binary.left, true),

                            _ => {},
                        }

                        // x + x -> x * 2
                        if Self::expr_equal(&binary.left, &binary.right) {
                            return (Expr::Binary(BinaryExpr {
                                left: Box::new(*binary.left),
                                right: Box::new(Expr::Literal(Literal::Int(2))),
                                op: BinaryOp::Mul,
                                op_span: Span{line: 0, col: 0},
                                expr_type: binary.expr_type,
                            }), true);
                        }

                        // x * a + x * b -> x * (a + b)

                        // x + x * y -> x * (y + 1)
                        // let pull_out_left = match &*binary.right {
                        //     Expr::Binary(sub_binary) if sub_binary.op == BinaryOp::Mul => {
                        //         if Self::expr_equal(&binary.left, &sub_binary.left) {
                        //             Some(true)
                        //         }
                        //         else if Self::expr_equal(&binary.left, &sub_binary.right) {
                        //             Some(false)
                        //         }
                        //         else {
                        //             None
                        //         }
                        //     }
                        //     _ => None
                        // };
                        // match pull_out_left {
                        //     Some(true) => if let Expr::Binary(sub_binary) = *binary.right {
                        //         return Expr::Binary(BinaryExpr {
                        //             left: Box::new(*binary.left),
                        //             right: Box::new(Expr::Binary(BinaryExpr {
                        //                 left: Box::new(*sub_binary.left),
                        //                 right: Box::new(Expr::Literal(Literal::Int(1))),
                        //                 op: BinaryOp::Add,
                        //                 op_span: Span{line: 0, col: 0},
                        //                 expr_type: sub_binary.expr_type,
                        //             })),
                        //             op: BinaryOp::Mul,
                        //             op_span: Span{line: 0, col: 0},
                        //             expr_type: binary.expr_type,
                        //         });
                        //     }

                        //     Some(false) => if let Expr::Binary(sub_binary) = *binary.right {
                        //         return Expr::Binary(BinaryExpr {
                        //             left: Box::new(*binary.left),
                        //             right: Box::new(Expr::Binary(BinaryExpr {
                        //                 left: Box::new(*sub_binary.right),
                        //                 right: Box::new(Expr::Literal(Literal::Int(1))),
                        //                 op: BinaryOp::Add,
                        //                 op_span: Span{line: 0, col: 0},
                        //                 expr_type: sub_binary.expr_type,
                        //             })),
                        //             op: BinaryOp::Mul,
                        //             op_span: Span{line: 0, col: 0},
                        //             expr_type: binary.expr_type,
                        //         });
                        //     }

                        //     _ => {}
                        // }

                        // x * y + x -> x * (y + 1)
                        // let pull_out_left = match &*binary.left {
                        //     Expr::Binary(sub_binary) if sub_binary.op == BinaryOp::Mul => {
                        //         if Self::expr_equal(&binary.right, &sub_binary.left) {
                        //             Some(true)
                        //         }
                        //         else if Self::expr_equal(&binary.right, &sub_binary.right) {
                        //             Some(false)
                        //         }
                        //         else {
                        //             None
                        //         }
                        //     }
                        //     _ => None
                        // };
                        // match pull_out_left {
                        //     Some(true) => if let Expr::Binary(sub_binary) = *binary.left {
                        //         return Expr::Binary(BinaryExpr {
                        //             left: Box::new(*binary.right),
                        //             right: Box::new(Expr::Binary(BinaryExpr {
                        //                 left: Box::new(*sub_binary.left),
                        //                 right: Box::new(Expr::Literal(Literal::Int(1))),
                        //                 op: BinaryOp::Add,
                        //                 op_span: Span{line: 0, col: 0},
                        //                 expr_type: sub_binary.expr_type,
                        //             })),
                        //             op: BinaryOp::Mul,
                        //             op_span: Span{line: 0, col: 0},
                        //             expr_type: binary.expr_type,
                        //         });
                        //     }

                        //     Some(false) => if let Expr::Binary(sub_binary) = *binary.left {
                        //         return Expr::Binary(BinaryExpr {
                        //             left: Box::new(*binary.right),
                        //             right: Box::new(Expr::Binary(BinaryExpr {
                        //                 left: Box::new(*sub_binary.right),
                        //                 right: Box::new(Expr::Literal(Literal::Int(1))),
                        //                 op: BinaryOp::Add,
                        //                 op_span: Span{line: 0, col: 0},
                        //                 expr_type: sub_binary.expr_type,
                        //             })),
                        //             op: BinaryOp::Mul,
                        //             op_span: Span{line: 0, col: 0},
                        //             expr_type: binary.expr_type,
                        //         });
                        //     }

                        //     _ => {}
                        // }

                        // x + (-x) -> 0, x + (-y) -> x - y
                        let negate_right = match &*binary.right {
                            Expr::Unary(unary) if unary.op == UnaryOp::Neg => {
                                if Self::expr_equal(&binary.left, &unary.expr) {
                                    return (Expr::Literal(Literal::Int(0)), true);
                                }

                                true
                            }
                            _ => false,
                        };
                        if negate_right {
                            if let Expr::Unary(unary) = *binary.right {
                                binary.right = unary.expr;
                                binary.op = BinaryOp::Sub;

                                return (Expr::Binary(binary), true);
                            }
                        }

                        // (-x) + x -> 0, (-x) + y -> y - x
                        let negate_left = match &*binary.left {
                            Expr::Unary(unary) if unary.op == UnaryOp::Neg => {
                                if Self::expr_equal(&binary.right, &unary.expr) {
                                    return (Expr::Literal(Literal::Int(0)), true);
                                }

                                true
                            }
                            _ => false,
                        };
                        if negate_left {
                            if let Expr::Unary(unary) = *binary.left {
                                binary.left = binary.right;
                                binary.right = unary.expr;
                                binary.op = BinaryOp::Sub;

                                return (Expr::Binary(binary), true);
                            }
                        }

                        // a + x -> x + a (a is literal)
                        let swap = match &*binary.left {
                            &Expr::Literal(_) => true,
                            _ => false,
                        };
                        if swap {
                            if let Expr::Literal(literal) = *binary.left {
                                binary.left = Box::new(*binary.right);
                                binary.right = Box::new(Expr::Literal(literal));

                                return (Expr::Binary(binary), true);
                            };
                        }
                        
                        (Expr::Binary(binary), modified)
                    }

                    // x - x -> 0 DONE
                    // 0 - x -> -x DONE
                    // x - 0 -> x DONE
                    // x - (x + y) -> -y
                    // (x + y) - x -> y
                    // x - (x - y) -> y
                    // x - (y - x) -> x*2 - y
                    // (x - y) - x -> -y
                    // (y - x) - x -> y - x*2
                    // x - y * x -> x * (1 - y)
                    // y * x - x -> x * (y - 1)
                    // x - x/y -> x * (1 - 1/y)
                    // x/y - x -> x * (1/y - 1)
                    // a * x - b * x -> x * (a - b)
                    // a * x - x -> x * (a - 1)
                    // x - (-y) -> x + y
                    BinaryOp::Sub => {
                        // x - x -> 0
                        if Self::expr_equal(&binary.left, &binary.right) {
                            return (Expr::Literal(Literal::Int(0)), true);
                        }

                        match (&*binary.left, &*binary.right) {
                            // 0 - x
                            (&Expr::Literal(Literal::Int(0)), _) => return (Expr::Unary(UnaryExpr {
                                expr: binary.right,
                                op: UnaryOp::Neg,
                                op_span: Span{line: 0, col: 0}, // span no longer matters
                                expr_type: binary.expr_type,
                            }), true),

                            // 0.0 - x
                            (&Expr::Literal(Literal::Real(0.0)), _) => return (Expr::Unary(UnaryExpr {
                                expr: binary.right,
                                op: UnaryOp::Neg,
                                op_span: Span{line: 0, col: 0}, // span no longer matters
                                expr_type: binary.expr_type,
                            }), true),

                            // x - 0
                            (_, &Expr::Literal(Literal::Int(0))) => return (*binary.left, true),

                            // x - 0.0
                            (_, &Expr::Literal(Literal::Real(0.0))) => return (*binary.left, true),

                            _ => {},
                        }

                        (Expr::Binary(binary), modified)
                    }

                    // x * 1 -> x DONE
                    // x * 0 -> 0 DONE
                    // (-x) * (-y) -> x * y
                    // x * (-y) -> -(x * y)
                    // a * (x * b) -> x * (a * b) (a, b are literal)
                    // a * x -> x * a (a is literal)
                    BinaryOp::Mul => {
                        match (&*binary.left, &*binary.right) {
                            // 0 * x
                            (&Expr::Literal(Literal::Int(0)), _) => return (Expr::Literal(Literal::Int(0)), true),

                            // 0.0 * x
                            (&Expr::Literal(Literal::Real(0.0)), _) => return (Expr::Literal(Literal::Int(0)), true),

                            // x * 0
                            (_, &Expr::Literal(Literal::Int(0))) => return (Expr::Literal(Literal::Int(0)), true),

                            // x * 0.0
                            (_, &Expr::Literal(Literal::Real(0.0))) => return (Expr::Literal(Literal::Int(0)), true),

                            // 1 * x
                            (&Expr::Literal(Literal::Int(1)), _) => return (*binary.right, true),

                            // 1.0 * x
                            (&Expr::Literal(Literal::Real(1.0)), _) => return (*binary.right, true),

                            // x * 1
                            (_, &Expr::Literal(Literal::Int(1))) => return (*binary.left, true),

                            // x * 1.0
                            (_, &Expr::Literal(Literal::Real(1.0))) => return (*binary.left, true),

                            _ => {},
                        }

                        (Expr::Binary(binary), modified)
                    }

                    // 0 / x -> 0 DONE
                    // x / 1 -> x DONE
                    // x / x -> 1 DONE
                    // (-x) / (-y) -> x / y
                    // x / (-y) -> -(x / y)
                    // (-x) / y -> -(x / y)
                    BinaryOp::Div => {
                        match (&*binary.left, &*binary.right) {
                            // 0 / x
                            (&Expr::Literal(Literal::Int(0)), _) => return (Expr::Literal(Literal::Int(0)), true),

                            // 0.0 / x
                            (&Expr::Literal(Literal::Real(0.0)), _) => return (Expr::Literal(Literal::Int(0)), true),

                            // x / 1
                            (_, &Expr::Literal(Literal::Int(1))) => return (*binary.left, true),

                            // x / 1.0
                            (_, &Expr::Literal(Literal::Real(1.0))) => return (*binary.left, true),

                            _ => {},
                        }

                        // x / x -> 1
                        if Self::expr_equal(&binary.left, &binary.right) {
                            return (Expr::Literal(Literal::Int(1)), true);
                        }

                        (Expr::Binary(binary), modified)
                    }

                    // 1 ^ x -> 1 DONE
                    // x ^ 1 -> x DONE
                    // x ^ 0 -> 1 DONE
                    BinaryOp::Pow => {
                        match (&*binary.left, &*binary.right) {
                            // x ^ 0
                            (_, &Expr::Literal(Literal::Int(0))) => return (Expr::Literal(Literal::Int(1)), true),

                            // x ^ 0.0
                            (_, &Expr::Literal(Literal::Real(0.0))) => return (Expr::Literal(Literal::Int(1)), true),

                            // 1 ^ x
                            (&Expr::Literal(Literal::Int(1)), _) => return (Expr::Literal(Literal::Int(1)), true),

                            // 1.0 ^ x
                            (&Expr::Literal(Literal::Real(1.0)), _) => return (Expr::Literal(Literal::Int(1)), true),

                            // x ^ 1
                            (_, &Expr::Literal(Literal::Int(1))) => return (*binary.left, true),

                            // x ^ 1.0
                            (_, &Expr::Literal(Literal::Real(1.0))) => return (*binary.left, true),

                            _ => {},
                        }

                        (Expr::Binary(binary), modified)
                    }

                    // x & x -> x DONE
                    // x & ~x -> false
                    // (~x) & (~y) -> ~(x | y)
                    // x & true -> x DONE
                    // x & false -> false DONE
                    BinaryOp::And => {
                        // x & x -> x
                        if Self::expr_equal(&binary.left, &binary.right) {
                            return (*binary.left, true);
                        }

                        match (&*binary.left, &*binary.right) {
                            // true & x
                            (&Expr::Literal(Literal::Bool(true)), _) => return (*binary.right, true),

                            // x & true
                            (_, &Expr::Literal(Literal::Bool(true))) => return (*binary.left, true),
                            
                            // false & x
                            (&Expr::Literal(Literal::Bool(false)), _) => return (Expr::Literal(Literal::Bool(false)), true),

                            // x & true
                            (_, &Expr::Literal(Literal::Bool(false))) => return (Expr::Literal(Literal::Bool(false)), true),

                            _ => {},
                        }

                        (Expr::Binary(binary), modified)
                    }

                    // x | x -> x DONE
                    // x | ~x -> true
                    // (~x) | (~y) -> ~(x & y)
                    // x | true -> true DONE
                    // x | false -> x DONE
                    BinaryOp::Or => {
                        // x | x -> x
                        if Self::expr_equal(&binary.left, &binary.right) {
                            return (*binary.left, true);
                        }

                        match (&*binary.left, &*binary.right) {
                            // true | x
                            (&Expr::Literal(Literal::Bool(true)), _) => return (Expr::Literal(Literal::Bool(true)), true),

                            // x | true
                            (_, &Expr::Literal(Literal::Bool(true))) => return (Expr::Literal(Literal::Bool(true)), true),
                            
                            // false | x
                            (&Expr::Literal(Literal::Bool(false)), _) => return (*binary.right, true),

                            // x | true
                            (_, &Expr::Literal(Literal::Bool(false))) => return (*binary.left, true),

                            _ => {},
                        }

                        (Expr::Binary(binary), modified)
                    }

                    // x xor true -> ~x
                    // x xor false -> x
                    // BinaryOp::Xor

                    // x == x -> true
                    // BinaryOp::Eq

                    // x != x -> false
                    // BinaryOp::Ne

                    _ => (Expr::Binary(binary), modified),
                }
            }

            // DONE
            Expr::Tuple(mut tuple) => {
                let mut modified = false; 
                for expr in &mut tuple {
                    let owned_expr = std::mem::replace(expr, Expr::Error);
                    let (transformed_expr, this_modified) = Self::algebraic_transform(owned_expr);

                    *expr = transformed_expr;

                    if this_modified {
                        modified = true;
                    }
                }

                (Expr::Tuple(tuple), modified)
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

            other => (other, false),
        }
    }

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
    fn equal_binary_1() {
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
    fn equal_binary_2() {
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

        let (result, _modified) = CodeGen::algebraic_transform(expr);

        assert_eq!(result, Expr::Ident(Ident::Symbol(0)));
    }

    #[test]
    fn binary_add_1() {
        // x + 0
        let expr = Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Ident(Ident::Symbol(0))),
            right: Box::new(Expr::Literal(Literal::Int(0))),
            op: BinaryOp::Add,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Int,
        });

        let (result, _modified) = CodeGen::algebraic_transform(expr);

        assert_eq!(result, Expr::Ident(Ident::Symbol(0)));
    }

    #[test]
    fn binary_add_2() {
        // x + (-x)
        let expr = Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Ident(Ident::Symbol(0))),
            right: Box::new(Expr::Unary(UnaryExpr {
                expr: Box::new(Expr::Ident(Ident::Symbol(0))),
                op: UnaryOp::Neg,
                op_span: Span{line: 0, col: 0},
                expr_type: Type::Int,
            })),
            op: BinaryOp::Add,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Int,
        });

        let (result, _modified) = CodeGen::algebraic_transform(expr);

        assert_eq!(result, Expr::Literal(Literal::Int(0)));
    }

    #[test]
    // x + (-y)
    fn binary_add_3() {
        let expr = Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Ident(Ident::Symbol(0))),
            right: Box::new(Expr::Unary(UnaryExpr {
                expr: Box::new(Expr::Ident(Ident::Symbol(1))),
                op: UnaryOp::Neg,
                op_span: Span{line: 0, col: 0},
                expr_type: Type::Int,
            })),
            op: BinaryOp::Add,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Int,
        });

        let (result, _modified) = CodeGen::algebraic_transform(expr);

        assert_eq!(result, Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Ident(Ident::Symbol(0))),
            right: Box::new(Expr::Ident(Ident::Symbol(1))),
            op: BinaryOp::Sub,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Int,
        }));
    }

    #[test]
    // (-x) + y -> y - x
    fn binary_add_4() {
        let expr = Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Unary(UnaryExpr {
                expr: Box::new(Expr::Ident(Ident::Symbol(1))),
                op: UnaryOp::Neg,
                op_span: Span{line: 0, col: 0},
                expr_type: Type::Int,
            })),
            right: Box::new(Expr::Ident(Ident::Symbol(0))),
            op: BinaryOp::Add,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Int,
        });

        let (result, _modified) = CodeGen::algebraic_transform(expr);

        assert_eq!(result, Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Ident(Ident::Symbol(0))),
            right: Box::new(Expr::Ident(Ident::Symbol(1))),
            op: BinaryOp::Sub,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Int,
        }));
    }

    #[test]
    // x + x -> x * 2
    fn binary_add_5() {
        let expr = Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Ident(Ident::Symbol(0))),
            right: Box::new(Expr::Ident(Ident::Symbol(0))),
            op: BinaryOp::Add,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Real,
        });

        let (result, _modified) = CodeGen::algebraic_transform(expr);

        assert_eq!(result, Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Ident(Ident::Symbol(0))),
            right: Box::new(Expr::Literal(Literal::Int(2))),
            op: BinaryOp::Mul,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Real,
        }));
    }

    // #[test]
    // // x + y * x -> x * (y + 1)
    // fn binary_add_6() {
    //     let expr = Expr::Binary(BinaryExpr {
    //         left: Box::new(Expr::Ident(Ident::Symbol(0))),
    //         right: Box::new(Expr::Binary(BinaryExpr {
    //             left: Box::new(Expr::Ident(Ident::Symbol(1))),
    //             right: Box::new(Expr::Ident(Ident::Symbol(0))),
    //             op: BinaryOp::Mul,
    //             op_span: Span{line: 0, col: 0},
    //             expr_type: Type::Int,
    //         })),
    //         op: BinaryOp::Add,
    //         op_span: Span{line: 0, col: 0},
    //         expr_type: Type::Int,
    //     });

    //     let result = CodeGen::algebraic_transform(expr);

    //     assert_eq!(result, Expr::Binary(BinaryExpr {
    //         left: Box::new(Expr::Ident(Ident::Symbol(0))),
    //         right: Box::new(Expr::Binary(BinaryExpr {
    //             left: Box::new(Expr::Ident(Ident::Symbol(0))),
    //             right: Box::new(Expr::Literal(Literal::Int(1))),
    //             op: BinaryOp::Add,
    //             op_span: Span{line: 0, col: 0},
    //             expr_type: Type::Int,
    //         })),
    //         op: BinaryOp::Mul,
    //         op_span: Span{line: 0, col: 0},
    //         expr_type: Type::Int,
    //     }));
    // }

    // #[test]
    // // y * x + x-> x * (y + 1)
    // fn binary_add_7() {
    //     let expr = Expr::Binary(BinaryExpr {
    //         left: Box::new(Expr::Binary(BinaryExpr {
    //             left: Box::new(Expr::Ident(Ident::Symbol(1))),
    //             right: Box::new(Expr::Ident(Ident::Symbol(0))),
    //             op: BinaryOp::Mul,
    //             op_span: Span{line: 0, col: 0},
    //             expr_type: Type::Int,
    //         })),
    //         right: Box::new(Expr::Ident(Ident::Symbol(0))),
    //         op: BinaryOp::Add,
    //         op_span: Span{line: 0, col: 0},
    //         expr_type: Type::Int,
    //     });

    //     let result = CodeGen::algebraic_transform(expr);

    //     assert_eq!(result, Expr::Binary(BinaryExpr {
    //         left: Box::new(Expr::Ident(Ident::Symbol(0))),
    //         right: Box::new(Expr::Binary(BinaryExpr {
    //             left: Box::new(Expr::Ident(Ident::Symbol(0))),
    //             right: Box::new(Expr::Literal(Literal::Int(1))),
    //             op: BinaryOp::Add,
    //             op_span: Span{line: 0, col: 0},
    //             expr_type: Type::Int,
    //         })),
    //         op: BinaryOp::Mul,
    //         op_span: Span{line: 0, col: 0},
    //         expr_type: Type::Int,
    //     }));
    // }

    #[test]
    // 1 + x -> x + 1
    fn binary_add_8() {
        let expr = Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Literal(Literal::Int(1))),
            right: Box::new(Expr::Ident(Ident::Symbol(0))),
            op: BinaryOp::Add,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Int,
        });

        let (result, _modified) = CodeGen::algebraic_transform(expr);

        assert_eq!(result, Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Ident(Ident::Symbol(0))),
            right: Box::new(Expr::Literal(Literal::Int(1))),
            op: BinaryOp::Add,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Int,
        }));
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

        let (result, _modified) = CodeGen::algebraic_transform(expr);

        assert_eq!(result, Expr::Ident(Ident::Symbol(0)));
    }

    #[test]
    fn binary_sub_2() {
        // 3*x - x*3
        let expr = Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Literal(Literal::Int(3))),
                right: Box::new(Expr::Ident(Ident::Symbol(0))),
                op: BinaryOp::Mul,
                op_span: Span{line: 0, col: 0},
                expr_type: Type::Int,
            })),
            right: Box::new(Expr::Binary(BinaryExpr {
                right: Box::new(Expr::Ident(Ident::Symbol(0))),
                left: Box::new(Expr::Literal(Literal::Int(3))),
                op: BinaryOp::Mul,
                op_span: Span{line: 0, col: 0},
                expr_type: Type::Int,
            })),
            op: BinaryOp::Sub,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Int,
        });

        let (result, _modified) = CodeGen::algebraic_transform(expr);

        assert_eq!(result, Expr::Literal(Literal::Int(0)));
    }

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

        let (result, _modified) = CodeGen::algebraic_transform(expr);

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

        let (result, _modified) = CodeGen::algebraic_transform(expr);

        assert_eq!(result, Expr::Literal(Literal::Int(0)));
    }

    // TODO: Test boolean operations
}