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
use crate::compiler::parser::ast;
use super::ann_ast;

impl <'a> SemAnalyzer<'a> {
    pub(super) fn resolve_names(&mut self, ast: ast::Program) {
        for item in ast.items {
            let ann_item = match self.resolve_item(item) {
                Some(item) => item,
                None => ann_ast::Item::Error,
            };

            self.ann_ast.items.push(ann_item);
        }
    }

    fn resolve_item(&mut self, item: ast::Item) -> Option<ann_ast::Item> {
        match item {
            // ast::Item::Let(stmt) => {
            //     ann_ast::Item::Let(self.resolve_let(stmt))
            // }

            // ast::Item::Ent(ent_t) => {
            //     ann_ast::Item::Ent(self.resolve_ent(ent_t))
            // }

            // ast::Item::Rel(rel_t) => {
            //     ann_ast::Item::Rel(self.resolve_rel(rel_t))
            // }

            // ast::Item::Net(net) => {
            //     ann_ast::Item::Net(self.resolve_net(net))
            // }

            // ast::Item::Error => {
            //     ann_ast::Item::Error
            // }

            _ => None,
        }
    }

    // fn resolve_let(&mut self, stmt) -> ann_ast::LetStatement {

    // }

    // fn resolve_ent(&mut self, ent_t) -> ann_ast::EntType {

    // }

    // fn resolve_rel(&mut self, rel_t) -> ann_ast::RelType {

    // }

    // fn resolve_rel(&mut self, net) -> ann_ast::Net {

    // }
}