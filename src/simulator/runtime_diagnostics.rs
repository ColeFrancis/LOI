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

#[derive(Debug, PartialEq)]
pub enum RuntimeError {
    InvalidOpcode(u8),
    IntegerOverflow,
    DivisionByZero,
    IntNegativeExponent, // For integers
    InvalidProb(f64),
}

impl RuntimeError {
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