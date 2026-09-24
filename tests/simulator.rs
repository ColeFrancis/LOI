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

use loi::simulator::Simulator;

#[test]
fn flip_flop() {
    let path = format!("{}/tests/fixtures/binary_adder.loi", env!("CARGO_MANIFEST_DIR"));
    println!("path: {}", path);

    let result = Compiler::compile(&path, "d_FLIP_FLOP");

    assert!(result.is_ok(), "Compilation failed: {:?}", result);

    let Ok((netlist, relations, inits)) = result else {
        unreachable!();
    };

    let mut sim = Simulator::new(netlist, relations, inits);

    let inputs = vec![
        ("D".to_string(), vec![
            (0, 0),
            (11, 1),
            (31, 0),
        ]),
        ("clk".to_string(), vec![
            (10, 1),
            (15, 0),
            (20, 1),
            (25, 0),
            (30, 1),
            (35, 0),
            (40, 1),
            (45, 0),
        ]),
    ];

    let success = sim.load_inputs(inputs);

    assert_eq!(success, Ok(()));

    let _steps = sim.run(256);

    let output = sim.dump_outputs();

    assert_eq!(output, vec![
        ("Q".to_string(), vec![
            (0, 0),
            (24, 1),
            (45, 0),
        ]),
        ("Qp".to_string(), vec![
            (0, 1),
            (25, 0),
            (44, 1),
        ]),
    ]);
}

#[test]
fn adder() {
    let path = format!("{}/tests/fixtures/binary_adder.loi", env!("CARGO_MANIFEST_DIR"));
    println!("path: {}", path);

    let result = Compiler::compile(&path, "ADDER");

    assert!(result.is_ok(), "Compilation failed: {:?}", result);

    let Ok((netlist, relations, inits)) = result else {
        unreachable!();
    };

    let mut sim = Simulator::new(netlist, relations, inits);

    let inputs = vec![
        ("cin".to_string(), vec![
            (0, 0),
            (30, 1),
        ]),
        ("A".to_string(), vec![
            (0, 0),
            (10, 1),
            (40, 0),
        ]),
        ("B".to_string(), vec![
            (0, 0),
            (20, 1),
        ]),
    ];

    let success = sim.load_inputs(inputs);

    assert_eq!(success, Ok(()));

    let _steps = sim.run(256);

    let output = sim.dump_outputs();

    assert_eq!(output, vec![
        ("S".to_string(), vec![
            (6, 0),
            (14, 1),
            (25, 0),
            (32, 1),
            (46, 0),
        ]),
        ("cout".to_string(), vec![
            (5, 0),
            (22, 1),
            (42, 0),
            (45, 1),
        ]),
    ]);
}