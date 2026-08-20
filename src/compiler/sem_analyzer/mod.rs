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

mod core;
pub mod types;
mod scope;
mod resolve_names;
mod resolve_expr;
mod check_types;
mod check_expr;
mod fold_const;
mod fold_expr;

use crate::compiler::symbol::Symbol;
use crate::compiler::sem_analyzer::scope::Scope;
use crate::compiler::diagnostics::Diagnostics;
use crate::compiler::ast::Program;

pub struct SemAnalyzer<'a> {
    ast: Program,
    symbols: Vec<Symbol>,
    scopes: Vec<Scope>,

    diagnostics: &'a mut Diagnostics,
}