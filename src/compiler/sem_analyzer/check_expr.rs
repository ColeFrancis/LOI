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

//! # check_expr
//!
//! Handles checking and assigning of types for annotated ast
//!
//! ## Invariants
//!
//! - All incoming expressions will have Type::Unknown
//!
//! Author: Cole Francis

use super::SemAnalyzer;
use super::types::Type;
use crate::compiler::parser::ast::*;
use crate::compiler::diagnostics::{CompilerError, Span};

impl <'a> SemAnalyzer<'a> {
    fn check_expr(&mut self, mut expr: Expr) -> Option<Expr> {
        Some(expr)
    }
}