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

use rand::Rng;

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

    pub fn evaluate (&mut self, relation_id: usize, args: &[u64], sim_timestep: usize, rel_delay: usize) -> u64 {
        // Fill time info and arguments
        self.registers[0] = sim_timestep as u64;
        self.registers[1] = rel_delay as u64;
        self.registers[2..args.len() + 2].copy_from_slice(args);
        
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

            let inst_type = inst & 0b11100000;
            let op        = inst & 0b00011100;
            
            let src1_is_reg = (inst & 0b10) == 0;
            let src2_is_reg = (inst & 0b01) == 0;

            inst_counter += 1;

            match inst & 0b11100000 {
                // Int arith
                0b00000000 => {
                    let dest_reg = bytecode[inst_counter] as usize;

                    inst_counter += 1;

                    // Handles incrementing inst_counter
                    let src1_val = Self::read_source(
                        registers,
                        bytecode,
                        &mut inst_counter,
                        src1_is_reg,
                    ) as i64;

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
                    let src2_val = Self::read_source(
                        registers,
                        bytecode,
                        &mut inst_counter,
                        src2_is_reg,
                    ) as i64;

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
                                return Err(RuntimeError::IntNegativeExponent);
                            }

                            registers[dest_reg] = src1_val
                                .checked_pow(src2_val as u32)
                                .ok_or(RuntimeError::IntegerOverflow)? as u64;
                        }

                        // IMOD
                        0b00011000 => {
                            registers[dest_reg] = src1_val
                                .checked_rem(src2_val)
                                .ok_or(RuntimeError::DivisionByZero)? as u64;
                        }

                        _ => return Err(RuntimeError::InvalidOpcode(inst as u8)),
                    }
                }

                // Float arith
                0b00100000 => {
                    let dest_reg = bytecode[inst_counter] as usize;

                    inst_counter += 1;

                    // Handles incrementing inst_counter
                    let src1_val = f64::from_bits(Self::read_source(
                        registers,
                        bytecode,
                        &mut inst_counter,
                        src1_is_reg,
                    ));

                    // Single source ABS inst
                    match op {
                        // ABS
                        0b00010100 => {
                            registers[dest_reg] = src1_val.abs().to_bits();
                            continue;
                        }

                        _ => {}
                    }

                    // Handles incrementing inst_counter
                    let src2_val = f64::from_bits(Self::read_source(
                        registers,
                        bytecode,
                        &mut inst_counter,
                        src2_is_reg,
                    ));

                    match op {
                        // ADD
                        0b00000000 => {
                            registers[dest_reg] = (src1_val + src2_val).to_bits();
                        }

                        // SUB
                        0b00000100 => {
                            registers[dest_reg] = (src1_val - src2_val).to_bits();
                        }

                        // MUL
                        0b00001000 => {
                            registers[dest_reg] = (src1_val * src2_val).to_bits();
                        }

                        // DIV
                        0b00001100 => {
                            registers[dest_reg] = (src1_val / src2_val).to_bits();
                        }

                        // POW
                        0b00010000 => {
                            registers[dest_reg] = src1_val.powf(src2_val).to_bits();
                        }

                        _ => return Err(RuntimeError::InvalidOpcode(inst as u8)),
                    }
                }

                // Logical and conversions
                0b01000000 => {
                    let dest_reg = bytecode[inst_counter] as usize;

                    inst_counter += 1;

                    match op {
                        // AND
                        0b00000000 => {
                            let src1_val = Self::read_source(
                                registers,
                                bytecode,
                                &mut inst_counter,
                                src1_is_reg,
                            ) != 0;

                            let src2_val = Self::read_source(
                                registers,
                                bytecode,
                                &mut inst_counter,
                                src2_is_reg,
                            ) != 0;

                            registers[dest_reg] = (src1_val & src2_val) as u64;
                        }

                        // OR
                        0b00000100 => {
                            let src1_val = Self::read_source(
                                registers,
                                bytecode,
                                &mut inst_counter,
                                src1_is_reg,
                            ) != 0;

                            let src2_val = Self::read_source(
                                registers,
                                bytecode,
                                &mut inst_counter,
                                src2_is_reg,
                            ) != 0;

                            registers[dest_reg] = (src1_val | src2_val) as u64;
                        }

                        // NOT
                        0b00001000 => {
                            let src_val = Self::read_source(
                                registers,
                                bytecode,
                                &mut inst_counter,
                                src1_is_reg,
                            ) != 0;

                            registers[dest_reg] = (!src_val) as u64;
                        }

                        // XOR
                        0b00001100 => {
                            let src1_val = Self::read_source(
                                registers,
                                bytecode,
                                &mut inst_counter,
                                src1_is_reg,
                            ) != 0;

                            let src2_val = Self::read_source(
                                registers,
                                bytecode,
                                &mut inst_counter,
                                src2_is_reg,
                            ) != 0;

                            registers[dest_reg] = (src1_val ^ src2_val) as u64;
                        }

                        // I2F
                        0b00011000 => {
                            let src_val = Self::read_source(
                                registers,
                                bytecode,
                                &mut inst_counter,
                                src1_is_reg,
                            ) as i64;

                            registers[dest_reg] = (src_val as f64).to_bits();
                        }

                        _ => return Err(RuntimeError::InvalidOpcode(inst as u8)),
                    }
                }

                // Complex arith
                // 0b01100000 => {
                //     match inst & 0b0011100 {
                //         // ADD
                //         0b00000000 => {}

                //         // SUB
                //         0b00000100 => {}

                //         // MUL
                //         0b00001000 => {}

                //         // DIV
                //         0b00001100 => {}

                //         // POW
                //         0b00010000 => {}

                //         // ABS
                //         0b00010100 => {}

                //         _ => {}
                //     }
                // }

                // Int comp jumps, jump
                0b10000000 => {
                    let offset = bytecode[inst_counter] as usize;
                    inst_counter += 1;

                    // No source JMP inst
                    match op {
                        0b00000000 => {
                            inst_counter += offset;
                            continue;
                        }

                        _ => {}
                    }

                    let src1_val = Self::read_source(
                        registers,
                        bytecode,
                        &mut inst_counter,
                        src1_is_reg,
                    ) as i64;

                    let src2_val = Self::read_source(
                        registers,
                        bytecode,
                        &mut inst_counter,
                        src2_is_reg,
                    ) as i64;

                    match op {
                        // IJEQ
                        0b00001000 => if src1_val == src2_val {
                            inst_counter += offset;
                        }
                        

                        // IJNE
                        0b00001100 => if src1_val != src2_val {
                            inst_counter += offset;
                        }

                        // IJLT
                        0b00010000 => if src1_val < src2_val {
                            inst_counter += offset;
                        }

                        // IJLE
                        0b00010100 => if src1_val <= src2_val {
                            inst_counter += offset;
                        }

                        // IJGT
                        0b00011000 => if src1_val > src2_val {
                            inst_counter += offset;
                        }

                        // IJGE
                        0b00011100 => if src1_val >= src2_val {
                            inst_counter += offset;
                        }

                        _ => return Err(RuntimeError::InvalidOpcode(inst as u8)),
                    }
                }

                // Float comp jumps, ret, err
                0b10100000 => {
                    // Get special ops handled first
                    match op {
                        // RET
                        0b00000000 => {
                            // Even if its a float we're returning, we just need to move the raw bytes
                            let src_val = Self::read_source(
                                registers,
                                bytecode,
                                &mut inst_counter,
                                src1_is_reg,
                            );

                            return Ok(src_val);
                        }

                        // ERR
                        0b00000100 => {
                            let code = bytecode[inst_counter];
                            inst_counter += 1;

                            if src1_is_reg { // no info, only code
                                return Err(RuntimeError::from_index(code as usize, None));
                            }else { 
                                let src_val = Self::read_source(
                                    registers,
                                    bytecode,
                                    &mut inst_counter,
                                    src2_is_reg,
                                );

                                return Err(RuntimeError::from_index(code as usize, Some(src_val)));
                            }
                        }

                        _ => {}
                    }

                    let offset = bytecode[inst_counter] as usize;
                    inst_counter += 1;

                    let src1_val = f64::from_bits(Self::read_source(
                        registers,
                        bytecode,
                        &mut inst_counter,
                        src1_is_reg,
                    ));

                    let src2_val = f64::from_bits(Self::read_source(
                        registers,
                        bytecode,
                        &mut inst_counter,
                        src2_is_reg,
                    ));

                    match op {
                        // FJEQ
                        0b00001000 => if src1_val == src2_val {
                            inst_counter += offset;
                        }
                        

                        // FJNE
                        0b00001100 => if src1_val != src2_val {
                            inst_counter += offset;
                        }

                        // FJLT
                        0b00010000 => if src1_val < src2_val {
                            inst_counter += offset;
                        }

                        // FJLE
                        0b00010100 => if src1_val <= src2_val {
                            inst_counter += offset;
                        }

                        // FJGT
                        0b00011000 => if src1_val > src2_val {
                            inst_counter += offset;
                        }

                        // FJGE
                        0b00011100 => if src1_val >= src2_val {
                            inst_counter += offset;
                        }

                        _ => return Err(RuntimeError::InvalidOpcode(inst as u8)),
                    }
                }

                // Int comp ops, mov
                0b11000000 => {
                    let dest_reg = bytecode[inst_counter] as usize;

                    inst_counter += 1;

                    // Single source mov
                    match op {
                        0b00000000 => {
                            // Even if its a float we're moving, we just need to move the raw bytes
                            let src_val = Self::read_source(
                                registers,
                                bytecode,
                                &mut inst_counter,
                                src1_is_reg,
                            );

                            registers[dest_reg] = src_val;
                            continue;
                        }

                        _ => {}
                    }

                    let src1_val = Self::read_source(
                        registers,
                        bytecode,
                        &mut inst_counter,
                        src1_is_reg,
                    ) as i64;

                    let src2_val = Self::read_source(
                        registers,
                        bytecode,
                        &mut inst_counter,
                        src2_is_reg,
                    ) as i64;

                    match op {
                        // IEQ
                        0b00001000 => registers[dest_reg] = (src1_val == src2_val) as u64,

                        // INE
                        0b00001100 => registers[dest_reg] = (src1_val != src2_val) as u64,

                        // ILT
                        0b00010000 => registers[dest_reg] = (src1_val < src2_val) as u64,

                        // ILE
                        0b00010100 => registers[dest_reg] = (src1_val <= src2_val) as u64,

                        // IGT
                        0b00011000 => registers[dest_reg] = (src1_val > src2_val) as u64,

                        // IGE
                        0b00011100 => registers[dest_reg] = (src1_val >= src2_val) as u64,

                        _ => return Err(RuntimeError::InvalidOpcode(inst as u8)),
                    }
                }

                // Float comp ops, rnd
                0b11100000 => {
                    let dest_reg = bytecode[inst_counter] as usize;
                    inst_counter += 1;

                    // No source RND op
                    match op {
                        0b00000000 => {
                            let mut rng = rand::rng();
                            let random_val: f64 = rng.random();

                            registers[dest_reg] = random_val.to_bits();
                            continue;
                        }

                        _ => {}
                    }

                    let src1_val = f64::from_bits(Self::read_source(
                        registers,
                        bytecode,
                        &mut inst_counter,
                        src1_is_reg,
                    ));

                    let src2_val = f64::from_bits(Self::read_source(
                        registers,
                        bytecode,
                        &mut inst_counter,
                        src2_is_reg,
                    ));

                    match op {
                        // FEQ
                        0b00001000 => registers[dest_reg] = (src1_val == src2_val) as u64,

                        // FNE
                        0b00001100 => registers[dest_reg] = (src1_val != src2_val) as u64,

                        // FLT
                        0b00010000 => registers[dest_reg] = (src1_val < src2_val) as u64,

                        // FLE
                        0b00010100 => registers[dest_reg] = (src1_val <= src2_val) as u64,

                        // FGT
                        0b00011000 => registers[dest_reg] = (src1_val > src2_val) as u64,

                        // FGE
                        0b00011100 => registers[dest_reg] = (src1_val >= src2_val) as u64,

                        _ => return Err(RuntimeError::InvalidOpcode(inst as u8)),
                    }
                }

                other => return Err(RuntimeError::InvalidOpcode(inst as u8)),
            }
        }
    }

    fn read_source(registers: &[u64; 64], bytecode: &[u8], inst_counter: &mut usize, is_reg: bool) -> u64 {
        if is_reg {
            let result = registers[bytecode[*inst_counter] as usize] as u64;
            *inst_counter += 1;
            result
        } else {
            let result = u64::from_le_bytes(
                bytecode[*inst_counter..*inst_counter + 8]
                    .try_into()
                    .unwrap(),
            );
            *inst_counter += 8;
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulator::rel_interpreter::test_assembler::assemble;
    use crate::simulator::runtime_diagnostics::RuntimeError;

    #[test]
    fn int_arith() {
        // ((((((23-20) * 2) + 1) / 2) ^ 2) % 10)
        let bytecode = assemble("
            MOV r0 i23
            ISUB r1 r0 i26
            IABS r1 r1
            IMUL r1 r1 r2
            IADD r1 r1 i1
            IDIV r1 r1 i2
            IPOW r1 r1 i2
            IMOD r1 r1 i10
            RET r1
        ").unwrap();

        let mut registers = [0; 64];
        registers[2] = 3;

        let result = RelInterpreter::execute(&mut registers, &bytecode);

        assert_eq!(result, Ok(5 as u64));
    }

    #[test]
    fn float_arith() {
        let bytecode = assemble("
            FADD r0 f1.0 f1.0
            FMUL r0 r0 f2.5  
            I2F r1 i10
            FDIV r0 r0 r1 
            FSUB r0 r0 f1.0  
            FABS r0 r0       
            FPOW r0 r0 f0.0
            RET r0
        ").unwrap();

        let mut registers = [0; 64];

        let result = RelInterpreter::execute(&mut registers, &bytecode);

        assert_eq!(result, Ok((1.0_f64).to_bits() as u64));
    }

    #[test]
    fn bool_arith() {
        let bytecode = assemble("
            MOV r1 i0
            OR r0 r1 i1
            XOR r0 r0 i1
            AND r0 r0 i1
            RET r0
        ").unwrap();

        let mut registers = [0; 64];

        let result = RelInterpreter::execute(&mut registers, &bytecode);

        assert_eq!(result, Ok(0 as u64));
    }

    #[test]
    fn int_jumps() {
        let bytecode = assemble("
            MOV r0 i3
            IJEQ b9 r0 i3
            RET i0
            IJNE b9 r0 i4
            RET i1
            IJLT b9 r0 i5
            RET i2
            IJGT b9 r0 i2
            RET i3
            IJLE b9 r0 i3
            RET i4
            IJGE b9 r0 i3
            RET i5
            RET i6
        ").unwrap();
            

        let mut registers = [0; 64];

        let result = RelInterpreter::execute(&mut registers, &bytecode);

        assert_eq!(result, Ok(6 as u64));
    }

    #[test]
    fn float_jumps() {
        let bytecode = assemble("
            JMP b9
            RET i0
            MOV r0 f3.0
            FJEQ b9 r0 f3.0
            RET i1
            FJNE b9 r0 f4.0
            RET i2
            FJLT b9 r0 f5.0
            RET i3
            FJGT b9 r0 f2.0
            RET i4
            FJLE b9 r0 f3.0
            RET i5
            FJGE b9 r0 f3.0
            RET i6
            RET i7
        ").unwrap();

        let mut registers = [0; 64];

        let result = RelInterpreter::execute(&mut registers, &bytecode);

        assert_eq!(result, Ok(7 as u64));
    }

    #[test]
    fn int_cmp() {
        let bytecode = assemble("
            IEQ r0 i3 i3
            IEQ r1 i3 i4
            INE r2 i3 i4
            INE r3 i3 i3
            ILT r4 i3 i4
            ILT r5 i4 i3
            ILE r6 i3 i3
            ILE r7 i3 i2
            IGT r8 i4 i3
            IGT r9 i4 i4
            IGE r10 i4 i4
            IGE r11 i3 i4
            AND r0 r0 r2
            AND r0 r0 r4
            AND r0 r0 r6
            AND r0 r0 r8
            AND r0 r0 r10
            OR r1 r1 r3
            OR r1 r1 r5
            OR r1 r1 r7
            OR r1 r1 r9
            OR r1 r1 r11
            NOT r1 r1
            AND r0 r0 r1
            RET r0
        ").unwrap();

        let mut registers = [0; 64];

        let result = RelInterpreter::execute(&mut registers, &bytecode);

        assert_eq!(result, Ok(1 as u64));
    }

    #[test]
    fn float_cmp() {
        let bytecode = assemble("
            FEQ r0 f3.0 f3.0
            FEQ r1 f3.0 f4.0
            FNE r2 f3.0 f4.0
            FNE r3 f3.0 f3.0
            FLT r4 f3.0 f4.0
            FLT r5 f4.0 f3.0
            FLE r6 f3.0 f3.0
            FLE r7 f3.0 f2.0
            FGT r8 f4.0 f3.0
            FGT r9 f4.0 f4.0
            FGE r10 f4.0 f4.0
            FGE r11 f3.0 f4.0
            AND r0 r0 r2
            AND r0 r0 r4
            AND r0 r0 r6
            AND r0 r0 r8
            AND r0 r0 r10
            OR r1 r1 r3
            OR r1 r1 r5
            OR r1 r1 r7
            OR r1 r1 r9
            OR r1 r1 r11
            NOT r1 r1
            AND r0 r0 r1
            RET r0
        ").unwrap();

        let mut registers = [0; 64];

        let result = RelInterpreter::execute(&mut registers, &bytecode);

        assert_eq!(result, Ok(1 as u64));
    }

    #[test]
    fn rand() {
        let bytecode = assemble("
            RND r0
            MOV r1 i1
            I2F r1 r1
            FJLE b9 r0 r1
            RET i0
            RET i1
        ").unwrap();

        let mut registers = [0; 64];

        let result = RelInterpreter::execute(&mut registers, &bytecode);

        assert_eq!(result, Ok(1 as u64));
    }

    #[test]
    fn divide_by_zero() {
        let bytecode = assemble("
            IDIV r0 i1 i0
        ").unwrap();

        let mut registers = [0; 64];

        let result = RelInterpreter::execute(&mut registers, &bytecode);

        assert_eq!(result, Err(RuntimeError::DivisionByZero));
    }

    #[test]
    fn int_neg_exponent() {
        let bytecode = assemble("
            IPOW r0 i1 i-1
        ").unwrap();

        let mut registers = [0; 64];

        let result = RelInterpreter::execute(&mut registers, &bytecode);

        assert_eq!(result, Err(RuntimeError::IntNegativeExponent));
    }

    #[test]
    fn int_overflow() {
        // Add 1 to max int value
        let bytecode: Vec<u8> = vec![0b00000011, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f];

        let mut registers = [0; 64];

        let result = RelInterpreter::execute(&mut registers, &bytecode);

        assert_eq!(result, Err(RuntimeError::IntegerOverflow));
    }

    #[test]
    fn invalid_prob() {
        let bytecode = assemble("
            MOV r0 f1.1
            FJLE b3 r0 f1.0
            ERR b4 r0
            RET i1
        ").unwrap();

        let mut registers = [0; 64];

        let result = RelInterpreter::execute(&mut registers, &bytecode);

        assert_eq!(result, Err(RuntimeError::InvalidProb(1.1)));
    }

    #[test]
    fn test_evaluate() {
        let relations = vec![
            CompiledRel {
                complexity: 0,
                bytecode: assemble("
                    FADD r0 r2 r3
                    RET r0
                ").unwrap(),
            },
        ];

        let args = vec![(2.5_f64).to_bits() as u64, (3.0_f64).to_bits() as u64];

        let result = RelInterpreter::new(relations).evaluate(0, &args, 0, 0);

        assert_eq!(5.5_f64.to_bits() as u64, result);


    }
}