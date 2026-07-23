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

//! # scope
//!
//! Holds the structures used in creating the scope
//!
//! ## Invariants
//!
//! - 
//!
//! Author: Cole Francis

use std::collections::HashMap;
use super::symbol::SymbolId;

pub type ScopeId = usize;

pub struct Scope {
    pub parent: Option<ScopeId>,
    pub symbols: HashMap<String, SymbolId>,
}

// Vec<Scope>
// whenever enterinc scope, create a new scope, define its parents, and push it onto the vector. Then when leaving scope, pop back tto parents

// LOOKUP ALGORITHM
// fn lookup(name: &str, mut scope: ScopeId) -> Option<SymbolId> {
//     loop {
//         let s = &self.scopes[scope];

//         if let Some(id) = s.symbols.get(name) {
//             return Some(*id);
//         }

//         match s.parent {
//             Some(parent) => scope = parent,
//             None => return None,
//         }
//     }
// }

// Declare a symbol:
//  search current scope. if present, duplicate definition compiler error
//  Create symbol struct
//  Store in symbols
//  insert into scope
//      current_scope.symbols.insert("x", 42);
// I dont want duplicate definition
//     when declaring, search current s

// when a symbol is used, look it up in scope
//  if its not there, emit undeclared <thing> compiler error