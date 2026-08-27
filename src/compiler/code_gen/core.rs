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
        // after optimization, the last step should be to turn x^2 back to x*x and 2x back to x+x
        // also turn (-x) + a (a is literal) back to a - x
        Vec::new()
    }

    fn optimize(&mut self) {
        for relation in &mut self.relations {
            let mut owned_expr = std::mem::replace(&mut relation.body, Expr::Error);

            loop {
                let (transformed_expr, modified) = Self::algebraic_transform(owned_expr);
                owned_expr = transformed_expr;

                if !modified {
                    break;
                }

                // TODO: Fold expression
            }
            
            relation.body = owned_expr;
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