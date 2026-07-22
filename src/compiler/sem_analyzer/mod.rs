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

pub mod ann_ast;
mod core;
mod symbol;
mod scope;

use crate::compiler::sem_analyzer::symbol::{Symbol, SymbolKind};
use crate::compiler::sem_analyzer::scope::{Scope, ScopeId};
use crate::compiler::sem_analyzer::ann_ast;
use crate::compiler::parser::ast;
use crate::compiler::diagnostics::Diagnostics;

pub struct SemAnalyzer<'a> {
    ast: ast::Program,
    ann_ast: ann_ast::Program,
    symbols: Vec<Symbol>,
    scopes: Vec<Scope>,
    current_scope: ScopeId,

    diagnostics: &'a mut Diagnostics,
}