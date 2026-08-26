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
//! Handles the compilation of relations into bytecode
//!
//! ## Invariants
//!
//! - 
//!
//! Author: Cole Francis

use super::CodeGen;
use crate::compiler::ast::RelType;
use crate::compiler::compiled_rel::CompiledRel;

impl CodeGen {
    pub fn new(relations: Vec<RelType>) -> Self {
        Self {
            relations,
        }
    }

    pub fn compile(mut self) -> Vec<CompiledRel> {
        // after optimization, the last step should be to turn x^2 back to x*x and 2x back to x+x
        // also turn (-x) + a (a is literal) back to a - x
        Vec::new()
    }
}