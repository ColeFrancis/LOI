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

//! # check_types
//!
//! Handles checking and assigning of types for annotated ast
//!
//! ## Invariants
//!
//! - All incoming symbols will have Type::Unknown
//!
//! Author: Cole Francis

use super::SemAnalyzer;
use super::types::Type;
use super::symbol::SymbolKind;
use crate::compiler::parser::ast::*;
use crate::compiler::diagnostics::{CompilerError, Span};

impl <'a> SemAnalyzer<'a> {
    // Add type info to symbols
    // Verify all types match
    // Check number of relation arguments
    // Also check only one default in samples/Caseses
    pub(super) fn check_types(&mut self) {
        let items = std::mem::take(&mut self.ast.items);
        self.ast.items = Vec::with_capacity(items.len());

        for item in items {
            let resolved_item = self.check_item(item).unwrap_or(Item::Error);
            self.ast.items.push(resolved_item);
        }
    }

    fn check_item(&mut self, item: Item) -> Option<Item> {
        match item {
            Item::Let(stmt)     => Some(Item::Let(self.check_let(stmt))),
            Item::Ent(ent_type) => Some(Item::Ent(ent_type)),
            Item::Rel(rel_type) => self.check_rel(rel_type).map(Item::Rel),
            Item::Net(net)      => self.check_net(net).map(Item::Net),
            Item::Error         => Some(Item::Error),
        }
    }

    // There is no way checking types on LetStatement results in an error item.
    pub(super) fn check_let(&mut self, mut stmt: LetStatement) -> LetStatement {
        stmt.expr = self.add_types_expr(stmt.expr).unwrap_or(Expr::Error);

        let expr_type = self.get_expr_type(&stmt.expr);

        if let Ident::Symbol(symbol_id) = stmt.name {
            if let SymbolKind::Variable(var_type) = &mut self.symbols[symbol_id].kind {
                *var_type = expr_type;
            }
        }

        stmt
    }

    fn check_rel(&mut self, mut rel_t: RelType) -> Option<RelType> {
        // Annotate types of each parameter's symbol 
        // Annotate types of parameter and return in the relations name symbol
        // Check expression type matches return type

        for param in &mut rel_t.params {
            if let Ident::Symbol(param_id) = param.name {
                if let SymbolKind::Variable(param_symbol_type) = &mut self.symbols[param_id].kind {
                    *param_symbol_type = param.param_type.clone();
                }
            }


            if let Ident::Symbol(rel_name_id) = rel_t.name {
                if let SymbolKind::Rel_t {input_types, ..} = &mut self.symbols[rel_name_id].kind {
                    input_types.push(param.param_type.clone());
                }
            }
        }

        rel_t.body = self.add_types_expr(rel_t.body).unwrap_or(Expr::Error);

        let expr_type = self.get_expr_type(&rel_t.body);

        let span = match &rel_t.name {
            Ident::Str { span, ..} => span.clone(),
            Ident::Symbol(id) => self.symbols[*id].span.clone(),
        };

        rel_t.return_type = self.verify_rel_return_type(
            &rel_t.return_type, 
            &expr_type,
            &span
        ).unwrap_or(Type::Error);

        if let Ident::Symbol(rel_name_id) = rel_t.name {
            if let SymbolKind::Rel_t {return_type, ..} = &mut self.symbols[rel_name_id].kind {
                *return_type = rel_t.return_type.clone();
            }
        }

        Some(rel_t)
    }

    fn check_net(&mut self, mut net: Net) -> Option<Net> {
        Some(net)
    }

    fn verify_rel_return_type(&mut self, return_type: &Type, expr_type: &Type, rel_span: &Span) -> Option<Type> {
        match (return_type, expr_type) {
            (Type::Impulse, Type::Impulse) => Some(Type::Impulse),

            (Type::Bool, Type::Impulse) => Some(Type::Bool), // Keep?
            (Type::Bool, Type::Bool) => Some(Type::Bool),

            (Type::Mod(val_l), Type::Mod(val_r)) => {
                if val_l == val_r {
                    Some(Type::Mod(*val_l))
                }
                else {
                    self.diagnostics.error(CompilerError::IncmmpatibleReturnType {
                        return_type: return_type.clone(),
                        expr_type: expr_type.clone(),
                        rel_span: rel_span.clone(),
                    });

                    None
                }
            }
            (Type::Int,  Type::Mod(_)) => Some(Type::Int),
            (Type::Int,  Type::Int   ) => Some(Type::Int),
            (Type::Real, Type::Mod(_)) => Some(Type::Real),
            (Type::Real, Type::Int   ) => Some(Type::Real),
            (Type::Real, Type::Real  ) => Some(Type::Real),

            (Type::Custom(ident_l), Type::Custom(ident_r)) => {
                let (parent_l, parent_r) = match (ident_l, ident_r) {
                    (Ident::Symbol(id_l), Ident::Symbol(id_r)) => (*id_l, *id_r),
                    
                    _ => return None, // Unreachable after name resolution
                };

                if parent_l == parent_r {
                    Some(Type::Custom(Ident::Symbol(parent_l)))
                }
                else {
                    self.diagnostics.error(CompilerError::IncmmpatibleReturnType {
                        return_type: return_type.clone(),
                        expr_type: expr_type.clone(),
                        rel_span: rel_span.clone(),
                    });

                    None
                }
            }

            _ => {
                self.diagnostics.error(CompilerError::IncmmpatibleReturnType {
                    return_type: return_type.clone(),
                    expr_type: expr_type.clone(),
                    rel_span: rel_span.clone(),
                });

                None
            }
        }
    }

    // fn compare_types(&mut self, symbol_type: Type, object_type: Type, symbol_span: Span) -> Option<Type> {
    //     if symbol_type == object_type {
    //         return Some(symbol_type)
    //     }

    //     match symbol_type {
    //         Type::Unknown => {
    //             Some(object_type)
    //         }

    //         _ => {
    //             self.diagnostics.error(CompilerError::UnexpectedType {
    //                 expected: object_type,
    //                 found: symbol_type,
    //                 span: symbol_span,
    //             });

    //             None
    //         }
    //     }
    // }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::compiler::sem_analyzer::scope::Scope;
    use crate::compiler::sem_analyzer::symbol::Symbol;
    use crate::compiler::diagnostics::Diagnostics;

    #[test]
    fn check_let_1() {
        // let n = 1;
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

        let _result = sem_analyzer.check_let(LetStatement {
            name: Ident::Symbol(0),
            expr: Expr::Literal(Literal::Int(1)),
        });

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
    fn check_rel_1() {
        // rel_t ADD : (a: Real) -> Real = a + 1;
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    id: 0,
                    name: "ADD".to_string(),
                    kind: SymbolKind::Rel_t {
                        input_types: Vec::new(),
                        return_type: Type::Unknown,
                    },
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    id: 1,
                    name: "a".to_string(),
                    kind: SymbolKind::Variable(Type::Unknown),
                    span: Span{line: 0, col: 5},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("ADD".to_string(), 0),
                        ("a".to_string(), 1),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.check_rel(RelType {
            name: Ident::Symbol(0),
            params: vec![Param {
                name: Ident::Symbol(1),
                param_type: Type::Real,
            }],
            return_type: Type::Real,
            body: Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Ident(Ident::Symbol(1))),
                right: Box::new(Expr::Literal(Literal::Int(1))),
                op: BinaryOp::Add,
                op_span: Span {line: 0, col: 10},
                expr_type: Type::Unknown,
            }),
        });

        assert_eq!(result, Some(RelType {
            name: Ident::Symbol(0),
            params: vec![Param {
                name: Ident::Symbol(1),
                param_type: Type::Real,
            }],
            return_type: Type::Real,
            body: Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Ident(Ident::Symbol(1))),
                right: Box::new(Expr::Literal(Literal::Int(1))),
                op: BinaryOp::Add,
                op_span: Span {line: 0, col: 10},
                expr_type: Type::Real,
            }),
        }));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                id: 0,
                name: "ADD".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Real],
                    return_type: Type::Real,
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                id: 1,
                name: "a".to_string(),
                kind: SymbolKind::Variable(Type::Real),
                span: Span{line: 0, col: 5},
            },
        ]);
    }

    #[test]
    fn check_rel_2() {
        // rel_t EX : () -> PARENT = CHILD;
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    id: 0,
                    name: "PARENT".to_string(),
                    kind: SymbolKind::EntType,
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    id: 1,
                    name: "CHILD".to_string(),
                    kind: SymbolKind::EntMember(0),
                    span: Span {line: 0, col: 0},
                },
                Symbol {
                    id: 2,
                    name: "EX".to_string(),
                    kind: SymbolKind::Rel_t {
                        input_types: Vec::new(),
                        return_type: Type::Unknown,
                    },
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("PARENT".to_string(), 0),
                        ("CHILD".to_string(), 1),
                        ("EX".to_string(), 1),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.check_rel(RelType {
            name: Ident::Symbol(2),
            params: vec![],
            return_type: Type::Custom(Ident::Symbol(0)),
            body: Expr::Ident(Ident::Symbol(1)),
        });

        assert_eq!(result, Some(RelType {
            name: Ident::Symbol(2),
            params: vec![],
            return_type: Type::Custom(Ident::Symbol(0)),
            body: Expr::Ident(Ident::Symbol(1)),
        }));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                id: 0,
                name: "PARENT".to_string(),
                kind: SymbolKind::EntType,
                span: Span {line: 0, col: 0},
            },
            Symbol {
                id: 1,
                name: "CHILD".to_string(),
                kind: SymbolKind::EntMember(0),
                span: Span {line: 0, col: 0},
            },
            Symbol {
                id: 2,
                name: "EX".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: Vec::new(),
                    return_type: Type::Custom(Ident::Symbol(0)),
                },
                span: Span{line: 0, col: 0},
            },
        ]);
    }

    #[test]
    fn check_rel_3() {
        // rel_t ADD : (a: Int) -> Int = a + 1.0; //  incompatible return type
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    id: 0,
                    name: "ADD".to_string(),
                    kind: SymbolKind::Rel_t {
                        input_types: Vec::new(),
                        return_type: Type::Unknown,
                    },
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    id: 1,
                    name: "a".to_string(),
                    kind: SymbolKind::Variable(Type::Unknown),
                    span: Span{line: 0, col: 5},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("ADD".to_string(), 0),
                        ("a".to_string(), 1),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.check_rel(RelType {
            name: Ident::Symbol(0),
            params: vec![Param {
                name: Ident::Symbol(1),
                param_type: Type::Int,
            }],
            return_type: Type::Int,
            body: Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Ident(Ident::Symbol(1))),
                right: Box::new(Expr::Literal(Literal::Real(1.0))),
                op: BinaryOp::Add,
                op_span: Span {line: 0, col: 10},
                expr_type: Type::Unknown,
            }),
        });

        assert_eq!(result, Some(RelType {
            name: Ident::Symbol(0),
            params: vec![Param {
                name: Ident::Symbol(1),
                param_type: Type::Int,
            }],
            return_type: Type::Error,
            body: Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Ident(Ident::Symbol(1))),
                right: Box::new(Expr::Literal(Literal::Real(1.0))),
                op: BinaryOp::Add,
                op_span: Span {line: 0, col: 10},
                expr_type: Type::Real,
            }),
        }));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                id: 0,
                name: "ADD".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Int],
                    return_type: Type::Error,
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                id: 1,
                name: "a".to_string(),
                kind: SymbolKind::Variable(Type::Int),
                span: Span{line: 0, col: 5},
            },
        ]);
        
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn check_rel_4() {
        // rel_t ADD : (a: Bool) -> Real = a + 1; // incompatible types
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    id: 0,
                    name: "ADD".to_string(),
                    kind: SymbolKind::Rel_t {
                        input_types: Vec::new(),
                        return_type: Type::Unknown,
                    },
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    id: 1,
                    name: "a".to_string(),
                    kind: SymbolKind::Variable(Type::Unknown),
                    span: Span{line: 0, col: 5},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("ADD".to_string(), 0),
                        ("a".to_string(), 1),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.check_rel(RelType {
            name: Ident::Symbol(0),
            params: vec![Param {
                name: Ident::Symbol(1),
                param_type: Type::Bool,
            }],
            return_type: Type::Real,
            body: Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Ident(Ident::Symbol(1))),
                right: Box::new(Expr::Literal(Literal::Int(1))),
                op: BinaryOp::Add,
                op_span: Span {line: 0, col: 10},
                expr_type: Type::Unknown,
            }),
        });

        assert_eq!(result, Some(RelType {
            name: Ident::Symbol(0),
            params: vec![Param {
                name: Ident::Symbol(1),
                param_type: Type::Bool,
            }],
            return_type: Type::Error,
            body: Expr::Error,
        }));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                id: 0,
                name: "ADD".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Bool],
                    return_type: Type::Error,
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                id: 1,
                name: "a".to_string(),
                kind: SymbolKind::Variable(Type::Bool),
                span: Span{line: 0, col: 5},
            },
        ]);
    }
}