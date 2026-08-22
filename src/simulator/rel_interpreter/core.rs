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

//! # interpreter
//!
//! Used to execute relations during simulation
//!
//! ## Invariants
//!
//! - bytecode match description in bytecode_design.txt
//!
//! Author: Cole Francis

use super::RelInterpreter;

impl RelInterpreter {
    pub fn new(bytecode: Vec<Vec<u8>>) -> Self {
        Self {
            bytecode,
            registers: [0; 64],
        }
    }

    pub fn evaluate (&mut self, relation_id: usize, args: Vec<u64>, sim_timestep: usize, rel_delay: usize) -> u64 {
        // Fill time info and arguments
        self.registers[0] = sim_timestep as u64;
        self.registers[1] = rel_delay as u64;
        self.registers[2..args.len() + 2].copy_from_slice(&args);
        
        let code = &self.bytecode[relation_id];
        Self::execute(&mut self.registers, code)
    }

    fn execute(registers: &mut [u64; 64], bytecode: &[u8]) -> u64 {
        0
    }
}