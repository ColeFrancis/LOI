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

//! # runtime_diagnostics
//!
//! Handles runtime errors in the simulator
//!
//! Author: Cole Francis

use crate::compiler::objects::{
    types::Type,
    netlist::{EntId, RelId},
};

#[derive(Debug, PartialEq)]
pub enum RuntimeError {
    Interpreter {
        err: InterpreterError,
        rel_name: String,
        rel_id: RelId,
        timestep: usize,
    },
    SimultaneousDrivers {
        ent_id: EntId,
        timestep: usize,
    },
    NonexistantInput(String),
    NonexistantInputValue(String),
    IncompatibleTypes {
        ent_id: EntId,
        expected: Type,
        found: Type,
    },
}

#[derive(Debug, PartialEq)]
pub enum InterpreterError {
    InvalidOpcode(u8),
    IntegerOverflow,
    DivisionByZero,
    IntNegativeExponent, // For integers
    InvalidProb(f64),
}

impl InterpreterError {
    pub fn from_index(index: usize, info: Option<u64>) -> Self {
        match index {
            0 => Self::InvalidOpcode(
                info.expect("InvalidOpcode requires info") as u8
            ),
            1 => Self::IntegerOverflow,
            2 => Self::DivisionByZero,
            3 => Self::IntNegativeExponent,
            4 => Self::InvalidProb(
                f64::from_bits(info.expect("InvalidProb reqires info")),
            ),
            _ => panic!("Invalid runtime error code: {index}"),
        }
    }
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::Interpreter {err, rel_name, rel_id, timestep} => {
                write!(f, "Runtime Error: {} while executing relation \"{}\" (id {}) at timestep {}", err, rel_name, rel_id, timestep)
            }

            RuntimeError::SimultaneousDrivers {ent_id, timestep} => {
                write!(f, "Runtime Error: Entity {} driven by conflicting values.\ntimestep: {}", ent_id, timestep)
            }

            RuntimeError::NonexistantInput(string) => {
                write!(f, "Runtime Error: Input entity {} does not exist.", string)
            }

            RuntimeError::NonexistantInputValue(string) => {
                write!(f, "Runtime Error: Input entity value {} does not exist.", string)
            }

            RuntimeError::IncompatibleTypes {ent_id, expected, found} => {
                write!(f, "Runtime Error: Given type {} incompatible with entity {}'s type: {}", found, ent_id, expected)
            }
        }
    }
}

impl std::fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterpreterError::InvalidOpcode(code) => {
                // If the relation compiler is bug_free
                write!(f, "Bug in relation compiler resulted in invalid opcode {}", code)
            }

            InterpreterError::IntegerOverflow => {
                write!(f, "Integer overflow")
            }

            InterpreterError::DivisionByZero => {
                write!(f, "Division by zero")
            }

            InterpreterError::IntNegativeExponent => {
                write!(f, "Integer raised to negative power")
            }

            InterpreterError::InvalidProb(prob) => {
                write!(f, "Probability value \"{}\" out of range (Should be 1)", prob)
            }
        }
    }
}