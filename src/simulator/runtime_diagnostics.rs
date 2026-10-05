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

use crate::compiler::netlist::{EntId, RelId};
use crate::compiler::sem_analyzer::types::Type;

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
    InputFileRead(InputFileReadError),
    OutputFileWrite(OutputFileWriteError),
}

#[derive(Debug, PartialEq)]
pub enum InputFileReadError {
    InvalidInputFileType,
    Io(IoError),
    InvalidHeader,
    IncorrectNumberOfFields {
        expected: usize,
        found: usize,
        line_num: usize,
    },
    InvalidStep {
        source: std::num::ParseIntError,
        line_num: usize,
    },
    InvalidNumber {
        num: String,
        line_num: usize,
    },
}

#[derive(Debug, PartialEq)]
pub enum OutputFileWriteError {
    InvalidOutputFileType,
    Io(IoError),
}

#[derive(Debug)]
pub struct IoError(pub std::io::Error);

impl PartialEq for IoError {
    fn eq(&self, other: &Self) -> bool {
        self.0.kind() == other.0.kind()
    }
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

impl RuntimeError {
    pub fn print(&self) {
        match self {
            RuntimeError::Interpreter {err, rel_name, rel_id, timestep} => err.print(rel_name, *rel_id, *timestep),

            RuntimeError::SimultaneousDrivers {ent_id, timestep} => {
                eprintln!("Runtime Error: Entity {} driven by conflicting values.\ntimestep: {}\n\n", ent_id, timestep);
            }

            RuntimeError::NonexistantInput(string) => {
                eprintln!("Runtime Error: Input entity {} does not exist.", string);
            }

            RuntimeError::NonexistantInputValue(string) => {
                eprintln!("Runtime Error: Input entity value {} does not exist.", string);
            }

            RuntimeError::IncompatibleTypes {ent_id, expected, found} => {
                eprintln!("Runtime Error: Given type {} incompatible with entity {}'s type: {}", found, ent_id, expected);
            }

            RuntimeError::InputFileRead(read_error) => read_error.print(),
 
            RuntimeError::OutputFileWrite(write_error) => write_error.print(),
        }
    }
}

impl InputFileReadError {
    pub fn print(&self) {
        match self {
            InputFileReadError::InvalidInputFileType => {
                eprintln!("Runtime Error: Invalid input file type.");
            }

            InputFileReadError::Io(io_error) => io_error.print(),

            InputFileReadError::InvalidHeader => {
                eprintln!("Runtime Error: Invalid input file header. First line should be: \"step entity value\" (Whitespace variable)");
            }

            InputFileReadError::IncorrectNumberOfFields {expected, found, line_num} => {
                eprintln!("Runtime Error: Incorrect number of fields in input file. Expected: {}, found: {}\nat line {}\n\n", expected, found, line_num);
            }

            InputFileReadError::InvalidStep {source, line_num} => {
                eprintln!("Runtime Error: Invalid step value in input file at line {}.\n{}\n\n", source, line_num);
            }

            InputFileReadError::InvalidNumber {num, line_num} => {
                eprintln!("Runtime Error: Invalid number {} in putput file.\nat line {}\n\n", num, line_num);
            }
        }
    }
}

impl OutputFileWriteError {
    pub fn print(&self) {
        match self {
            OutputFileWriteError::InvalidOutputFileType => {
                eprintln!("Runtime Error: Invalid output file type.");
            }

            OutputFileWriteError::Io(io_error) => io_error.print(),
        }
    }
}

impl IoError {
    pub fn print(&self) {
        eprintln!("I/O error accessing file: {}", self.0);
    }
}

impl InterpreterError {
    pub fn print(&self, rel_name: &str, rel_id: RelId, timestep: usize) {
        match self {
            InterpreterError::InvalidOpcode(code) => {
                // If the relation compiler is bug_free
                eprintln!("Runtime Error: Bug in relation compiler resulted in invalid opcode: {:b}.\nRelation \"{}\" (id {}) at timestep {}\n\n", code, rel_name, rel_id, timestep);
            }

            InterpreterError::IntegerOverflow => {
                eprintln!("Runtime Error: Integer overflow while executing.\nRelation \"{}\" (id {}) at timestep {}\n\n", rel_name, rel_id, timestep);
            }

            InterpreterError::DivisionByZero => {
                eprintln!("Runtime Error: Division by zero while executing.\nRelation \"{}\" (id {}) at timestep {}\n\n", rel_name, rel_id, timestep);
            }

            InterpreterError::IntNegativeExponent => {
                eprintln!("Runtime Error: Integer raised to negative power.\nRelation \"{}\" (id {}) at timestep {}\n\n", rel_name, rel_id, timestep);
            }

            InterpreterError::InvalidProb(prob) => {
                eprintln!("Runtime Error: Probability value \"{}\" out of range. Should be 1.\nrelation \"{}\" (id {}) at timestep {}\n\n", prob, rel_name, rel_id, timestep);
            }
        }
    }
}