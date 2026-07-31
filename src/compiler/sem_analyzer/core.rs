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
use crate::compiler::parser::ast::Program;
use crate::compiler::diagnostics::Diagnostics;
use crate::compiler::sem_analyzer::symbol::Symbol;
use crate::compiler::sem_analyzer::scope::Scope;

impl <'a> SemAnalyzer<'a> {
    pub fn new(ast: Program, diagnostics: &'a mut Diagnostics) -> Self {
        Self {
            ast,
            symbols: Vec::new(),
            scopes: vec![
                Scope {
                    parent: None,
                    symbols: HashMap::new(),
                }
            ],
            current_scope: 0,
            diagnostics,
        }
    }

    pub fn analyze(mut self) -> (Program, Vec<Symbol>) {
        self.resolve_names();
        self.check_types();
        self.fold_const();

        (self.ast, self.symbols)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
// }