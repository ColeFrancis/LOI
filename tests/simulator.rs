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

use loi::compiler::Compiler;

use loi::simulator::{Simulator, IoVal, io_file::IoFile};

#[test]
fn flip_flop() {
    let path = format!("{}/tests/fixtures/binary_adder.loi", env!("CARGO_MANIFEST_DIR"));
    println!("path: {}", path);

    let result = Compiler::compile(&path, "d_FLIP_FLOP");

    assert!(result.is_ok(), "Compilation failed: {:?}", result);

    let Ok((netlist, interface, relations, inits)) = result else {
        unreachable!();
    };

    let mut sim = Simulator::new(netlist, interface, relations, inits);

    let inputs = vec![
        ("D".to_string(), vec![
            (0, IoVal::Bool(false)),
            (11, IoVal::Bool(true)),
            (31, IoVal::Bool(false)),
        ]),
        ("clk".to_string(), vec![
            (10, IoVal::Bool(true)),
            (15, IoVal::Bool(false)),
            (20, IoVal::Bool(true)),
            (25, IoVal::Bool(false)),
            (30, IoVal::Bool(true)),
            (35, IoVal::Bool(false)),
            (40, IoVal::Bool(true)),
            (45, IoVal::Bool(false)),
        ]),
    ];

    let success = sim.load_inputs(inputs);

    assert_eq!(success, Ok(()));

    let _steps = sim.run(256);

    let output = sim.dump_outputs();

    assert_eq!(output, vec![
        ("Q".to_string(), vec![
            (0, IoVal::Bool(false)),
            (24, IoVal::Bool(true)),
            (45, IoVal::Bool(false)),
        ]),
        ("Qp".to_string(), vec![
            (0, IoVal::Bool(true)),
            (25, IoVal::Bool(false)),
            (44, IoVal::Bool(true)),
        ]),
    ]);
}

#[test]
fn adder() {
    let path = format!("{}/tests/fixtures/binary_adder.loi", env!("CARGO_MANIFEST_DIR"));
    println!("path: {}", path);

    let result = Compiler::compile(&path, "ADDER");

    assert!(result.is_ok(), "Compilation failed: {:?}", result);

    let Ok((netlist, interface, relations, inits)) = result else {
        unreachable!();
    };

    let mut sim = Simulator::new(netlist, interface, relations, inits);

    let inputs = vec![
        ("cin".to_string(), vec![
            (0, IoVal::Bool(false)),
            (30, IoVal::Bool(true)),
        ]),
        ("A".to_string(), vec![
            (0, IoVal::Bool(false)),
            (10, IoVal::Bool(true)),
            (40, IoVal::Bool(false)),
        ]),
        ("B".to_string(), vec![
            (0, IoVal::Bool(false)),
            (20, IoVal::Bool(true)),
        ]),
    ];

    let success = sim.load_inputs(inputs);

    assert_eq!(success, Ok(()));

    let _steps = sim.run(256);

    let output = sim.dump_outputs();

    assert_eq!(output, vec![
        ("S".to_string(), vec![
            (6, IoVal::Bool(false)),
            (14, IoVal::Bool(true)),
            (25, IoVal::Bool(false)),
            (32, IoVal::Bool(true)),
            (46, IoVal::Bool(false)),
        ]),
        ("cout".to_string(), vec![
            (5, IoVal::Bool(false)),
            (22, IoVal::Bool(true)),
            (42, IoVal::Bool(false)),
            (45, IoVal::Bool(true)),
        ]),
    ]);
}

#[test]
fn impulse_and_custom() {
    let loi_path = format!("{}/tests/fixtures/custom_type_latch.loi", env!("CARGO_MANIFEST_DIR"));

    let result = Compiler::compile(&loi_path, "LATCH");

    assert!(result.is_ok(), "Compilation failed: {:?}", result);

    let Ok((netlist, interface, relations, inits)) = result else {
        unreachable!();
    };

    let mut sim = Simulator::new(netlist, interface, relations, inits);

    let input_path = format!("{}/tests/fixtures/latch_inputs.txt", env!("CARGO_MANIFEST_DIR"));

    let inputs = IoFile::read(&input_path).unwrap();

    let success = sim.load_inputs(inputs);

    assert_eq!(success, Ok(()));

    let _steps = sim.run(256);

    let output = sim.dump_outputs();

    assert_eq!(output, vec![
        ("out".to_string(), vec![
            (0, IoVal::Custom("A".to_string())),
            (3, IoVal::Custom("B".to_string())),
            (9, IoVal::Custom("A".to_string())),
        ]),
    ]);
}