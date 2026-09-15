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

use std::fs;
use std::path::PathBuf;

use std::collections::HashMap;

use super::Compiler;
use crate::compiler::{
    lexer::Lexer,
    parser::Parser,
    sem_analyzer::SemAnalyzer,
    code_gen::CodeGen,
    symbol::{Symbol, SymbolId},
    ast::{Program, Item, Ident},
    diagnostics::Diagnostics,
    compiled_rel::CompiledRel,
    netlist::Netlist,
};

pub enum CompileError {
    Io(std::io::Error),
    Diagnostics(Diagnostics),
}

impl Compiler {
    // return netlist and vector of compiled relations or panic
    pub fn compile(file_path: &str, top_net: &str) -> Result<(Netlist, Vec<CompiledRel>), CompileError> {
        let path = PathBuf::from(file_path);
        let code = match fs::read_to_string(&path) {
            Ok(code) => code,
            Err(err) => return Err(CompileError::Io(err)),
        };

        let mut diagnostics = Diagnostics::new();

        let (ast, symbols) = Self::front_end(&code,&mut diagnostics);

        let mut compiled_relations = Vec::new();
        let mut rel_map = HashMap::<SymbolId, usize>::new();
        let mut nets = Vec::new();

        for item in ast.items {
            match item {
                Item::Rel(relation) => {
                    let Ident::Symbol(id) = relation.name else {
                        unreachable!("relation name must be ident::symbol by this point"); // unreachable
                    };

                    let Some(compiled_relation) = CodeGen::compile(relation, &symbols, &mut diagnostics) else {
                        continue;
                    }

                    let idx = compiled_relations.len();

                    compiled_relations.push(compiled_relation);
                    rel_map.insert(id, idx);
                }

                Item::Net(net) => nets.push(net),

                _ => {}
            }
        }

        // syntehsyze nets here or after returning error

        if diagnostics.has_errors() {
            return Err(CompileError::Diagnostics(diagnostics));
        }

        // TODO: return syntehsyzed netlists after its implemented
        Ok((Netlist {
            relations: vec![],
            ents: vec![],
        }, compiled_relations))
    }

    fn front_end(code: &str, diagnostics: &mut Diagnostics) -> (Program, Vec<Symbol>) {
        let tokens = Lexer::new(code, diagnostics).tokenize();
        let program = Parser::new(tokens, diagnostics).parse();

        SemAnalyzer::new(program, diagnostics).analyze()
    }
}