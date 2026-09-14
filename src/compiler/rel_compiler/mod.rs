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
mod intermediate_rep;
mod expr_compiler;
mod lower_ir;

use std::collections::HashMap;

use crate::compiler::symbol::{Symbol, SymbolId};
use crate::compiler::diagnostics::{Diagnostics};

pub struct RelCompiler<'a> {
    reg_map: HashMap<SymbolId, usize>,
    reg_used: [bool; 64],

    rel_symbol_id: SymbolId,
    symbol_table: &'a [Symbol],
    diagnostics: &'a mut Diagnostics,
}