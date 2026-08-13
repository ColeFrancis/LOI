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

//! # check_expr
//!
//! Handles checking and assigning of types for annotated ast
//!
//! ## Invariants
//!
//! - All incoming expressions will have Type::Unknown
//! - All symbols have been resolved to 
//!
//! Author: Cole Francis

use super::SemAnalyzer;
use super::types::Type;
use super::symbol::SymbolKind;
use crate::compiler::parser::ast::*;
use crate::compiler::diagnostics::{CompilerError, Operation, Span};

impl <'a> SemAnalyzer<'a> {
    // Verify typers/operators and set expr_types
    // Recursively call to set all sub-expression types, then verify types with get_expr_type
    pub(super) fn add_types_expr(&mut self, mut expr: Expr) -> Option<Expr> {
        match expr {
            Expr::Literal(literal) => Some(Expr::Literal(literal)),

            Expr::Ident(ident) => Some(Expr::Ident(ident)),

            Expr::Unary(mut unary_expr) => {
                unary_expr.expr = Box::new(self.add_types_expr(*unary_expr.expr)
                    .unwrap_or(Expr::Error));

                unary_expr.expr_type = self.get_expr_type(&unary_expr.expr);

                self.verify_unary_op_type_match(&unary_expr.expr_type, &unary_expr.op, &unary_expr.op_span);

                Some(Expr::Unary(unary_expr))
            }

            // Expr::Binary(binary_expr) => {}

            // Expr::Tuple(tuple_expr) => {}

            // Expr::Block(block_expr) => {}

            // Expr::Match(match_expr) => {}

            // Expr::Sample(sample_expr) => {}

            // Expr::Error => Some(Expr::Error)

            _ => Some(Expr::Error)
        }
    }

    fn get_expr_type(&self, expr: &Expr) -> Type {
        match expr {
            Expr::Literal(literal) => match literal {
                Literal::Bool(_) => Type::Bool,

                Literal::Int(_) => Type::Int,

                Literal::Real(_) => Type::Real,
            }

            Expr::Ident(ident) => match ident {
                Ident::Symbol(symbol_id) => match &self.symbols[*symbol_id].kind {
                    SymbolKind::Variable(ty) => ty.clone(),

                    _ => Type::Error, // Should not be reachable
                }
                
                Ident::Str{..} => Type::Error, // Should not be reachable
            }

            Expr::Unary(unary_expr) => unary_expr.expr_type.clone(),

            Expr::Binary(binary_expr) => binary_expr.expr_type.clone(),

            Expr::Tuple(tuple_expr) => {
                let mut types: Vec<Type> = Vec::new();

                for expr in tuple_expr {
                    types.push(self.get_expr_type(expr));
                }

                Type::Tuple(types)
            }

            Expr::Block(block_expr) => block_expr.expr_type.clone(),

            Expr::Match(match_expr) => match_expr.expr_type.clone(),

            Expr::Sample(sample_expr) => sample_expr.expr_type.clone(),

            Expr::Error => Type::Error,
        }
    }

    fn verify_unary_op_type_match(&mut self, expr_type: &Type, op: &UnaryOp, op_span: &Span) -> Option<()> {
        match (expr_type, op) {
            (Type::Bool,    UnaryOp::BitNot) => Some(()),
            (Type::Impulse, UnaryOp::BitNot) => Some(()),

            (Type::Int,     UnaryOp::Neg) => Some(()),
            (Type::Real,    UnaryOp::Neg) => Some(()),
            (Type::Mod(_),  UnaryOp::Neg) => Some(()),

            _ => {
                let diagnostics_op = match op {
                    UnaryOp::BitNot => Operation::Not,
                    UnaryOp::Neg => Operation::Sub,
                };

                self.diagnostics.error(CompilerError::IncompatibleOp {
                    expr_type: expr_type.clone(),
                    op: diagnostics_op,
                    op_span: op_span.clone(),
                });

                None
            },
        }
    }

    // fn verify_binary_expr_type_match(&mut self, left: &Type, right: &Type) -> Option<Type> {}

    // fn verify_binary_op_type_match(&mut self, expr_type: &Type, op: &BinaryOp, op_span: &Span) -> Option<()> {}


}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::compiler::sem_analyzer::scope::Scope;
    use crate::compiler::sem_analyzer::symbol::Symbol;
    use crate::compiler::diagnostics::Diagnostics;

    #[test]
    fn get_expr_type() {
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    id: 0,
                    name: "a".to_string(),
                    kind: SymbolKind::Variable(Type::Int),
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

            diagnostics: &mut diagnostics,
        };

        // (1+1, true, 2.0, a) // a already in symbol table as an Int
        let result = sem_analyzer.get_expr_type(&Expr::Tuple(vec![
            Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Literal(Literal::Int(1))),
                op: BinaryOp::Add,
                right: Box::new(Expr::Literal(Literal::Int(1))),
                op_span: Span {line: 0, col: 0},
                expr_type: Type::Int,
            }),
            Expr::Literal(Literal::Bool(true)),
            Expr::Literal(Literal::Real(2.0)),
            Expr::Ident(Ident::Symbol(0)),
        ]));

        assert_eq!(result, Type::Tuple(vec![
            Type::Int,
            Type::Bool,
            Type::Real,
            Type::Int,
        ]));
    }
}