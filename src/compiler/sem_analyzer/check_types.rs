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
use crate::compiler::parser::ast::*;

impl <'a> SemAnalyzer<'a> {
    pub(super) fn check_types(&mut self) {
        let mut items = std::mem::take(&mut self.ast.items);

        for item in &mut items {
            self.check_item(item);
        }

        self.ast.items = items;
    }

    fn check_item(&mut self, item: &mut Item) {
        match item {
            Item::Let(stmt)     => self.check_let(stmt),
            Item::Ent(ent_type) => self.check_ent(ent_type),
            Item::Rel(rel_type) => self.check_rel(rel_type),
            Item::Net(net)      => self.check_net(net),
            Item::Error         => {}
        }
    }

    fn check_let(&mut self, stmt: &mut LetStatement) {

    }

    fn check_ent(&mut self, ent_t: &mut EntType) {

    }

    fn check_rel(&mut self, rel_t: &mut RelType) {

    }

    fn check_net(&mut self, net: &mut Net) {

    }
}