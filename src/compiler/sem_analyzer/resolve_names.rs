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

use super::SemAnalyzer;
use crate::compiler::parser::ast::*;
use crate::compiler::diagnostics::CompilerError;
use super::symbol::{Symbol, SymbolKind, SymbolId};

// TEMPORARY
use crate::compiler::diagnostics::Span;

impl <'a> SemAnalyzer<'a> {
    pub(super) fn resolve_names(&mut self) {
        // for item in self.ast.items {
        //     let ann_item = match self.resolve_item(item) {
        //         Some(item) => item,
        //         None => ann_ast::Item::Error,
        //     };

        //     self.ann_ast.items.push(ann_item);
        // }
    }

    // fn resolve_item(&mut self, item: ast::Item) -> Option<ann_ast::Item> {
    //     match item {
    //         // ast::Item::Let(stmt) => match self.resolve_let(stmt) {
    //         //     Some(ann_stmt) => Some(ann_ast::Item::Let(ann_stmt)),
    //         //     None => None,
    //         // }

    //         // ast::Item::Ent(ent_t) => {
    //         //     ann_ast::Item::Ent(self.resolve_ent(ent_t))
    //         // }

    //         // ast::Item::Rel(rel_t) => {
    //         //     ann_ast::Item::Rel(self.resolve_rel(rel_t))
    //         // }

    //         // ast::Item::Net(net) => {
    //         //     ann_ast::Item::Net(self.resolve_net(net))
    //         // }

    //         // ast::Item::Error => {
    //         //     ann_ast::Item::Error
    //         // }

    //         _ => None,
    //     }
    // }

    // fn resolve_let(&mut self, stmt: ast::LetStatement) -> Option<ann_ast::LetStatement> {
    //     let expr = self.resolve_expr(stmt.expr)?;

    //     // TODO: add span to ast
    //     let symbol = self.define_symbol(
    //         stmt.name, 
    //         SymbolKind::Variable, 
    //         Span{line: 0, col: 0},
    //     )?;
        
    //     Some(ann_ast::LetStatement {
    //         symbol,
    //         expr,
    //     })
    // }

    // fn resolve_ent(&mut self, ent_t) -> ann_ast::EntType {

    // }

    // fn resolve_rel(&mut self, rel_t) -> ann_ast::RelType {

    // }

    // fn resolve_rel(&mut self, net) -> ann_ast::Net {

    // }

    // fn resolve_expr(&mut self, expr: ast::Expr) -> Option<ann_ast::Expr> {
    //     // TODO
    //     Some(ann_ast::Expr::Error)
    // }

    pub fn define_symbol(&mut self, name: String, kind: SymbolKind, span: Span) -> Option<SymbolId> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
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
}