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

//! # resolve_names
//!
//! Handles name resolution and building the symbol table of semantic analysis
//!
//! ## Invariants
//!
//! - 
//!
//! Author: Cole Francis

use std::collections::HashMap;
use super::SemAnalyzer;
use crate::compiler::parser::ast::*;
use crate::compiler::diagnostics::CompilerError;
use super::symbol::{Symbol, SymbolKind, SymbolId};
use crate::compiler::diagnostics::Span;
use crate::compiler::sem_analyzer::scope::Scope;

impl <'a> SemAnalyzer<'a> {
    pub(super) fn resolve_names(&mut self) {
        // for mut item in self.ast.items {
        //     item = match self.resolve_item(item) {
        //         Some(item) => item,
        //         None => Item::Error,
        //     }
        // }
    }

    // fn resolve_item(&mut self, item: Item) -> Option<Item> {
    //     match item {
    //         Item::Let(stmt) => self.resolve_let(stmt)

    //         _ => None,
    //     }
    // }

    fn resolve_let(&mut self, stmt: LetStatement) -> Option<LetStatement> {
        let (name, span) = self.extract_ident_str(stmt.name)?; // Should not return None
        let symbol_id = self.define_symbol(
            name, 
            SymbolKind::Variable, 
            span,
        )?;

        let expr = match self.resolve_expr(stmt.expr) {
            Some(expr) => expr,
            None => Expr::Error,
        };
        
        Some(LetStatement {
            name: Ident::Symbol(symbol_id),
            expr,
        })
    }

    // fn resolve_ent(&mut self, ent_t) -> ann_ast::EntType {

    // }

    // fn resolve_rel(&mut self, rel_t) -> ann_ast::RelType {

    // }

    // fn resolve_rel(&mut self, net) -> ann_ast::Net {

    // }

    fn resolve_expr(&mut self, expr: Expr) -> Option<Expr> {
        match expr { 
            Expr::Literal(literal) => Some(Expr::Literal(literal)),

            Expr::Ident(ident) => {
                let (name, span) = self.extract_ident_str(ident)?;

                let symbol = self.find_symbol(&name, span)?;

                Some(Expr::Ident(Ident::Symbol(symbol)))
            },

            _ => None,
        }
    }

    fn extract_ident_str(&self, ident: Ident) -> Option<(String, Span)> {
        match ident {
            Ident::Str {val, span} => Some((val, span)),
            Ident::Symbol(_) => None,
        }
    }

    fn define_symbol(&mut self, name: String, kind: SymbolKind, span: Span) -> Option<SymbolId> {
        // No duplicate definitions
        if self.scopes[self.current_scope].symbols.contains_key(&name) {
            self.diagnostics.error(CompilerError::DuplicateDefinition {
                name,
                span,
            });

            return None;
        }
        
        let id = self.symbols.len();

        self.symbols.push(Symbol {
            id,
            name: name.clone(),
            kind,
            span,
        });

        self.scopes[self.current_scope].symbols.insert(name, id);

        Some(id)
    }

    fn create_scope(&mut self) {
        let new_scope = self.scopes.len();

        self.scopes.push(Scope {
            parent: Some(self.current_scope),
            symbols: HashMap::new(),
        });

        self.current_scope = new_scope;
    }

    fn exit_scope(&mut self) -> Option<()> {
        self.current_scope = self.scopes[self.current_scope].parent?;
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::diagnostics::Diagnostics;
    use crate::compiler::sem_analyzer::scope::Scope;

    #[test]
    fn test_define_1() {
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    id: 0,
                    name: "a".to_string(),
                    kind: SymbolKind::Variable,
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    id: 1,
                    name: "b".to_string(),
                    kind: SymbolKind::Variable,
                    span: Span{line: 0, col: 0},
                }
            ],
            scopes: vec![
                Scope {
                    parent: None,
                    symbols: HashMap::from([
                        ("a".to_string(), 0),
                    ])
                },
                Scope {
                    parent: Some(0),
                    symbols: HashMap::from([
                        ("b".to_string(), 1),
                    ])
                }
            ],
            current_scope: 1,

            diagnostics: &mut Diagnostics::new(),
        };

        let result = sem_analyzer.define_symbol("a".to_string(), SymbolKind::Variable, Span{line:0,col:0});

        assert_eq!(result, Some(2));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                id: 0,
                name: "a".to_string(),
                kind: SymbolKind::Variable,
                span: Span{line: 0, col: 0},
            },
            Symbol {
                id: 1,
                name: "b".to_string(),
                kind: SymbolKind::Variable,
                span: Span{line: 0, col: 0},
            },
            Symbol {
                id: 2,
                name: "a".to_string(),
                kind: SymbolKind::Variable,
                span: Span{line: 0, col: 0},
            }
        ]);
        assert_eq!(sem_analyzer.scopes, vec![
            Scope {
                parent: None,
                symbols: HashMap::from([
                    ("a".to_string(), 0),
                ])
            },
            Scope {
                parent: Some(0),
                symbols: HashMap::from([
                    ("b".to_string(), 1),
                    ("a".to_string(), 2),
                ])
            }
        ]);
    }
    
    #[test]
    fn test_define_2() {
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    id: 0,
                    name: "a".to_string(),
                    kind: SymbolKind::Variable,
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    id: 1,
                    name: "b".to_string(),
                    kind: SymbolKind::Variable,
                    span: Span{line: 0, col: 0},
                }
            ],
            scopes: vec![
                Scope {
                    parent: None,
                    symbols: HashMap::from([
                        ("a".to_string(), 0),
                    ])
                },
                Scope {
                    parent: Some(0),
                    symbols: HashMap::from([
                        ("b".to_string(), 1),
                    ])
                }
            ],
            current_scope: 0,

            diagnostics: &mut Diagnostics::new(),
        };

        let result = sem_analyzer.define_symbol("a".to_string(), SymbolKind::Variable, Span{line:0,col:0});

        assert_eq!(result, None);
        assert_eq!(sem_analyzer.diagnostics.num_errors(), 1);
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                id: 0,
                name: "a".to_string(),
                kind: SymbolKind::Variable,
                span: Span{line: 0, col: 0},
            },
            Symbol {
                id: 1,
                name: "b".to_string(),
                kind: SymbolKind::Variable,
                span: Span{line: 0, col: 0},
            },
        ]);
        assert_eq!(sem_analyzer.scopes, vec![
            Scope {
                parent: None,
                symbols: HashMap::from([
                    ("a".to_string(), 0),
                ])
            },
            Scope {
                parent: Some(0),
                symbols: HashMap::from([
                    ("b".to_string(), 1),
                ])
            }
        ]);
    }

    #[test]
    fn enter_exit_scope() {
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer::new(
            Program {items: Vec::new()},
            &mut diagnostics,
        );

        sem_analyzer.create_scope();

        assert_eq!(sem_analyzer.current_scope, 1);
        assert_eq!(sem_analyzer.scopes, vec![
            Scope {
                parent: None,
                symbols: HashMap::from([
                ])
            },
            Scope {
                parent: Some(0),
                symbols: HashMap::from([
                ])
            }
        ]);

        let result = sem_analyzer.exit_scope();

        assert_eq!(result, Some(()));
        assert_eq!(sem_analyzer.current_scope, 0);

        let result = sem_analyzer.exit_scope();

        assert_eq!(result, None);
    }

    #[test]
    fn expr_literal() {
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![],
            scopes: vec![
                Scope {
                    parent: None,
                    symbols: HashMap::new(),
                },
            ],
            current_scope: 0,

            diagnostics: &mut Diagnostics::new(),
        };

        let result = sem_analyzer.resolve_expr(Expr::Literal(Literal::Bool(false)));

        assert_eq!(result, Some(Expr::Literal(Literal::Bool(false))));
    }
    
    #[test]
    fn expr_ident_fail() {
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![],
            scopes: vec![
                Scope {
                    parent: None,
                    symbols: HashMap::new(),
                },
            ],
            current_scope: 0,

            diagnostics: &mut Diagnostics::new(),
        };

        let result = sem_analyzer.resolve_expr(Expr::Ident(Ident::Str {
            val: "a".to_string(),
            span: Span{line: 1, col: 0},
        }));

        assert_eq!(result, None);
    }

    #[test]
    fn expr_ident() {
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![],
            scopes: vec![
                Scope {
                    parent: None,
                    symbols: HashMap::from([
                        ("a".to_string(), 1),
                    ]),
                },
            ],
            current_scope: 0,

            diagnostics: &mut Diagnostics::new(),
        };

        let result = sem_analyzer.resolve_expr(Expr::Ident(Ident::Str {
            val: "a".to_string(),
            span: Span{line: 2, col: 0},
        }));

        assert_eq!(result, Some(Expr::Ident(Ident::Symbol(1))));
    }

    // #[test]
    // fn resolve_let() {
    //     let mut diagnostics = Diagnostics::new()
    //     let mut sem_analyzer = SemAnalyzer::new(
    //         Program{items: vec![]},
    //         &mut diagnostics,
    //     );

    //     let result = sem_analyzer.resolve_let(LetStatement {
    //         name: Ident::Str {
    //             val: "a".to_string(),
    //             span: Span{line: 1, col: 2},
    //         },
    //         expr: Expr::Literal(Literal::Bool(false)),
    //     });

    //     assert_eq!(result, Some(LetStatement {
    //         name: Ident::Symbol(0),
    //         expr: Expr::Literal(Literal::Bool(false)),
    //     }));
    //     assert_eq!(sem_analyzer.symbols, vec![
    //         Symbol {
    //             id: 0,
    //             name: "a".to_string(),
    //             kind: SymbolKind::Variable,
    //             span: Span{line: 1, col: 2},
    //         },
    //     ]);
    // }
}