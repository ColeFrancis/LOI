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
use crate::compiler::parser::ast::*;
use crate::compiler::diagnostics::{CompilerError, Span};

impl <'a> SemAnalyzer<'a> {
    // Add type info to symbols
    // Verify all types match
    // Check number of relation arguments
    // Also check only one default in samples/matches
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
            Item::Let(stmt)     => self.check_let(stmt).map(Item::Let),
            Item::Ent(ent_type) => self.check_ent(ent_type).map(Item::Ent),
            Item::Rel(rel_type) => self.check_rel(rel_type).map(Item::Rel),
            Item::Net(net)      => self.check_net(net).map(Item::Net),
            Item::Error         => Some(Item::Error),
        }
    }

    fn check_let(&mut self, mut stmt: LetStatement) -> Option<LetStatement> {
        // Recursively go through expr. Then at the end, set variable type to that
        Some(stmt)
    }

    fn check_ent(&mut self, mut ent_t: EntType) -> Option<EntType> {
        Some(ent_t)
    }

    fn check_rel(&mut self, mut rel_t: RelType) -> Option<RelType> {
        Some(rel_t)
    }

    fn check_net(&mut self, mut net: Net) -> Option<Net> {
        Some(net)
    }

    fn compare_types(&mut self, symbol_type: Type, object_type: Type, symbol_span: Span) -> Option<Type> {
        if symbol_type == object_type {
            return Some(symbol_type)
        }

        match symbol_type {
            Type::Unknown => {
                Some(object_type)
            }

            _ => {
                self.diagnostics.error(CompilerError::UnexpectedType {
                    expected: object_type,
                    found: symbol_type,
                    span: symbol_span,
                });

                None
            }
        }
    }
}