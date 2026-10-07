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

mod check_constraints;
mod check_expr;
mod check_types;
mod core;
mod fold_const;
mod fold_expr;
mod resolve_expr;
mod resolve_names;
mod scope;

use crate::compiler::error_handling::diagnostics::Diagnostics;
use crate::compiler::objects::ast::Program;
use crate::compiler::objects::symbol::Symbol;
use crate::compiler::sem_analyzer::scope::Scope;

pub struct SemAnalyzer<'a> {
    ast: Program,
    symbols: Vec<Symbol>,
    scopes: Vec<Scope>,

    diagnostics: &'a mut Diagnostics,
}
