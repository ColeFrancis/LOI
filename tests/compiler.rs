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

//! # compiler
//!
//! Performs integration-level compiler tests
//!
//! Author: Cole Francis

use std::path::PathBuf;

use loi::compiler::{
    Compiler,
    netlist::{Netlist, Relation, Entity},
    compiled_rel::CompiledRel,
};

use loi::simulator::rel_interpreter::test_assembler::assemble;

// To test:
//  missing file
//  file without .loi extension
//  working file with (mostly) all of the language features
//  file with (mostly) all of diagnostics errors

#[test]
fn calculator() {
    let path = format!("{}/tests/fixtures/binary_adder.loi", env!("CARGO_MANIFEST_DIR"));
    println!("path: {}", path);

    let result = Compiler::compile(&path, "CALC");

    assert_eq!(result, Ok((
        Netlist {
            inputs: vec![
                ("I1".to_string(), 0),
                ("I2".to_string(), 1),
                ("clk".to_string(), 2),
                ("sub".to_string(), 3),
            ],
            outputs: vec![
                ("O1".to_string(), 4),
                ("O2".to_string(), 5),
            ],
            relations: vec![
                // Relation { // 2_BIT_ADD_SUB/XOR(0)/(0)
                //     idx: 0,
                //     input_ents: vec![0],
                //     output_ent: 
                // }, 
            ],
            ents: vec![
                Entity { // I1, 2_BIT_ADD_SUB/B1, 2_BIT_ADD_SUB/XOR(0)/A
                    val: None,
                    sinks: vec![0],
                },
                Entity { // I2, 2_BIT_ADD_SUB/B2
                    val: None,
                    sinks: vec![],
                },
                Entity { // clk, 2_BIT_REG/clk
                    val: None,
                    sinks: vec![],
                },
                Entity { // sub, 2_BIT_ADD_SUB/sub
                    val: None,
                    sinks: vec![],
                },
                Entity { // O1, 2_BIT_ADD_SUB/A1, 2_BIT_REG/R1
                    val: None,
                    sinks: vec![],
                },
                Entity { // O2, 2_BIT_ADD_SUB/A2, 2_BIT_REG/R2
                    val: None,
                    sinks: vec![],
                },
                Entity { // cin, 2_BIT_ADD_SUB/cin
                    val: Some(0),
                    sinks: vec![],
                },
                Entity { // S1, 2_BIT_ADD_SUB/S1, 2_BIT_REG/I1
                    val: None,
                    sinks: vec![],
                },
                Entity { // S1, 2_BIT_ADD_SUM/S2, 2_BIT_REG/I2
                    val: None,
                    sinks: vec![],
                },
                Entity { // 2_BIT_ADD_SUB/cout
                    val: None,
                    sinks: vec![],
                },
            ],
        }, 
        vec![
            CompiledRel {
                name: "NAND".to_string(),
                complexity: 0,
                bytecode: vec![],
            }
        ],
    )));
}