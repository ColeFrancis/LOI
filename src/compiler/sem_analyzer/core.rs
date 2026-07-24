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

use super::SemAnalyzer;
use crate::compiler::parser::ast::Program;
use crate::compiler::diagnostics::Diagnostics;
use crate::compiler::sem_analyzer::symbol::{Symbol, SymbolId};

impl <'a> SemAnalyzer<'a> {
    pub fn new(ast: Program, diagnostics: &'a mut Diagnostics) -> Self {
        Self {
            ast,
            symbols: Vec::new(),
            scopes: Vec::new(),
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

    pub(super) fn lookup_symbol(&self, name: &str) -> Option<SymbolId> {
        let mut scope = self.current_scope;

        loop {
            let s = &self.scopes[scope];

            if let Some(id) = s.symbols.get(name) {
                return Some(*id);
            }

            match s.parent {
                Some(parent) => scope = parent,
                None => return None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::compiler::sem_analyzer::scope::Scope;
    use crate::compiler::sem_analyzer::symbol::SymbolKind;
    use crate::compiler::diagnostics::Span;

    #[test]
    fn test_lookup() {
        let sem_analyzer = SemAnalyzer {
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
                },
                Symbol {
                    id: 2,
                    name: "c".to_string(),
                    kind: SymbolKind::Variable,
                    span: Span{line: 0, col: 0},
                },
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
                },
                Scope {
                    parent: Some(1),
                    symbols: HashMap::from([
                        ("c".to_string(), 1),
                    ])
                },
            ],
            current_scope: 1,

            diagnostics: &mut Diagnostics::new(),
        };

        assert_eq!(sem_analyzer.lookup_symbol("a"), Some(0));
        assert_eq!(sem_analyzer.lookup_symbol("b"), Some(1));
        assert_eq!(sem_analyzer.lookup_symbol("c"), None);
    }
}