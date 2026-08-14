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

                binary_expr.expr_type = self.verify_binary_expr_type_match(&left_expr_type, &right_expr_type, &binary_expr.op_span)?;

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

            // Check scrutinee matches each arm's pattern
            // Check that there is at most one default arm
            // Annotate expression type
            // Expr::Match(match_expr) => {}

            // Expr::Sample(sample_expr) => {}

            Expr::Error => Some(Expr::Error),

            _ => Some(Expr::Error)
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

    fn verify_binary_expr_type_match(&mut self, left: &Type, right: &Type, op_span: &Span) -> Option<Type> {
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
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    id: 0,
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

        // 1 + r // r is a Real
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
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    id: 0,
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

        // 1 + b // b is a bool
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
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    id: 0,
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

        // true + b // b is a bool
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
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    id: 0,
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

        // 1 < r // r is a Real
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
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    id: 0,
                    name: "z2".to_string(),
                    kind: SymbolKind::Variable(Type::Mod(2)),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    id: 1,
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

        // z2 + z3 // mod variables incompatible
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
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    id: 0,
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

        // {let n = 1; n}
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
                id: 0,
                name: "n".to_string(),
                kind: SymbolKind::Variable(Type::Int),
                span: Span{line: 0, col: 0},
            },
        ]);
    }

    #[test]
    fn block_expr_2() {
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    id: 0,
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

        // {let n = 1+true; n} // 1 and true are incompatible types
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
                id: 0,
                name: "n".to_string(),
                kind: SymbolKind::Variable(Type::Error),
                span: Span{line: 0, col: 0},
            },
        ]);
        assert_eq!(diagnostics.num_errors(), 1);
    }
}