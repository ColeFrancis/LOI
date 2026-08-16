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

use std::collections::HashMap;

use super::types::Type;
use crate::compiler::parser::ast::Literal;
use crate::compiler::diagnostics::Span;

pub type SymbolId = usize;

#[derive(PartialEq, Debug)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,

    pub span: Span,
}

#[derive(PartialEq, Debug, Clone)]
#[allow(non_camel_case_types)]
pub enum SymbolKind {
    Const(Literal),
    Variable(Type),
    EntType,
    EntMember(SymbolId), // Parent
    Ent(Type),
    Rel_t {
        input_types: Vec<Type>,
        return_type: Type
    },
    Net {
       ports: HashMap<String, NetPort>
    },
}

#[derive(PartialEq, Debug, Clone)]
pub struct NetPort {
    pub symbol: SymbolId,
    pub ty: Type,
}