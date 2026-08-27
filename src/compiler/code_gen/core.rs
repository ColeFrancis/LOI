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
//! Handles the compilation of relations into bytecode
//!
//! ## Invariants
//!
//! - 
//!
//! Author: Cole Francis

use super::CodeGen;
use crate::compiler::ast::*;
use crate::compiler::ast::RelType;
use crate::compiler::compiled_rel::CompiledRel;

impl CodeGen {
    pub fn new(relations: Vec<RelType>) -> Self {
        Self {
            relations,
        }
    }

    pub fn compile(mut self) -> Vec<CompiledRel> {
        let mut compiled_relations = Vec::new();
        for relation in self.relations {
            let optimized_expr = Self::optimize_expr(relation.body);
        }
        
        compiled_relations
    }

    fn optimize_expr(mut expr: Expr) -> Expr {
        loop {
            let (transformed_expr, modified) = Self::algebraic_transform(expr);
            expr = transformed_expr;

            // TODO: Fold expression?

            if !modified {
                // TODO: Convert x^2 -> x*x, 2x -> x+x, (-x) + a -> a - x

                return expr;
            }
        }  
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::sem_analyzer::types::Type;
    use crate::compiler::diagnostics::Span;

    // #[test]
    // fn optimize_1() {
    //     // (x * (1 - 1) + 1) - 5
    //     let expr = Expr::Binary(BinaryExpr {
    //         left: Box::new(Expr::Binary(BinaryExpr {
    //             left: Box::new(Expr::Binary(BinaryExpr {
    //                 left: Box::new(Expr::Ident(Ident::Symbol(0))),
    //                 right: Box::new(Expr::Binary(BinaryExpr {
    //                     left: Box::new(Expr::Literal(Literal::Int(1))),
    //                     right: Box::new(Expr::Literal(Literal::Int(1))),
    //                     op: BinaryOp::Sub,
    //                     op_span: Span{line: 0, col: 0},
    //                     expr_type: Type::Int,
    //                 })),
    //                 op: BinaryOp::Mul,
    //                 op_span: Span{line: 0, col: 0},
    //                 expr_type: Type::Int,
    //             })),
    //             right: Box::new(Expr::Literal(Literal::Int(1))),
    //             op: BinaryOp::Add,
    //             op_span: Span{line: 0, col: 0},
    //             expr_type: Type::Int,
    //         })),
    //         right: Box::new(Expr::Literal(Literal::Int(5))),
    //         op: BinaryOp::Sub,
    //         op_span: Span{line: 0, col: 0},
    //         expr_type: Type::Int,
    //     });
        

    //     let (result, _modified) = CodeGen::algebraic_transform(expr);

    //     assert_eq!(result, Expr::Literal(Literal::Int(-4)));
    // }
}