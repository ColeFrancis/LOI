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
        Some(rel_t)
    }

    fn check_net(&mut self, mut net: Net) -> Option<Net> {
        Some(net)
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
}