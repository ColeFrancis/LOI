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

use loi::compiler::{
    Compiler,
    compile_error::CompileError,
    netlist::{Netlist, Relation, Entity},
    compiled_rel::CompiledRel,
};

use loi::simulator::rel_interpreter::test_assembler::assemble;
use loi::simulator::event::Event;

// To test:
//  missing file
//  file without .loi extension
//  working file with (mostly) all of the language features
//  file with (mostly) all of diagnostics errors

#[test]
fn missing_file() {
    let path = format!("{}/tests/fixtures/this_does_not_exist.loi", env!("CARGO_MANIFEST_DIR"));
    println!("path: {}", path);

    let result = Compiler::compile(&path, "CALC");

    assert!(matches!(
        result,
        Err(CompileError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound
    ));
}

#[test]
fn invalid_file_extension() {
    let path = format!("{}/tests/fixtures/not_loi.txt", env!("CARGO_MANIFEST_DIR"));
    println!("path: {}", path);

    let result = Compiler::compile(&path, "CALC");

    assert_eq!(result, Err(CompileError::InvalidFileExtension));
}

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
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![0, 3],
                    output_ent: 11,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![0, 11],
                    output_ent: 12,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![11, 3],
                    output_ent: 13,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![12, 13],
                    output_ent: 10,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![1, 3],
                    output_ent: 15,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![1, 15],
                    output_ent: 16,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![15, 3],
                    output_ent: 17,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![16, 17],
                    output_ent: 14,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![6, 3],
                    output_ent: 19,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![6, 19],
                    output_ent: 20,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![19, 3],
                    output_ent: 21
                }, 
                Relation { // 11
                    idx: 0,
                    delay: 1,
                    input_ents: vec![20, 21],
                    output_ent: 18
                }, 
                Relation { // 12
                    idx: 0,
                    delay: 1,
                    input_ents: vec![23, 24],
                    output_ent: 22,
                }, 
                Relation { // 13
                    idx: 0,
                    delay: 1,
                    input_ents: vec![4, 10],
                    output_ent: 23,
                }, 
                Relation { // 14
                    idx: 0,
                    delay: 1,
                    input_ents: vec![4, 23],
                    output_ent: 26,
                }, 
                Relation { // 15
                    idx: 0,
                    delay: 1,
                    input_ents: vec![23, 10],
                    output_ent: 27,
                }, 
                Relation { // 16
                    idx: 0,
                    delay: 1,
                    input_ents: vec![26, 27],
                    output_ent: 25,
                }, 
                Relation { // 17
                    idx: 0,
                    delay: 1,
                    input_ents: vec![18, 25],
                    output_ent: 24,
                }, 
                Relation { // 18
                    idx: 0,
                    delay: 1,
                    input_ents: vec![18, 24],
                    output_ent: 28,
                }, 
                Relation { // 19
                    idx: 0,
                    delay: 1,
                    input_ents: vec![24, 25],
                    output_ent: 29,
                }, 
                Relation { // 20
                    idx: 0,
                    delay: 1,
                    input_ents: vec![28, 29],
                    output_ent: 7,
                }, 
                Relation { // 21
                    idx: 0,
                    delay: 1,
                    input_ents: vec![30, 31],
                    output_ent: 9,
                }, 
                Relation { // 22
                    idx: 0,
                    delay: 1,
                    input_ents: vec![5, 14],
                    output_ent: 30,
                }, 
                Relation { // 23
                    idx: 0,
                    delay: 1,
                    input_ents: vec![5, 30],
                    output_ent: 33
                }, 
                Relation { // 24
                    idx: 0,
                    delay: 1,
                    input_ents: vec![30, 14],
                    output_ent: 34
                }, 
                Relation { // 25
                    idx: 0,
                    delay: 1,
                    input_ents: vec![33, 34],
                    output_ent: 32,
                }, 
                Relation { // 26
                    idx: 0,
                    delay: 1,
                    input_ents: vec![22, 32],
                    output_ent: 31,
                }, 
                Relation { // 27
                    idx: 0,
                    delay: 1,
                    input_ents: vec![22, 31],
                    output_ent: 35,
                }, 
                Relation { // 28
                    idx: 0,
                    delay: 1,
                    input_ents: vec![31, 32],
                    output_ent: 36,
                }, 
                Relation { // 29
                    idx: 0,
                    delay: 1,
                    input_ents: vec![35, 36],
                    output_ent: 8,
                }, 
                Relation { // 30
                    idx: 0,
                    delay: 1,
                    input_ents: vec![2, 2],
                    output_ent: 38,
                }, 
                Relation { // 31 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![38, 38],
                    output_ent: 39,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![7, 7],
                    output_ent: 42,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![7, 38],
                    output_ent: 43,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![42, 38],
                    output_ent: 44,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![43, 41],
                    output_ent: 40,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![44, 40],
                    output_ent: 41,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![40, 40],
                    output_ent: 45,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![40, 39],
                    output_ent: 46,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![45, 39],
                    output_ent: 47,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![46, 37],
                    output_ent: 4,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![47, 4],
                    output_ent: 37,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![2, 2],
                    output_ent: 49,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![49, 49],
                    output_ent: 50,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![8, 8],
                    output_ent: 53,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![8, 49],
                    output_ent: 54,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![53, 49],
                    output_ent: 55,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![54, 52],
                    output_ent: 51,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![55, 51],
                    output_ent: 52,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![51, 51],
                    output_ent: 56,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![51, 50],
                    output_ent: 57,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![56, 50],
                    output_ent: 58,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![57, 48],
                    output_ent: 5,
                }, 
                Relation { 
                    idx: 0,
                    delay: 1,
                    input_ents: vec![58, 5],
                    output_ent: 48,
                }, 
            ],
            ents: vec![
                Entity { // 0
                    val: None,
                    sinks: vec![0, 1],
                },
                Entity { // 1
                    val: None,
                    sinks: vec![4, 5],
                },
                Entity { // 2
                    val: None,
                    sinks: vec![30, 42],
                },
                Entity { // 3
                    val: None,
                    sinks: vec![0, 2, 4, 6, 8, 10],
                },
                Entity { // 4
                    val: None,
                    sinks: vec![13, 14, 41],
                },
                Entity { // 5
                    val: None,
                    sinks: vec![22, 23, 53],
                },
                Entity { // 6
                    val: None,
                    sinks: vec![8, 9],
                },
                Entity { // 7
                    val: None,
                    sinks: vec![32, 33],
                },
                Entity { // 8
                    val: None,
                    sinks: vec![44, 45],
                },
                Entity { // 9
                    val: None,
                    sinks: vec![],
                },
                Entity { // 10
                    val: None,
                    sinks: vec![13, 15],
                },
                Entity { // 11
                    val: None,
                    sinks: vec![1, 2],
                },
                Entity { // 12
                    val: None,
                    sinks: vec![3],
                },
                Entity { // 13
                    val: None,
                    sinks: vec![3],
                },
                Entity { // 14
                    val: None,
                    sinks: vec![22, 24],
                },
                Entity { // 15
                    val: None,
                    sinks: vec![5, 6],
                },
                Entity { // 16
                    val: None,
                    sinks: vec![7],
                },
                Entity { // 17
                    val: None,
                    sinks: vec![7],
                },
                Entity { // 18
                    val: None,
                    sinks: vec![17, 18],
                },
                Entity { // 19
                    val: None,
                    sinks: vec![9, 10],
                },
                Entity { // 20
                    val: None,
                    sinks: vec![11],
                },
                Entity { // 21
                    val: None,
                    sinks: vec![11],
                },
                Entity { // 22
                    val: None,
                    sinks: vec![26, 27],
                },
                Entity { // 23
                    val: None,
                    sinks: vec![12, 14, 15],
                },
                Entity { // 24
                    val: None,
                    sinks: vec![12, 18, 19],
                },
                Entity { // 25
                    val: None,
                    sinks: vec![17, 19],
                },
                Entity { // 26
                    val: None,
                    sinks: vec![16],
                },
                Entity { // 27
                    val: None,
                    sinks: vec![16],
                },
                Entity { // 28
                    val: None,
                    sinks: vec![20],
                },
                Entity { // 29
                    val: None,
                    sinks: vec![20],
                },
                Entity { // 30
                    val: None,
                    sinks: vec![21, 23, 24],
                },
                Entity { // 31
                    val: None,
                    sinks: vec![21, 27, 28],
                },
                Entity { // 32
                    val: None,
                    sinks: vec![26, 28],
                },
                Entity { // 33
                    val: None,
                    sinks: vec![25],
                },
                Entity { // 34
                    val: None,
                    sinks: vec![25],
                },
                Entity { // 35
                    val: None,
                    sinks: vec![29],
                },
                Entity { // 36
                    val: None,
                    sinks: vec![29],
                },
                Entity { // 37
                    val: None,
                    sinks: vec![40],
                },
                Entity { // 38
                    val: None,
                    sinks: vec![31, 33, 34],
                },
                Entity { // 39
                    val: None,
                    sinks: vec![38, 39],
                },
                Entity { // 40
                    val: None,
                    sinks: vec![36, 37, 38],
                },
                Entity { // 41
                    val: None,
                    sinks: vec![35],
                },
                Entity { // 42
                    val: None,
                    sinks: vec![34],
                },
                Entity { // 43
                    val: None,
                    sinks: vec![35],
                },
                Entity { // 44
                    val: None,
                    sinks: vec![36],
                },
                Entity { // 45
                    val: None,
                    sinks: vec![39],
                },
                Entity { // 46
                    val: None,
                    sinks: vec![40],
                },
                Entity { // 47
                    val: None,
                    sinks: vec![41],
                },
                Entity { // 48
                    val: None,
                    sinks: vec![52],
                },
                Entity { // 49
                    val: None,
                    sinks: vec![43,45,46],
                },
                Entity { // 50
                    val: None,
                    sinks: vec![50, 51],
                },
                Entity { // 52
                    val: None,
                    sinks: vec![48,49,50],
                },
                Entity { // 53
                    val: None,
                    sinks: vec![47],
                },
                Entity { // 54
                    val: None,
                    sinks: vec![46],
                },
                Entity { // 55
                    val: None,
                    sinks: vec![47],
                },
                Entity { // 56
                    val: None,
                    sinks: vec![48],
                },
                Entity { // 57
                    val: None,
                    sinks: vec![51],
                },
                Entity { // 58
                    val: None,
                    sinks: vec![52],
                },
                Entity { // 59
                    val: None,
                    sinks: vec![53],
                },
            ],
        }, 
        vec![
            CompiledRel {
                name: "NAND".to_string(),
                complexity: 0,
                bytecode: assemble("
                    AND r4 r2 r3
                    NOT r4 r4
                    RET r4
                ").unwrap(),
            }
        ],
        vec![
            Event {
                timestep: 0,
                entity: 4,
                new_val: 0,
            },
            Event {
                timestep: 0,
                entity: 5,
                new_val: 0,
            },
            Event {
                timestep: 0,
                entity: 6,
                new_val: 0,
            },
            Event {
                timestep: 0,
                entity: 37,
                new_val: 1,
            },
            Event {
                timestep: 0,
                entity: 40,
                new_val: 0,
            },
            Event {
                timestep: 0,
                entity: 41,
                new_val: 1,
            },
            Event {
                timestep: 0,
                entity: 48,
                new_val: 1,
            },
            Event {
                timestep: 0,
                entity: 51,
                new_val: 0,
            },
            Event {
                timestep: 0,
                entity: 52,
                new_val: 1,
            },
        ],
    )));
}