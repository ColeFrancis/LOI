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
use crate::compiler::{
    symbol::SymbolKind,
    ast::*,
    diagnostics::{CompilerError, Operation, Span, ExprType},
};

impl <'a> SemAnalyzer<'a> {
    // Verify typers/operators and set expr_types
    // Recursively call to set all sub-expression types, then verify types with get_expr_type
    pub(super) fn add_types_expr(&mut self, expr: Expr) -> Option<Expr> {
        match expr {
            Expr::Literal(literal) => Some(Expr::Literal(literal)),

            Expr::Ident(ident) => Some(Expr::Ident(ident)),

            Expr::Unary(mut unary_expr) => {
                unary_expr.expr = Box::new(self.add_types_expr(*unary_expr.expr)
                    .unwrap_or(Expr::Error));

                unary_expr.expr_type = self.get_expr_type(&unary_expr.expr);
                // Note: Performing any boolean operation on an Impulse type variable converts it automatically to a Bool.
                if unary_expr.expr_type == Type::Impulse {
                    unary_expr.expr_type = Type::Bool;
                }

                self.verify_unary_op_type_match(&unary_expr.expr_type, &unary_expr.op, &unary_expr.op_span)?;

                Some(Expr::Unary(unary_expr))
            }

            Expr::Binary(mut binary_expr) => {
                binary_expr.left = Box::new(self.add_types_expr(*binary_expr.left)
                    .unwrap_or(Expr::Error));
                binary_expr.right = Box::new(self.add_types_expr(*binary_expr.right)
                    .unwrap_or(Expr::Error));

                let left_expr_type = self.get_expr_type(&binary_expr.left);
                let right_expr_type = self.get_expr_type(&binary_expr.right);

                binary_expr.expr_type = self.verify_expr_type_match(&left_expr_type, &right_expr_type, &binary_expr.op_span)?;
                // Note: Performing any boolean operation on an Impulse type variable converts it automatically to a Bool.
                if binary_expr.expr_type == Type::Impulse {
                    binary_expr.expr_type = Type::Bool;
                }

                binary_expr.expr_type = self.verify_binary_op_type_match(&binary_expr.expr_type, &binary_expr.op, &binary_expr.op_span)?;

                Some(Expr::Binary(binary_expr))
            } 

            Expr::Tuple(mut tuple_expr) => {
                for expr in &mut tuple_expr {
                    let owned_expr = std::mem::replace(expr, Expr::Error);

                    *expr = self.add_types_expr(owned_expr).unwrap_or(Expr::Error);
                }

                Some(Expr::Tuple(tuple_expr))
            }

            Expr::Block(mut block_expr) => {
                for statement in &mut block_expr.statements {
                    let owned_statement = std::mem::replace(statement, Statement::Error);

                    *statement = match owned_statement {
                        Statement::Let(let_statement) => Statement::Let(self.check_let(let_statement)),

                        Statement::Error => Statement::Error,
                    };
                }

                block_expr.expr = Box::new(self.add_types_expr(*block_expr.expr).unwrap_or(Expr::Error));

                block_expr.expr_type = self.get_expr_type(&block_expr.expr);

                Some(Expr::Block(block_expr))
            }

            Expr::Cases(mut cases_expr) => {
                let mut has_errors = false;
                let mut expr_type = self.get_expr_type(&cases_expr.arms[0].expr);

                for arm in &cases_expr.arms {
                    if self.verify_pattern_expr_match(&cases_expr.scrutinee, &arm.pattern, &cases_expr.span, &arm.arm_span).is_none() {
                        has_errors = true;
                    }

                    let curr_expr_type = self.get_expr_type(&arm.expr);

                    expr_type = match self.verify_expr_type_match(&expr_type, &curr_expr_type, &cases_expr.span) {
                        Some(expr_type) => expr_type,
                        None => {
                            has_errors = true;
                            continue;
                        }
                    }
                }
                
                cases_expr.expr_type = expr_type;

                if has_errors {
                    None
                }
                else {
                    Some(Expr::Cases(cases_expr))
                }
            }

            Expr::Sample(mut sample_expr) => {
                let mut has_errors = false;
                let mut expr_type = self.get_expr_type(&sample_expr.arms[0].expr);

                for arm in &sample_expr.arms {
                    let curr_expr_type = self.get_expr_type(&arm.expr);

                    expr_type = match self.verify_expr_type_match(&expr_type, &curr_expr_type, &sample_expr.span) {
                        Some(expr_type) => expr_type,
                        None => {
                            has_errors = true;
                            continue;
                        }
                    };

                    if let Prob::Expr(expr) = &arm.prob {
                        let prob_type = self.get_expr_type(&expr);

                        if prob_type != Type::Int && prob_type != Type::Real {
                            self.diagnostics.error(CompilerError::NonRealProb {
                                prob_type,
                                arm_span: arm.arm_span.clone(),
                            });

                            has_errors = true;
                        }
                    };
                }
                
                sample_expr.expr_type = expr_type;

                if has_errors {
                    None
                }
                else {
                    Some(Expr::Sample(sample_expr))
                }
            }

            Expr::Error => Some(Expr::Error),
        }
    }

    pub(super) fn get_expr_type(&self, expr: &Expr) -> Type {
        match expr {
            Expr::Literal(literal) => match literal {
                Literal::Bool(_) => Type::Bool,

                Literal::Int(_) => Type::Int,

                Literal::Real(_) => Type::Real,
            }

            Expr::Ident(ident) => match ident {
                Ident::Symbol(symbol_id) => match &self.symbols[*symbol_id].kind {
                    SymbolKind::Variable(ty) => ty.clone(),

                    SymbolKind::EntMember { parent, ..} => Type::Custom(Ident::Symbol(*parent)),

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

            Expr::Cases(cases_expr) => cases_expr.expr_type.clone(),

            Expr::Sample(sample_expr) => sample_expr.expr_type.clone(),

            Expr::Error => Type::Error,
        }
    }

    fn get_simple_pattern_type(&self, simple_pattern: &SimplePattern) -> Type {
        match simple_pattern {
            SimplePattern::Default => Type::Unknown, // unreachable

            SimplePattern::Literal(literal) => match literal {
                Literal::Bool(_) => Type::Bool,

                Literal::Int(_) => Type::Int,

                Literal::Real(_) => Type::Real,
            }

            SimplePattern::Ident(ident) => match ident {
                Ident::Symbol(symbol_id) => match &self.symbols[*symbol_id].kind {
                    SymbolKind::Variable(ty) => ty.clone(),

                    SymbolKind::EntMember {parent, ..} => Type::Custom(Ident::Symbol(*parent)),

                    _ => Type::Error, // Should not be reachable
                }
                
                Ident::Str{..} => Type::Error, // Should not be reachable
            }

            SimplePattern::Tuple(tuple_pattern) => {
                let mut types: Vec<Type> = Vec::new();

                for pattern in tuple_pattern {
                    types.push(self.get_simple_pattern_type(pattern));
                }

                Type::Tuple(types)
            }

            SimplePattern::Comparison(_) => Type::Int, // Int can be converted to Real.

            SimplePattern::Error => Type::Error,
        }
    }

    fn verify_unary_op_type_match(&mut self, expr_type: &Type, op: &UnaryOp, op_span: &Span) -> Option<()> {
        match (expr_type, op) {
            (Type::Bool,    UnaryOp::BitNot) => Some(()),
            (Type::Impulse, UnaryOp::BitNot) => Some(()),

            (Type::Int,     UnaryOp::Neg) => Some(()),
            (Type::Real,    UnaryOp::Neg) => Some(()),
            (Type::Mod(_),  UnaryOp::Neg) => Some(()),

            (Type::Error, _) => None,

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

    fn verify_expr_type_match(&mut self, left: &Type, right: &Type, op_span: &Span) -> Option<Type> {
        match (left, right) {
            (Type::Impulse, Type::Impulse) => Some(Type::Impulse),
            (Type::Bool,    Type::Impulse) => Some(Type::Bool),
            (Type::Impulse, Type::Bool   ) => Some(Type::Bool),
            (Type::Bool,    Type::Bool   ) => Some(Type::Bool),

            (Type::Mod(val_left), Type::Mod(val_right)) => {
                if val_left == val_right {
                    Some(Type::Mod(*val_left))
                }
                else {
                    // Cannot combine mod types that are different
                    self.diagnostics.error(CompilerError::IncompatibleTypes {
                        left: left.clone(),
                        right: right.clone(),
                        op_span: op_span.clone(),
                    });
                    
                    None
                }
            } 
            (Type::Mod(_), Type::Int   ) => Some(Type::Int),
            (Type::Int,    Type::Mod(_)) => Some(Type::Int),
            (Type::Int,    Type::Int   ) => Some(Type::Int),
            (Type::Mod(_), Type::Real  ) => Some(Type::Real),
            (Type::Real,   Type::Mod(_)) => Some(Type::Real),
            (Type::Int,    Type::Real  ) => Some(Type::Real),
            (Type::Real,   Type::Int   ) => Some(Type::Real),
            (Type::Real,   Type::Real  ) => Some(Type::Real),

            (Type::Custom(ident_l), Type::Custom(ident_r)) => {
                let (parent_l, parent_r) = match (ident_l, ident_r) {
                    (Ident::Symbol(id_l), Ident::Symbol(id_r)) => (*id_l, *id_r),
                    
                    _ => return None, // Unreachable after name resolution
                };

                if parent_l == parent_r {
                    Some(Type::Custom(Ident::Symbol(parent_l)))
                }
                else {
                    self.diagnostics.error(CompilerError::IncompatibleTypes {
                        left: Type::Custom(Ident::Symbol(parent_l)),
                        right: Type::Custom(Ident::Symbol(parent_r)),
                        op_span: op_span.clone(),
                    });

                    None
                }
                
            }

            (Type::Error, _) => None,
            (_, Type::Error) => None,

            _ => {
                self.diagnostics.error(CompilerError::IncompatibleTypes {
                    left: left.clone(),
                    right: right.clone(),
                    op_span: op_span.clone(),
                });

                None
            }
        }
    }

    fn verify_binary_op_type_match(&mut self, expr_type: &Type, op: &BinaryOp, op_span: &Span) -> Option<Type> {
        match (expr_type, op) {
            (Type::Impulse,  BinaryOp::Or ) => Some(Type::Impulse),
            (Type::Impulse,  BinaryOp::And) => Some(Type::Impulse),

            (Type::Bool,     BinaryOp::Or ) => Some(Type::Bool),
            (Type::Bool,     BinaryOp::And) => Some(Type::Bool),

            (Type::Mod(_),   BinaryOp::Lt ) => Some(Type::Bool),
            (Type::Mod(_),   BinaryOp::Gt ) => Some(Type::Bool),
            (Type::Mod(_),   BinaryOp::Le ) => Some(Type::Bool),
            (Type::Mod(_),   BinaryOp::Ge ) => Some(Type::Bool),
            (Type::Mod(val), BinaryOp::Add) => Some(Type::Mod(*val)),
            (Type::Mod(val), BinaryOp::Sub) => Some(Type::Mod(*val)),
            (Type::Mod(val), BinaryOp::Mul) => Some(Type::Mod(*val)),
            (Type::Mod(val), BinaryOp::Div) => Some(Type::Mod(*val)),
            (Type::Mod(val), BinaryOp::Pow) => Some(Type::Mod(*val)),
            (Type::Int,      BinaryOp::Lt ) => Some(Type::Bool),
            (Type::Int,      BinaryOp::Gt ) => Some(Type::Bool),
            (Type::Int,      BinaryOp::Le ) => Some(Type::Bool),
            (Type::Int,      BinaryOp::Ge ) => Some(Type::Bool),
            (Type::Int,      BinaryOp::Add) => Some(Type::Int),
            (Type::Int,      BinaryOp::Sub) => Some(Type::Int),
            (Type::Int,      BinaryOp::Mul) => Some(Type::Int),
            (Type::Int,      BinaryOp::Div) => Some(Type::Int),
            (Type::Int,      BinaryOp::Pow) => Some(Type::Int),
            (Type::Real,     BinaryOp::Gt ) => Some(Type::Bool),
            (Type::Real,     BinaryOp::Lt ) => Some(Type::Bool),
            (Type::Real,     BinaryOp::Le ) => Some(Type::Bool),
            (Type::Real,     BinaryOp::Ge ) => Some(Type::Bool),
            (Type::Real,     BinaryOp::Add) => Some(Type::Real),
            (Type::Real,     BinaryOp::Sub) => Some(Type::Real),
            (Type::Real,     BinaryOp::Mul) => Some(Type::Real),
            (Type::Real,     BinaryOp::Div) => Some(Type::Real),
            (Type::Real,     BinaryOp::Pow) => Some(Type::Real),

            (Type::Error, _) => None,

            _ => {
                let diagnostics_op = match op {
                    BinaryOp::Lt  => Operation::Cmp,
                    BinaryOp::Gt  => Operation::Cmp,
                    BinaryOp::Le  => Operation::Cmp,
                    BinaryOp::Ge  => Operation::Cmp,
                    BinaryOp::Add => Operation::Add,
                    BinaryOp::Sub => Operation::Sub,
                    BinaryOp::Mul => Operation::Mul,
                    BinaryOp::Div => Operation::Div,
                    BinaryOp::Pow => Operation::Pow,
                    BinaryOp::Or  => Operation::Or,
                    BinaryOp::And => Operation::And,
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

    fn verify_pattern_expr_match(&mut self, scrutinee: &Expr, pattern: &[SimplePattern], cases_span: &Span, arm_span: &Span) -> Option<()> {
        let mut has_errors = false;
        
        for simple_pattern in pattern {
            match (scrutinee, simple_pattern) {
                (_, SimplePattern::Default) => {},

                (Expr::Tuple(tuple_expr), SimplePattern::Tuple(tuple_pattern)) => {
                    if tuple_expr.len() != tuple_pattern.len() {
                        self.diagnostics.error(CompilerError::UnequalTupleLength {
                            left_len: tuple_expr.len(),
                            right_len: tuple_pattern.len(),
                            right_span: arm_span.clone(),
                        });
                        
                        has_errors = true;
                        continue;
                    }

                    for (expr, pattern) in tuple_expr.iter().zip(tuple_pattern.iter()) {
                        let scrutinee_type = self.get_expr_type(expr);

                        if self.verify_simple_pattern_type_match(&scrutinee_type, pattern, arm_span).is_none() {
                            has_errors = true;
                        }
                    }
                }

                (Expr::Ident(_) | Expr::Unary(_) | Expr::Binary(_), simple_pattern) => {
                    let scrutinee_type = self.get_expr_type(scrutinee);

                    if self.verify_simple_pattern_type_match(&scrutinee_type, simple_pattern, arm_span).is_none() {
                        has_errors = true;
                    }
                }

                (Expr::Literal(_) | Expr::Block(_) | Expr::Cases(_) | Expr::Sample(_), _) =>  {
                    let found = match scrutinee {
                        Expr::Literal(_) => ExprType::Literal,
                        Expr::Block(_)   => ExprType::Block,
                        Expr::Cases(_)   => ExprType::Cases,
                        Expr::Sample(_)  => ExprType::Sample,
                        _ => ExprType::Error, //unreachable
                    };

                    self.diagnostics.error(CompilerError::IllegalScrutineeExpr {
                        expected: vec![
                            ExprType::Ident, 
                            ExprType::Unary, 
                            ExprType::Binary, 
                            ExprType::Tuple
                        ],
                        found,
                        cases_span: cases_span.clone(),
                    });

                    has_errors = true;
                }

                (_other_scrutinee, _other_pattern) => {
                    self.diagnostics.error(CompilerError::IncompatibleTypes {
                        left: self.get_expr_type(scrutinee),
                        right: self.get_simple_pattern_type(simple_pattern),
                        op_span: arm_span.clone(),
                    });

                    has_errors = true;
                }
            }
        }

        if has_errors {
            None
        }
        else {
            Some(())
        }
    }

    fn verify_simple_pattern_type_match(&mut self, scrutinee_type: &Type, simple_pattern: &SimplePattern, arm_span: &Span) -> Option<()> {
        let pattern_type = self.get_simple_pattern_type(simple_pattern);

        match (scrutinee_type, &pattern_type) {
            (Type::Impulse, Type::Impulse)
            | (Type::Impulse, Type::Bool   )
            | (Type::Bool,    Type::Bool   )
            | (Type::Bool,    Type::Impulse) => Some(()),

            (Type::Mod(val_left), Type::Mod(val_right)) => {
                if *val_left == *val_right {
                    Some(())
                }
                else {
                    // Cannot combine mod types that are different
                    self.diagnostics.error(CompilerError::IncompatibleTypes {
                        left: scrutinee_type.clone(),
                        right: pattern_type.clone(),
                        op_span: arm_span.clone(),
                    });
                    
                    None
                }
            } 
            (Type::Mod(_), Type::Int) => Some(()),
            (Type::Mod(_), Type::Real) => Some(()),
            (Type::Int,     Type::Mod(_)) => Some(()),
            (Type::Int,     Type::Int) => Some(()),
            (Type::Int,     Type::Real) => Some(()),
            (Type::Real,    Type::Mod(_)) => Some(()),
            (Type::Real,    Type::Int) => Some(()),
            (Type::Real,    Type::Real) => Some(()),

            (Type::Custom(ident_l), Type::Custom(ident_r)) => {
                let (parent_l, parent_r) = match (ident_l, ident_r) {
                    (Ident::Symbol(id_l), Ident::Symbol(id_r)) => (*id_l, *id_r),
                    
                    _ => return None, // Unreachable after name resolution
                };

                if parent_l == parent_r {
                    Some(())
                }
                else {
                    self.diagnostics.error(CompilerError::IncompatibleTypes {
                        left: Type::Custom(Ident::Symbol(parent_l)),
                        right: Type::Custom(Ident::Symbol(parent_r)),
                        op_span: arm_span.clone(),
                    });

                    None
                }
                
            }

            (Type::Error, _) => None,
            (_, Type::Error) => None,

            _ => {
                self.diagnostics.error(CompilerError::IncompatibleTypes {
                    left: scrutinee_type.clone(),
                    right: pattern_type.clone(),
                    op_span: arm_span.clone(),
                });

                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::compiler::sem_analyzer::scope::Scope;
    use crate::compiler::symbol::Symbol;
    use crate::compiler::diagnostics::Diagnostics;

    #[test]
    fn get_expr_type() {
        // (1+1, true, 2.0, a, H) // a already in symbol table as an Int, H as member of COIN
        let mut diagnostics = Diagnostics::new();
        let sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Variable(Type::Int),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "COIN".to_string(),
                    kind: SymbolKind::EntType,
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "H".to_string(),
                    kind: SymbolKind::EntMember {
                        parent: 1,
                        mapping: 0,
                    },
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "T".to_string(),
                    kind: SymbolKind::EntMember{
                        parent: 1,
                        mapping: 1,
                    },
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("a".to_string(), 0),
                        ("COIN".to_string(), 1),
                        ("H".to_string(), 2),
                        ("T".to_string(), 3),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

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
            Expr::Ident(Ident::Symbol(2)),
        ]));

        assert_eq!(result, Type::Tuple(vec![
            Type::Int,
            Type::Bool,
            Type::Real,
            Type::Int,
            Type::Custom(Ident::Symbol(1)),
        ]));
    }

    #[test]
    fn unary_expr_1() {
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer::new(
            Program {items: Vec::new()},
            &mut diagnostics,
        );

        let result = sem_analyzer.add_types_expr(Expr::Unary(UnaryExpr {
            op: UnaryOp::Neg,
            expr: Box::new(Expr::Unary(UnaryExpr {
                op: UnaryOp::Neg,
                expr: Box::new(Expr::Literal(Literal::Int(1))),
                op_span: Span {line:0, col: 0},
                expr_type: Type::Unknown,
            })),
            op_span: Span {line: 0, col: 0},
            expr_type: Type::Unknown,
        }));

        assert_eq!(result, Some(Expr::Unary(UnaryExpr {
            op: UnaryOp::Neg,
            expr: Box::new(Expr::Unary(UnaryExpr {
                op: UnaryOp::Neg,
                expr: Box::new(Expr::Literal(Literal::Int(1))),
                op_span: Span {line:0, col: 0},
                expr_type: Type::Int,
            })),
            op_span: Span {line: 0, col: 0},
            expr_type: Type::Int,
        })));
    }

    #[test]
    fn unary_expr_2() {
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer::new(
            Program {items: Vec::new()},
            &mut diagnostics,
        );

        let result = sem_analyzer.add_types_expr(Expr::Unary(UnaryExpr {
            op: UnaryOp::Neg,
            expr: Box::new(Expr::Unary(UnaryExpr {
                op: UnaryOp::Neg,
                expr: Box::new(Expr::Literal(Literal::Bool(true))),
                op_span: Span {line:0, col: 0},
                expr_type: Type::Unknown,
            })),
            op_span: Span {line: 0, col: 0},
            expr_type: Type::Unknown,
        }));

        diagnostics.debug_print();
        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn binary_expr_1() {
        // 1 + r // r is a Real
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "r".to_string(),
                    kind: SymbolKind::Variable(Type::Real),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("r".to_string(), 0),
                    ])
                },
            ],
            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.add_types_expr(Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Literal(Literal::Int(1))),
            right: Box::new(Expr::Ident(Ident::Symbol(0))),
            op: BinaryOp::Add,
            op_span: Span {line: 0, col: 0},
            expr_type: Type::Unknown,
        }));

        assert_eq!(result, Some(Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Literal(Literal::Int(1))),
            right: Box::new(Expr::Ident(Ident::Symbol(0))),
            op: BinaryOp::Add,
            op_span: Span {line: 0, col: 0},
            expr_type: Type::Real,
        })));
    }

    #[test]
    fn binary_expr_2() {
        // 1 + b // b is a bool
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "b".to_string(),
                    kind: SymbolKind::Variable(Type::Bool),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("b".to_string(), 0),
                    ])
                },
            ],
            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.add_types_expr(Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Literal(Literal::Int(1))),
            right: Box::new(Expr::Ident(Ident::Symbol(0))),
            op: BinaryOp::Add,
            op_span: Span {line: 0, col: 0},
            expr_type: Type::Unknown,
        }));
        
        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn binary_expr_3() {
        // true + b // b is a bool
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "b".to_string(),
                    kind: SymbolKind::Variable(Type::Bool),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("b".to_string(), 0),
                    ])
                },
            ],
            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.add_types_expr(Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Literal(Literal::Bool(true))),
            right: Box::new(Expr::Ident(Ident::Symbol(0))),
            op: BinaryOp::Add,
            op_span: Span {line: 0, col: 0},
            expr_type: Type::Unknown,
        }));
        
        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn binary_expr_4() {
        // 1 < r // r is a Real
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "r".to_string(),
                    kind: SymbolKind::Variable(Type::Real),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("r".to_string(), 0),
                    ])
                },
            ],
            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.add_types_expr(Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Literal(Literal::Int(1))),
            right: Box::new(Expr::Ident(Ident::Symbol(0))),
            op: BinaryOp::Lt,
            op_span: Span {line: 0, col: 0},
            expr_type: Type::Unknown,
        }));

        assert_eq!(result, Some(Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Literal(Literal::Int(1))),
            right: Box::new(Expr::Ident(Ident::Symbol(0))),
            op: BinaryOp::Lt,
            op_span: Span {line: 0, col: 0},
            expr_type: Type::Bool,
        })));
    }

    #[test]
    fn binary_expr_5() {
        // z2 + z3 // mod variables incompatible
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "z2".to_string(),
                    kind: SymbolKind::Variable(Type::Mod(2)),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "z3".to_string(),
                    kind: SymbolKind::Variable(Type::Mod(3)),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("z2".to_string(), 0),
                        ("z3".to_string(), 1),
                    ])
                },
            ],
            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.add_types_expr(Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Ident(Ident::Symbol(0))),
            right: Box::new(Expr::Ident(Ident::Symbol(1))),
            op: BinaryOp::Add,
            op_span: Span {line: 0, col: 0},
            expr_type: Type::Unknown,
        }));

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn binary_expr_6() {
        // z3 + z3
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "z2".to_string(),
                    kind: SymbolKind::Variable(Type::Mod(3)),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "z3".to_string(),
                    kind: SymbolKind::Variable(Type::Mod(3)),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("z2".to_string(), 0),
                        ("z3".to_string(), 1),
                    ])
                },
            ],
            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.add_types_expr(Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Ident(Ident::Symbol(0))),
            right: Box::new(Expr::Ident(Ident::Symbol(1))),
            op: BinaryOp::Add,
            op_span: Span {line: 0, col: 0},
            expr_type: Type::Unknown,
        }));

        assert_eq!(result, Some(Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Ident(Ident::Symbol(0))),
            right: Box::new(Expr::Ident(Ident::Symbol(1))),
            op: BinaryOp::Add,
            op_span: Span {line: 0, col: 0},
            expr_type: Type::Mod(3),
        })));
    }

    #[test]
    fn tuple_expr_1() {
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer::new(
            Program {items: Vec::new()},
            &mut diagnostics,
        );

        let result = sem_analyzer.add_types_expr(Expr::Tuple(vec![
            Expr::Unary(UnaryExpr {
                expr: Box::new(Expr::Literal(Literal::Int(3))),
                op: UnaryOp::Neg,
                op_span: Span {line: 0, col: 0},
                expr_type: Type::Unknown,
            }),
            Expr::Literal(Literal::Int(1)),
        ]));

        assert_eq!(result, Some(Expr::Tuple(vec![
            Expr::Unary(UnaryExpr {
                expr: Box::new(Expr::Literal(Literal::Int(3))),
                op: UnaryOp::Neg,
                op_span: Span {line: 0, col: 0},
                expr_type: Type::Int,
            }),
            Expr::Literal(Literal::Int(1)),
        ])));
    }

    #[test]
    fn block_expr_1() {
        // {let n = 1; n}
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "n".to_string(),
                    kind: SymbolKind::Variable(Type::Unknown),
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

        let result = sem_analyzer.add_types_expr(Expr::Block(BlockExpr {
            statements: vec![
                Statement::Let(LetStatement {
                    name: Ident::Symbol(0),
                    expr: Expr::Literal(Literal::Int(1)),
                }),
            ],
            expr: Box::new(Expr::Ident(Ident::Symbol(0))),
            expr_type: Type::Unknown,
        }));

        assert_eq!(result, Some(Expr::Block(BlockExpr {
            statements: vec![
                Statement::Let(LetStatement {
                    name: Ident::Symbol(0),
                    expr: Expr::Literal(Literal::Int(1)),
                }),
            ],
            expr: Box::new(Expr::Ident(Ident::Symbol(0))),
            expr_type: Type::Int,
        })));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                name: "n".to_string(),
                kind: SymbolKind::Variable(Type::Int),
                span: Span{line: 0, col: 0},
            },
        ]);
    }

    #[test]
    fn block_expr_2() {
        // {let n = 1+true; n} // 1 and true are incompatible types
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "n".to_string(),
                    kind: SymbolKind::Variable(Type::Unknown),
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

        let result = sem_analyzer.add_types_expr(Expr::Block(BlockExpr {
            statements: vec![
                Statement::Let(LetStatement {
                    name: Ident::Symbol(0),
                    expr: Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::Literal(Literal::Int(1))),
                        right: Box::new(Expr::Literal(Literal::Bool(true))),
                        op: BinaryOp::Add,
                        op_span: Span {line: 0, col: 0},
                        expr_type: Type::Unknown,
                    }),
                }),
            ],
            expr: Box::new(Expr::Ident(Ident::Symbol(0))),
            expr_type: Type::Unknown,
        }));

        assert_eq!(result, Some(Expr::Block(BlockExpr {
            statements: vec![
                Statement::Let(LetStatement {
                    name: Ident::Symbol(0),
                    expr: Expr::Error,
                }),
            ],
            expr: Box::new(Expr::Ident(Ident::Symbol(0))),
            expr_type: Type::Error,
        })));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                name: "n".to_string(),
                kind: SymbolKind::Variable(Type::Error),
                span: Span{line: 0, col: 0},
            },
        ]);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn cases_expr_1() {
        // cases (r, b) {
        //     (1, true) | (0, false) : true,
        //     _ : false,
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "r".to_string(),
                    kind: SymbolKind::Variable(Type::Real),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "b".to_string(),
                    kind: SymbolKind::Variable(Type::Bool),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("r".to_string(), 0),
                        ("b".to_string(), 1)
                    ])
                },
            ],
            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.add_types_expr(Expr::Cases(CasesExpr {
            scrutinee: Box::new(Expr::Tuple(vec![
                Expr::Ident(Ident::Symbol(0)),
                Expr::Ident(Ident::Symbol(1)),
            ])),
            arms: vec![
                CasesArm {
                    pattern: vec![
                        SimplePattern::Tuple(vec![
                            SimplePattern::Literal(Literal::Int(1)),
                            SimplePattern::Literal(Literal::Bool(true)),
                        ]),
                        SimplePattern::Tuple(vec![
                            SimplePattern::Literal(Literal::Int(0)),
                            SimplePattern::Literal(Literal::Bool(false)),
                        ]),
                    ],
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span {line: 0, col: 0},
                },
                CasesArm {
                    pattern: vec![SimplePattern::Default],
                    expr: Expr::Literal(Literal::Bool(false)),
                    arm_span: Span {line: 0, col: 0},
                },
            ],
            expr_type: Type::Unknown,
            span: Span {line: 0, col: 0},
        }));

        assert_eq!(result, Some(Expr::Cases(CasesExpr {
            scrutinee: Box::new(Expr::Tuple(vec![
                Expr::Ident(Ident::Symbol(0)),
                Expr::Ident(Ident::Symbol(1)),
            ])),
            arms: vec![
                CasesArm {
                    pattern: vec![
                        SimplePattern::Tuple(vec![
                            SimplePattern::Literal(Literal::Int(1)),
                            SimplePattern::Literal(Literal::Bool(true)),
                        ]),
                        SimplePattern::Tuple(vec![
                            SimplePattern::Literal(Literal::Int(0)),
                            SimplePattern::Literal(Literal::Bool(false)),
                        ]),
                    ],
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span {line: 0, col: 0},
                },
                CasesArm {
                    pattern: vec![SimplePattern::Default],
                    expr: Expr::Literal(Literal::Bool(false)),
                    arm_span: Span {line: 0, col: 0},
                },
            ],
            expr_type: Type::Bool,
            span: Span {line: 0, col: 0},
        })));
    }

    #[test]
    fn cases_expr_2() {
        // cases (r, b) {
        //     (1, true) : true,
        //     _ : 0, // mismatching return types
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "r".to_string(),
                    kind: SymbolKind::Variable(Type::Real),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "b".to_string(),
                    kind: SymbolKind::Variable(Type::Bool),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("r".to_string(), 0),
                        ("b".to_string(), 1)
                    ])
                },
            ],
            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.add_types_expr(Expr::Cases(CasesExpr {
            scrutinee: Box::new(Expr::Tuple(vec![
                Expr::Ident(Ident::Symbol(0)),
                Expr::Ident(Ident::Symbol(1)),
            ])),
            arms: vec![
                CasesArm {
                    pattern: vec![SimplePattern::Tuple(vec![
                        SimplePattern::Literal(Literal::Int(1)),
                        SimplePattern::Literal(Literal::Bool(true)),
                    ])],
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span {line: 0, col: 0},
                },
                CasesArm {
                    pattern: vec![SimplePattern::Default],
                    expr: Expr::Literal(Literal::Int(0)),
                    arm_span: Span {line: 0, col: 0},
                },
            ],
            expr_type: Type::Unknown,
            span: Span {line: 0, col: 0},
        }));

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn cases_expr_3() {
        // cases (r, b) {
        //     (1, true) | 0 : true, // not a tuple
        //     _ : false,
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "r".to_string(),
                    kind: SymbolKind::Variable(Type::Real),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "b".to_string(),
                    kind: SymbolKind::Variable(Type::Bool),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("r".to_string(), 0),
                        ("b".to_string(), 1)
                    ])
                },
            ],
            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.add_types_expr(Expr::Cases(CasesExpr {
            scrutinee: Box::new(Expr::Tuple(vec![
                Expr::Ident(Ident::Symbol(0)),
                Expr::Ident(Ident::Symbol(1)),
            ])),
            arms: vec![
                CasesArm {
                    pattern: vec![
                        SimplePattern::Tuple(vec![
                            SimplePattern::Literal(Literal::Int(1)),
                            SimplePattern::Literal(Literal::Bool(true)),
                        ]),
                        SimplePattern::Literal(Literal::Int(0)),
                    ],
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span {line: 0, col: 0},
                },
                CasesArm {
                    pattern: vec![SimplePattern::Default],
                    expr: Expr::Literal(Literal::Bool(false)),
                    arm_span: Span {line: 0, col: 0},
                },
            ],
            expr_type: Type::Unknown,
            span: Span {line: 0, col: 0},
        }));

        diagnostics.debug_print();
        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn cases_expr_4() {
        // cases (r, b) {
        //     (1, true) | (false, 0) : true,
        //     _ : false,
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "r".to_string(),
                    kind: SymbolKind::Variable(Type::Real),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "b".to_string(),
                    kind: SymbolKind::Variable(Type::Bool),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("r".to_string(), 0),
                        ("b".to_string(), 1)
                    ])
                },
            ],
            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.add_types_expr(Expr::Cases(CasesExpr {
            scrutinee: Box::new(Expr::Tuple(vec![
                Expr::Ident(Ident::Symbol(0)),
                Expr::Ident(Ident::Symbol(1)),
            ])),
            arms: vec![
                CasesArm {
                    pattern: vec![
                        SimplePattern::Tuple(vec![
                            SimplePattern::Literal(Literal::Int(1)),
                            SimplePattern::Literal(Literal::Bool(true)),
                        ]),
                        SimplePattern::Tuple(vec![
                            SimplePattern::Literal(Literal::Bool(false)),
                            SimplePattern::Literal(Literal::Int(0)),
                        ]),
                    ],
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span {line: 0, col: 0},
                },
                CasesArm {
                    pattern: vec![SimplePattern::Default],
                    expr: Expr::Literal(Literal::Bool(false)),
                    arm_span: Span {line: 0, col: 0},
                },
            ],
            expr_type: Type::Unknown,
            span: Span {line: 0, col: 0},
        }));

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 2);
    }

    #[test]
    fn cases_expr_5() {
        // cases (r, b) {
        //     (1, true) : true,
        //     (false, 0) : true, // swapped arguments
        //     _ : false,
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "r".to_string(),
                    kind: SymbolKind::Variable(Type::Real),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "b".to_string(),
                    kind: SymbolKind::Variable(Type::Bool),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("r".to_string(), 0),
                        ("b".to_string(), 1)
                    ])
                },
            ],
            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.add_types_expr(Expr::Cases(CasesExpr {
            scrutinee: Box::new(Expr::Tuple(vec![
                Expr::Ident(Ident::Symbol(0)),
                Expr::Ident(Ident::Symbol(1)),
            ])),
            arms: vec![
                CasesArm {
                    pattern: vec![
                        SimplePattern::Tuple(vec![
                            SimplePattern::Literal(Literal::Int(1)),
                            SimplePattern::Literal(Literal::Bool(true)),
                        ]),
                    ],
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span {line: 0, col: 0},
                },
                CasesArm {
                    pattern: vec![
                        SimplePattern::Tuple(vec![
                            SimplePattern::Literal(Literal::Bool(false)),
                            SimplePattern::Literal(Literal::Int(0)),
                        ]),
                    ],
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span {line: 0, col: 0},
                },
                CasesArm {
                    pattern: vec![SimplePattern::Default],
                    expr: Expr::Literal(Literal::Bool(false)),
                    arm_span: Span {line: 0, col: 0},
                },
            ],
            expr_type: Type::Unknown,
            span: Span {line: 0, col: 0},
        }));

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 2);
    }

    #[test]
    fn cases_expr_6() {
        // cases (r, b) {
        //     (1, true, 1) : true, // mismatched lengths
        //     _ : false,
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "r".to_string(),
                    kind: SymbolKind::Variable(Type::Real),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "b".to_string(),
                    kind: SymbolKind::Variable(Type::Bool),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("r".to_string(), 0),
                        ("b".to_string(), 1)
                    ])
                },
            ],
            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.add_types_expr(Expr::Cases(CasesExpr {
            scrutinee: Box::new(Expr::Tuple(vec![
                Expr::Ident(Ident::Symbol(0)),
                Expr::Ident(Ident::Symbol(1)),
            ])),
            arms: vec![
                CasesArm {
                    pattern: vec![
                        SimplePattern::Tuple(vec![
                            SimplePattern::Literal(Literal::Int(1)),
                            SimplePattern::Literal(Literal::Bool(true)),
                            SimplePattern::Literal(Literal::Int(1)),
                        ]),
                    ],
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span {line: 0, col: 0},
                },
                CasesArm {
                    pattern: vec![SimplePattern::Default],
                    expr: Expr::Literal(Literal::Bool(false)),
                    arm_span: Span {line: 0, col: 0},
                },
            ],
            expr_type: Type::Unknown,
            span: Span {line: 0, col: 0},
        }));

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn cases_expr_7() {
        // cases c {  // C is type COIN = {H, T}
        //     T : true,
        //     _ : false,
        // }
        let mut diagnostics = Diagnostics::new();  // TODO v
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "COIN".to_string(),
                    kind: SymbolKind::EntType,
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "H".to_string(),
                    kind: SymbolKind::EntMember{
                        parent: 0,
                        mapping: 0,
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "T".to_string(),
                    kind: SymbolKind::EntMember{
                        parent: 0,
                        mapping: 1,
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "c".to_string(),
                    kind: SymbolKind::Variable(Type::Custom(Ident::Symbol(0))),
                    span: Span {line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("COIN".to_string(), 0),
                        ("H".to_string(), 1),
                        ("T".to_string(), 2),
                        ("c".to_string(), 3),
                    ])
                },
            ],
            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.add_types_expr(Expr::Cases(CasesExpr {
            scrutinee: Box::new(Expr::Ident(Ident::Symbol(3))),
            arms: vec![
                CasesArm {
                    pattern: vec![
                        SimplePattern::Ident(Ident::Symbol(2)),
                    ],
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span {line: 0, col: 0},
                },
                CasesArm {
                    pattern: vec![SimplePattern::Default],
                    expr: Expr::Literal(Literal::Bool(false)),
                    arm_span: Span {line: 0, col: 0},
                },
            ],
            expr_type: Type::Unknown,
            span: Span {line: 0, col: 0},
        }));

        assert_eq!(result, Some(Expr::Cases(CasesExpr {
            scrutinee: Box::new(Expr::Ident(Ident::Symbol(3))),
            arms: vec![
                CasesArm {
                    pattern: vec![
                        SimplePattern::Ident(Ident::Symbol(2)),
                    ],
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span {line: 0, col: 0},
                },
                CasesArm {
                    pattern: vec![SimplePattern::Default],
                    expr: Expr::Literal(Literal::Bool(false)),
                    arm_span: Span {line: 0, col: 0},
                },
            ],
            expr_type: Type::Bool,
            span: Span {line: 0, col: 0},
        })));
    }

    #[test]
    fn cases_expr_8() {
        // cases c {  // C is type COIN = {H, T}
        //     T : true,
        //     B : false, // Different ent type
        // }
        let mut diagnostics = Diagnostics::new();  // TODO v
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "COIN".to_string(),
                    kind: SymbolKind::EntType,
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "H".to_string(),
                    kind: SymbolKind::EntMember{
                        parent: 0,
                        mapping: 0,
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "T".to_string(),
                    kind: SymbolKind::EntMember{
                        parent: 0,
                        mapping: 1,
                    },
                    span: Span {line: 0, col: 0},
                },
                
                Symbol {
                    name: "A".to_string(),
                    kind: SymbolKind::EntType,
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "B".to_string(),
                    kind: SymbolKind::EntMember{
                        parent: 3,
                        mapping: 0,
                    },
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    name: "c".to_string(),
                    kind: SymbolKind::Variable(Type::Custom(Ident::Symbol(0))),
                    span: Span {line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("COIN".to_string(), 0),
                        ("H".to_string(), 1),
                        ("T".to_string(), 2),
                        ("A".to_string(), 3),
                        ("B".to_string(), 4),
                        ("c".to_string(), 5),
                    ])
                },
            ],
            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.add_types_expr(Expr::Cases(CasesExpr {
            scrutinee: Box::new(Expr::Ident(Ident::Symbol(5))),
            arms: vec![
                CasesArm {
                    pattern: vec![
                        SimplePattern::Ident(Ident::Symbol(2)),
                    ],
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span {line: 0, col: 0},
                },
                CasesArm {
                    pattern: vec![
                        SimplePattern::Ident(Ident::Symbol(4))
                    ],
                    expr: Expr::Literal(Literal::Bool(false)),
                    arm_span: Span {line: 0, col: 0},
                },
            ],
            expr_type: Type::Unknown,
            span: Span {line: 0, col: 0},
        }));

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn sample_expr_1() {
        // sample {
        //     0.5 : true,
        //     _ : false,
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![],
            scopes: vec![],
            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.add_types_expr(Expr::Sample(SampleExpr {
            arms: vec![
                SampleArm {
                    prob: Prob::Expr(Expr::Literal(Literal::Real(0.5))),
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span {line: 0, col: 0},
                },
                SampleArm {
                    prob: Prob::Default,
                    expr: Expr::Literal(Literal::Bool(false)),
                    arm_span: Span {line: 0, col: 0},
                },
            ],
            expr_type: Type::Unknown,
            span: Span {line: 0, col: 0},
        }));

        assert_eq!(result, Some(Expr::Sample(SampleExpr {
            arms: vec![
                SampleArm {
                    prob: Prob::Expr(Expr::Literal(Literal::Real(0.5))),
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span {line: 0, col: 0},
                },
                SampleArm {
                    prob: Prob::Default,
                    expr: Expr::Literal(Literal::Bool(false)),
                    arm_span: Span {line: 0, col: 0},
                },
            ],
            expr_type: Type::Bool,
            span: Span {line: 0, col: 0},
        })));
    }

    #[test]
    fn sample_expr_2() {
        // sample {
        //     0.5 : true,
        //     0.2 : 1,
        //     _ : 0,
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer::new(
            Program {items: Vec::new()},
            &mut diagnostics,
        );

        let result = sem_analyzer.add_types_expr(Expr::Sample(SampleExpr {
            arms: vec![
                SampleArm {
                    prob: Prob::Expr(Expr::Literal(Literal::Real(0.5))),
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span {line: 0, col: 0},
                },
                SampleArm {
                    prob: Prob::Expr(Expr::Literal(Literal::Real(0.2))),
                    expr: Expr::Literal(Literal::Int(1)),
                    arm_span: Span {line: 0, col: 0},
                },
                SampleArm {
                    prob: Prob::Default,
                    expr: Expr::Literal(Literal::Int(0)),
                    arm_span: Span {line: 0, col: 0},
                },
            ],
            expr_type: Type::Unknown,
            span: Span {line: 0, col: 0},
        }));

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 2);
    }

    #[test]
    fn sample_expr_3() { 
        // ent_t COIN = {H, T};
        //
        // sample {
        //     0.5 : H,
        //     _ : T,
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "COIN".to_string(),
                    kind: SymbolKind::EntType,
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "H".to_string(),
                    kind: SymbolKind::EntMember{
                        parent: 0,
                        mapping: 0,
                    },
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "T".to_string(),
                    kind: SymbolKind::EntMember{
                        parent: 0,
                        mapping: 1,
                    },
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("COIN".to_string(), 0),
                        ("H".to_string(), 1),
                        ("T".to_string(), 2)
                    ])
                },
            ],
            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.add_types_expr(Expr::Sample(SampleExpr {
            arms: vec![
                SampleArm {
                    prob: Prob::Expr(Expr::Literal(Literal::Real(0.5))),
                    expr: Expr::Ident(Ident::Symbol(1)),
                    arm_span: Span {line: 0, col: 0},
                },
                SampleArm {
                    prob: Prob::Default,
                    expr: Expr::Ident(Ident::Symbol(2)),
                    arm_span: Span {line: 0, col: 0},
                },
            ],
            expr_type: Type::Unknown,
            span: Span {line: 0, col: 0},
        }));

        diagnostics.debug_print();

        assert_eq!(result, Some(Expr::Sample(SampleExpr {
            arms: vec![
                SampleArm {
                    prob: Prob::Expr(Expr::Literal(Literal::Real(0.5))),
                    expr: Expr::Ident(Ident::Symbol(1)),
                    arm_span: Span {line: 0, col: 0},
                },
                SampleArm {
                    prob: Prob::Default,
                    expr: Expr::Ident(Ident::Symbol(2)),
                    arm_span: Span {line: 0, col: 0},
                },
            ],
            expr_type: Type::Custom(Ident::Symbol(0)),
            span: Span {line: 0, col: 0},
        })));
    }

    #[test]
    fn sample_expr_4() {
        // sample {
        //     true : true, // Non real probability
        //     _ : false,
        // }
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![],
            scopes: vec![],
            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.add_types_expr(Expr::Sample(SampleExpr {
            arms: vec![
                SampleArm {
                    prob: Prob::Expr(Expr::Literal(Literal::Bool(true))),
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span {line: 0, col: 0},
                },
                SampleArm {
                    prob: Prob::Default,
                    expr: Expr::Literal(Literal::Bool(false)),
                    arm_span: Span {line: 0, col: 0},
                },
            ],
            expr_type: Type::Unknown,
            span: Span {line: 0, col: 0},
        }));

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }
}