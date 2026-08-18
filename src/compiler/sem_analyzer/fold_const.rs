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

//! # fold_const
//!
//! Handles folding of compile-time constants in the annotated ast
//!
//! Author: Cole Francis

use super::SemAnalyzer;

use crate::compiler::parser::ast::*;

impl <'a> SemAnalyzer<'a> {
    pub(super) fn fold_const(&mut self) {
        let items = std::mem::take(&mut self.ast.items);
        self.ast.items = Vec::with_capacity(items.len());

        for item in items {
            if let Some(folded_item) = self.fold_item(item) {
                self.ast.items.push(folded_item);
            }
        }
    }

    fn fold_item(&mut self, item: Item) -> Option<Item> {
        match item {
            Item::Let(stmt) => {
                self.fold_let(stmt);
                None
            }

            Item::Rel(rel_type) => Some(Item::Rel(self.fold_rel(rel_type))),
            Item::Net(net_type) => Some(Item::Net(self.fold_net(net_type))),

            other => Some(other),
        }
    }

    fn fold_let(&mut self, mut stmt: LetStatement) {

    }

    fn fold_rel(&mut self, mut rel_t: RelType) -> RelType {
        rel_t
    }

    fn fold_net(&mut self, mut net_t: Net) -> Net {
        net_t
    }
}