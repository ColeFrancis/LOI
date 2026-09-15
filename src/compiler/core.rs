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
//! compiles code to a netlist and bytecode
//!
//! ## Invariants
//!
//! Author: Cole Francis

use super::Compiler;
use crate::compiler::{
    lexer::Lexer,
    parser::Parser,
    sem_analyzer::SemAnalyzer,
    symbol::Symbol,
    ast::Program,
    diagnostics::Diagnostics,
};

impl Compiler {
    // pub fn compile(file_path: &str, <args>) {

    // }

    fn front_end(code: &str, diagnostics: &mut Diagnostics) -> (Program, Vec<Symbol>) {
        let tokens = Lexer::new(code, diagnostics).tokenize();
        let program = Parser::new(tokens, diagnostics).parse();

        SemAnalyzer::new(program, diagnostics).analyze()
    }
}