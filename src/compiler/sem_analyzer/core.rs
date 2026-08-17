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

//! # core
//!
//! Handles the core of the semantic analyzer
//!
//! ## Invariants
//!
//! - 
//!
//! Author: Cole Francis

use std::collections::HashMap;

use super::SemAnalyzer;
use super::symbol::Symbol;
use super::scope::Scope;
use crate::compiler::parser::ast::Program;
use crate::compiler::diagnostics::Diagnostics;

impl <'a> SemAnalyzer<'a> {
    pub fn new(ast: Program, diagnostics: &'a mut Diagnostics) -> Self {
        Self {
            ast,
            symbols: Vec::new(),
            scopes: vec![
                Scope {
                    symbols: HashMap::new(),
                }
            ],
            diagnostics,
        }
    }

    pub fn analyze(mut self) -> (Program, Vec<Symbol>) {
        self.resolve_names();
        self.check_types();
        self.fold_const();
        // self.check_constraints();
            // values add up to 1 in sample
            // at most 1 default branch in cases and sample
            // inputs only assigned to outputs and vice versa of rel inst ad net inst in nets 
            // ents only driven once (not used as output multiple times)

        (self.ast, self.symbols)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
// }