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
    synthesis::Synthesis,
    symbol::{Symbol, SymbolId},
    ast::{Program, Item, Ident},
    diagnostics::{Diagnostics, Diagnostic},
    compiled_rel::CompiledRel,
    netlist::Netlist,
};

pub enum CompileError {
    Io(std::io::Error),
    InvalidFileExtension,
    Diagnostics(Diagnostics),
}

impl Compiler {
    // return netlist and vector of compiled relations or panic
    pub fn compile(file_path: &str, top_net: &str) -> Result<(Netlist, Vec<CompiledRel>), CompileError> {
        let path = PathBuf::from(file_path);

        if path.extension().and_then(|ext| ext.to_str()) != Some("loi") {
            return Err(CompileError::InvalidFileExtension);
        }

        let code = match fs::read_to_string(&path) {
            Ok(code) => code,
            Err(err) => return Err(CompileError::Io(err)),
        };

        let mut diagnostics = Diagnostics::new();

        let (ast, mut symbols) = Self::front_end(&code, &mut diagnostics);

        Self::back_end(ast, &mut symbols, top_net, diagnostics)
    }

    fn front_end(code: &str, diagnostics: &mut Diagnostics) -> (Program, Vec<Symbol>) {
        let tokens = Lexer::new(code, diagnostics).tokenize();
        let program = Parser::new(tokens, diagnostics).parse();

        SemAnalyzer::new(program, diagnostics).analyze()
    }

    fn back_end(ast: Program, symbols: &mut [Symbol], top_net: &str, mut diagnostics: Diagnostics) -> Result<(Netlist, Vec<CompiledRel>), CompileError> {
        let mut compiled_relations = Vec::new();
        let mut obj_map = HashMap::<SymbolId, usize>::new();
        let mut nets = Vec::new();

        let mut top_net_idx_option: Option<usize> = None;
        for item in ast.items {
            match item {
                Item::Rel(relation) => {
                    let idx = compiled_relations.len();

                    let Ident::Symbol(id) = relation.name else {
                        unreachable!("relation name must be ident::symbol by this point"); // unreachable
                    };

                    let Some(compiled_relation) = CodeGen::compile(relation, symbols, &mut diagnostics) else {
                        continue;
                    };

                    compiled_relations.push(compiled_relation);
                    obj_map.insert(id, idx);
                }

                Item::Net(net) => {
                    let idx = nets.len();

                    let Ident::Symbol(id) = net.name else {
                        unreachable!("net name must be ident::symbol by this point");
                    };
                    
                    if symbols[id].name == top_net {
                        top_net_idx_option = Some(id);
                    }

                    nets.push(net);
                    obj_map.insert(id, idx);
                },

                _ => {}
            }
        }

        let Some(top_net_idx) = top_net_idx_option else {
            diagnostics.error(Diagnostic::NonexistantTopLevelNet {
                name: top_net.to_string(),
            });

            return Err(CompileError::Diagnostics(diagnostics));
        };

        let netlist = Synthesis::synthesize(nets, top_net_idx, obj_map, symbols, &mut diagnostics);

        if diagnostics.has_errors() {
            return Err(CompileError::Diagnostics(diagnostics));
        }

        Ok((netlist, compiled_relations))
    }
}


// see tests/compiler for integration tests