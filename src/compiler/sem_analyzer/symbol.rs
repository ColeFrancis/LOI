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

//! # symbol
//!
//! Holds the structures used in creating the symbol table
//!
//! ## Invariants
//!
//! - 
//!
//! Author: Cole Francis

use crate::compiler::diagnostics::Span;

pub type SymbolId = usize;

pub enum SymbolKind {
    Variable,
    Parameter,
    Ent_t,
    Rel_t,
    Net,
}

struct Symbol {
    id: SymbolId,   // index in Vec<Symbol>
    name: String,
    kind: SymbolKind,

    span: Span,

    // filled in by later passes
    // ty: Option<TypeId>
}