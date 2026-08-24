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
//! - All bytecode sequences end in a return instruction
//!
//! Author: Cole Francis

use super::RelInterpreter;

use crate::simulator::runtime_diagnostics::RuntimeError;

use crate::compiler::compiled_rel::CompiledRel;

impl RelInterpreter {
    pub fn new(relations: Vec<CompiledRel>) -> Self {
        Self {
            relations,
            registers: [0; 64],
        }
    }

    pub fn evaluate (&mut self, relation_id: usize, args: Vec<u64>, sim_timestep: usize, rel_delay: usize) -> u64 {
        // Fill time info and arguments
        self.registers[0] = sim_timestep as u64;
        self.registers[1] = rel_delay as u64;
        self.registers[2..args.len() + 2].copy_from_slice(&args);
        
        let code = &self.relations[relation_id].bytecode;
        
        Self::execute(&mut self.registers, code)
            .unwrap_or_else(|error| {
                // TODO: call a runtime_diagnostics report function
                //  error, relation (convert id to the string), sim_timestep

                panic!("Runtime diagnostics not yet implemented");
            })
    }

    fn execute(registers: &mut [u64; 64], bytecode: &[u8]) -> Result<u64, RuntimeError> {
        let mut inst_counter = 0;

        loop {
            let inst = bytecode[inst_counter];

            let src1_is_reg = (inst & 0b10) == 0;
            let src2_is_reg = (inst & 0b01) == 0;

            match inst & 0b11100000 {
                // Int arith
                0b00000000 => {
                    let op = inst & 0b00011100;

                    inst_counter += 1;

                    let dest_reg = bytecode[inst_counter] as usize;

                    inst_counter += 1;

                    // Handles incrementing inst_counter
                    let src1_val = Self::read_int_source(
                        registers,
                        bytecode,
                        &mut inst_counter,
                        src1_is_reg,
                    );

                    // Single source ABS inst
                    match op {
                        // ABS
                        0b00010100 => {
                            registers[dest_reg] = src1_val
                                .checked_abs()
                                .ok_or(RuntimeError::IntegerOverflow)? as u64;
                            continue;
                        }

                        _ => {}
                    }

                    // Handles incrementing inst_counter
                    let src2_val = Self::read_int_source(
                        registers,
                        bytecode,
                        &mut inst_counter,
                        src2_is_reg,
                    );

                    match op {
                        // ADD
                        0b00000000 => {
                            registers[dest_reg] = src1_val
                                .checked_add(src2_val)
                                .ok_or(RuntimeError::IntegerOverflow)? as u64;
                        }

                        // SUB
                        0b00000100 => {
                            registers[dest_reg] = src1_val
                                .checked_sub(src2_val)
                                .ok_or(RuntimeError::IntegerOverflow)? as u64;
                        }

                        // MUL
                        0b00001000 => {
                            registers[dest_reg] = src1_val
                                .checked_mul(src2_val)
                                .ok_or(RuntimeError::IntegerOverflow)? as u64;
                        }

                        // DIV
                        0b00001100 => {
                            registers[dest_reg] = src1_val
                                .checked_div(src2_val)
                                .ok_or(RuntimeError::DivisionByZero)? as u64;
                        }

                        // POW
                        0b00010000 => {
                            if src2_val < 0 {
                                return Err(RuntimeError::NegativeExponent);
                            }

                            registers[dest_reg] = src1_val
                                .checked_pow(src2_val as u32)
                                .ok_or(RuntimeError::IntegerOverflow)? as u64;
                        }

                        _ => unreachable!(),
                    }
                }

                // Float arith
                0b00100000 => {
                    match inst & 0b0011100 {
                        // ADD
                        0b00000000 => {}

                        // SUB
                        0b00000100 => {}

                        // MUL
                        0b00001000 => {}

                        // DIV
                        0b00001100 => {}

                        // POW
                        0b00010000 => {}

                        // ABS
                        0b00010100 => {}

                        _ => {}
                    }
                }

                // Logical and conversions
                0b01000000 => {}

                // Complex arith
                0b01100000 => {
                    match inst & 0b0011100 {
                        // ADD
                        0b00000000 => {}

                        // SUB
                        0b00000100 => {}

                        // MUL
                        0b00001000 => {}

                        // DIV
                        0b00001100 => {}

                        // POW
                        0b00010000 => {}

                        // ABS
                        0b00010100 => {}

                        _ => {}
                    }
                }

                // Int comp jumps, jump
                0b10000000 => {}

                // Float comp jumps, return
                0b10100000 => {}

                // Int comp ops, mov
                0b11000000 => {}

                // Float comp ops, rnd
                0b11100000 => {}

                _ => {}
            }
        }
    }

    fn read_int_source(registers: &[u64; 64], bytecode: &[u8], inst_counter: &mut usize, is_reg: bool) -> i64 {
        if is_reg {
            let result = registers[bytecode[*inst_counter] as usize] as i64;
            *inst_counter += 1;
            result
        } else {
            let result = i64::from_le_bytes(
                bytecode[*inst_counter..*inst_counter + 8]
                    .try_into()
                    .unwrap(),
            );
            *inst_counter += 8;
            result
        }
    }
}