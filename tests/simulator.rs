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

use loi::simulator::{Simulator, IO_VAL};

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
            (0, IO_VAL::Bool(false)),
            (11, IO_VAL::Bool(true)),
            (31, IO_VAL::Bool(false)),
        ]),
        ("clk".to_string(), vec![
            (10, IO_VAL::Bool(true)),
            (15, IO_VAL::Bool(false)),
            (20, IO_VAL::Bool(true)),
            (25, IO_VAL::Bool(false)),
            (30, IO_VAL::Bool(true)),
            (35, IO_VAL::Bool(false)),
            (40, IO_VAL::Bool(true)),
            (45, IO_VAL::Bool(false)),
        ]),
    ];

    let success = sim.load_inputs(inputs);

    assert_eq!(success, Ok(()));

    let _steps = sim.run(256);

    let output = sim.dump_outputs();

    assert_eq!(output, vec![
        ("Q".to_string(), vec![
            (0, IO_VAL::Bool(false)),
            (24, IO_VAL::Bool(true)),
            (45, IO_VAL::Bool(false)),
        ]),
        ("Qp".to_string(), vec![
            (0, IO_VAL::Bool(true)),
            (25, IO_VAL::Bool(false)),
            (44, IO_VAL::Bool(true)),
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
            (0, IO_VAL::Bool(false)),
            (30, IO_VAL::Bool(true)),
        ]),
        ("A".to_string(), vec![
            (0, IO_VAL::Bool(false)),
            (10, IO_VAL::Bool(true)),
            (40, IO_VAL::Bool(false)),
        ]),
        ("B".to_string(), vec![
            (0, IO_VAL::Bool(false)),
            (20, IO_VAL::Bool(true)),
        ]),
    ];

    let success = sim.load_inputs(inputs);

    assert_eq!(success, Ok(()));

    let _steps = sim.run(256);

    let output = sim.dump_outputs();

    assert_eq!(output, vec![
        ("S".to_string(), vec![
            (6, IO_VAL::Bool(false)),
            (14, IO_VAL::Bool(true)),
            (25, IO_VAL::Bool(false)),
            (32, IO_VAL::Bool(true)),
            (46, IO_VAL::Bool(false)),
        ]),
        ("cout".to_string(), vec![
            (5, IO_VAL::Bool(false)),
            (22, IO_VAL::Bool(true)),
            (42, IO_VAL::Bool(false)),
            (45, IO_VAL::Bool(true)),
        ]),
    ]);
}

// TODO: tests with custom ent types, impulses, and mods