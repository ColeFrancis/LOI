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

//! # lower_ir
//!
//! lowers IR to bytes
//!
//! ## Invariants
//!
//! Author: Cole Francis

use super::CodeGen;
use super::intermediate_rep::{Instruction, Source};

impl<'a> CodeGen<'a> {
    pub(super) fn lower_ir(ir_bytecode: Vec<Instruction>) -> Vec<u8> {
        let mut bytes = Vec::new();

        for instruction in ir_bytecode {
            match instruction {
                Instruction::IADD {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b000000 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::ISUB {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b000001 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::IMUL {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b000010 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::IDIV {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b000011 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::IPOW {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b000100 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::IABS {dest, src       } => {
                    let (src_immediate, src_bytes) = Self::src_to_bytes(src);

                    let ss = (src_immediate as u8) << 1;

                    let opcode = (0b000101 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src_bytes);
                }
                Instruction::IMOD  {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b000110 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                
                Instruction::FADD {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b001000 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::FSUB {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b001001 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::FMUL {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b001010 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::FDIV {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b001011 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::FPOW {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b001100 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::FABS {dest, src       } => {
                    let (src_immediate, src_bytes) = Self::src_to_bytes(src);

                    let ss = (src_immediate as u8) << 1;

                    let opcode = (0b001101 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src_bytes);
                }
                
                Instruction::AND  {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b010000 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::OR   {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b010001 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::NOT  {dest, src       } => {
                    let (src_immediate, src_bytes) = Self::src_to_bytes(src);

                    let ss = (src_immediate as u8) << 1;

                    let opcode = (0b010010 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src_bytes);
                }
                Instruction::XOR  {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b010011 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }

                Instruction::I2F  {dest, src       } => {
                    let (src_immediate, src_bytes) = Self::src_to_bytes(src);

                    let ss = (src_immediate as u8) << 1;

                    let opcode = (0b010110 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src_bytes);
                }

                Instruction::JMP  {offset          } => {
                    let opcode = 0b10000000;

                    let offset_bytes = (offset as i16).to_le_bytes();

                    bytes.push(opcode);
                    bytes.extend(offset_bytes);
                }

                Instruction::IJEQ {offset, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b100010 << 2) | ss;

                    let offset_bytes = (offset as i16).to_le_bytes();

                    bytes.push(opcode);
                    bytes.extend(offset_bytes);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::IJNE {offset, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b100011 << 2) | ss;

                    let offset_bytes = (offset as i16).to_le_bytes();

                    bytes.push(opcode);
                    bytes.extend(offset_bytes);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::IJLT {offset, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b100100 << 2) | ss;

                    let offset_bytes = (offset as i16).to_le_bytes();

                    bytes.push(opcode);
                    bytes.extend(offset_bytes);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::IJGT {offset, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b100110 << 2) | ss;

                    let offset_bytes = (offset as i16).to_le_bytes();

                    bytes.push(opcode);
                    bytes.extend(offset_bytes);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::IJLE {offset, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b100101 << 2) | ss;

                    let offset_bytes = (offset as i16).to_le_bytes();

                    bytes.push(opcode);
                    bytes.extend(offset_bytes);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::IJGE {offset, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b100111 << 2) | ss;

                    let offset_bytes = (offset as i16).to_le_bytes();

                    bytes.push(opcode);
                    bytes.extend(offset_bytes);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }

                Instruction::FJEQ {offset, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b101010 << 2) | ss;

                    let offset_bytes = (offset as i16).to_le_bytes();

                    bytes.push(opcode);
                    bytes.extend(offset_bytes);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::FJNE {offset, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b101011 << 2) | ss;

                    let offset_bytes = (offset as i16).to_le_bytes();

                    bytes.push(opcode);
                    bytes.extend(offset_bytes);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::FJLT {offset, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b101100 << 2) | ss;

                    let offset_bytes = (offset as i16).to_le_bytes();

                    bytes.push(opcode);
                    bytes.extend(offset_bytes);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::FJGT {offset, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b101110 << 2) | ss;

                    let offset_bytes = (offset as i16).to_le_bytes();

                    bytes.push(opcode);
                    bytes.extend(offset_bytes);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::FJLE {offset, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b101101 << 2) | ss;

                    let offset_bytes = (offset as i16).to_le_bytes();

                    bytes.push(opcode);
                    bytes.extend(offset_bytes);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::FJGE {offset, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b101111 << 2) | ss;

                    let offset_bytes = (offset as i16).to_le_bytes();

                    bytes.push(opcode);
                    bytes.extend(offset_bytes);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }

                Instruction::IEQ  {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b110010 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::INE  {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b110011 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::ILT  {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b110100 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::IGT  {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b110110 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::ILE  {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b110101 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::IGE  {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b110111 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }

                Instruction::FEQ  {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b111010 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::FNE  {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b111011 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::FLT  {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b111100 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::FGT  {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b111110 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::FLE  {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b111101 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }
                Instruction::FGE  {dest, src1, src2} => {
                    let (src1_immediate, src1_bytes) = Self::src_to_bytes(src1);
                    let (src2_immediate, src2_bytes) = Self::src_to_bytes(src2);

                    let ss = (src1_immediate as u8) << 1 | (src2_immediate as u8);

                    let opcode = (0b111111 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src1_bytes);
                    bytes.extend(src2_bytes);
                }

                Instruction::MOV  {dest, src       } => {
                    let (src_immediate, src_bytes) = Self::src_to_bytes(src);

                    let ss = (src_immediate as u8) << 1;

                    let opcode = (0b110000 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                    bytes.extend(src_bytes);
                }
                Instruction::RET  {src             } => {
                    let (src_immediate, src_bytes) = Self::src_to_bytes(src);

                    let ss = (src_immediate as u8) << 1;

                    let opcode = (0b101000 << 2) | ss;

                    bytes.push(opcode);
                    bytes.extend(src_bytes);
                }
                Instruction::ERR  {code, src       } => {
                    let (ss, src_bytes) = match src {
                        Some(Source::RegInter(val)) => (0b10 as u8, vec![val as u8]),
                        Some(Source::RegVar  (val)) => (0b10 as u8, vec![val as u8]),
                        Some(Source::Bool    (val)) => (0b11 as u8, (val as u64).to_le_bytes().to_vec()),
                        Some(Source::Int     (val)) => (0b11 as u8, val.to_le_bytes().to_vec()),
                        Some(Source::Float   (val)) => (0b11 as u8, val.to_le_bytes().to_vec()),
                        None                        => (0b00 as u8, vec![]),
                    };

                    let opcode = (0b101001 << 2) | ss;

                    bytes.push(opcode);
                    bytes.push(code as u8);
                    bytes.extend(src_bytes);
                }
                Instruction::RND  {dest            } => {
                    let opcode = 0b11100000;

                    bytes.push(opcode);
                    bytes.push(dest as u8);
                }
            }
        }

        bytes
    }

    fn src_to_bytes(src: Source) -> (bool, Vec<u8>) {
        match src {
            Source::RegInter(val) => (false, vec![val as u8]),
            Source::RegVar  (val) => (false, vec![val as u8]),
            Source::Bool    (val) => (true,  (val as u64).to_le_bytes().to_vec()),
            Source::Int     (val) => (true,  val.to_le_bytes().to_vec()),
            Source::Float   (val) => (true,  val.to_le_bytes().to_vec()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulator::rel_interpreter::test_assembler::assemble;

    #[test]
    fn test_all_instructions() {
        let result = CodeGen::lower_ir(vec![
            Instruction::IADD {
                dest: 1,
                src1: Source::RegInter(0),
                src2: Source::Int(23),
            },
            Instruction::FMUL {
                dest: 2,
                src1: Source::RegVar(1),
                src2: Source::Float(-2.25),
            },
            Instruction::IABS {
                dest: 0,
                src: Source::RegInter(1),
            },
            Instruction::AND {
                dest: 0,
                src1: Source::RegInter(1),
                src2: Source::RegInter(2),
            },
            Instruction::NOT {
                dest: 0,
                src: Source::Int(1),
            },
            Instruction::I2F {
                dest: 0,
                src: Source::RegInter(1),
            },
            Instruction::JMP {
                offset: 48,
            },
            Instruction::IMOD {
                dest: 0,
                src1: Source::RegInter(1),
                src2: Source::Int(2),
            },
            Instruction::IEQ {
                dest: 0,
                src1: Source::RegInter(1),
                src2: Source::RegVar(2),
            },
            Instruction::FJGE {
                offset: 48,
                src1: Source::RegInter(0),
                src2: Source::RegInter(1),
            },
            Instruction::MOV {
                dest: 0,
                src: Source::Float(3.0),
            },
            Instruction::RET {
                src: Source::RegInter(0),
            },
            Instruction::RET {
                src: Source::Int(1),
            },
            Instruction::RND {
                dest: 0
            },
            Instruction::ERR {
                code: 4,
                src: Some(Source::Int(0)),
            },
            Instruction::ERR {
                code: 3,
                src: None,
            },
        ]);

        let test = assemble("
# test
IADD r1 r0 i23
FMUL r2 r1 f-2.25
IABS r0 r1
AND r0 r1 r2
NOT r0 i1
I2F r0 r1
JMP o48
IMOD r0 r1 i2
IEQ r0 r1 r2
FJGE o48 r0 r1
MOV r0 f3.0
RET r0
RET i1
RND r0
ERR b4 i0
ERR b3
        ").unwrap();

        assert_eq!(result, test);
    }

    #[test]
    fn exaustive_test() {
        let result = CodeGen::lower_ir(vec![
            Instruction::IADD {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::ISUB {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::IMUL {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::IDIV {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::IPOW {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::IABS {
                dest: 0,
                src: Source::RegInter(0),
            },
            Instruction::IMOD {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::FADD {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::FSUB {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::FMUL {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::FDIV {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::FPOW {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::FABS {
                dest: 0,
                src: Source::RegInter(0),
            },
            Instruction::AND {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::OR {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::NOT {
                dest: 0,
                src: Source::RegInter(0),
            },
            Instruction::XOR {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::I2F {
                dest: 0,
                src: Source::RegInter(0),
            },
            Instruction::JMP {
                offset: 0,
            },
            Instruction::IJEQ {
                offset: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::IJNE {
                offset: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::IJLT {
                offset: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::IJGT {
                offset: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::IJLE {
                offset: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::IJGE {
                offset: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::FJEQ {
                offset: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::FJNE {
                offset: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::FJLT {
                offset: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::FJGT {
                offset: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::FJLE {
                offset: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::FJGE {
                offset: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::IEQ {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::INE {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::ILT {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::IGT {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::ILE {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::IGE {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::FEQ {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::FNE {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::FLT {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::FGT {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::FLE {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::FGE {
                dest: 0,
                src1: Source::RegInter(0),
                src2: Source::RegInter(0),
            },
            Instruction::MOV {
                dest: 0,
                src: Source::RegInter(0),
            },
            Instruction::RET {
                src: Source::RegInter(0),
            },
            Instruction::ERR {
                code: 0,
                src: None,
            },
            Instruction::RND {
                dest: 0,
            },
        ]);

        let test = assemble("
# test
IADD r0 r0 r0
ISUB r0 r0 r0
IMUL r0 r0 r0
IDIV r0 r0 r0
IPOW r0 r0 r0
IABS r0 r0
IMOD r0 r0 r0

FADD r0 r0 r0
FSUB r0 r0 r0
FMUL r0 r0 r0
FDIV r0 r0 r0
FPOW r0 r0 r0
FABS r0 r0

AND r0 r0 r0
OR r0 r0 r0
NOT r0 r0
XOR r0 r0 r0

I2F r0 r0
JMP o0

IJEQ o0 r0 r0
IJNE o0 r0 r0
IJLT o0 r0 r0
IJGT o0 r0 r0
IJLE o0 r0 r0
IJGE o0 r0 r0

FJEQ o0 r0 r0
FJNE o0 r0 r0
FJLT o0 r0 r0
FJGT o0 r0 r0
FJLE o0 r0 r0
FJGE o0 r0 r0

IEQ r0 r0 r0
INE r0 r0 r0
ILT r0 r0 r0
IGT r0 r0 r0
ILE r0 r0 r0
IGE r0 r0 r0

FEQ r0 r0 r0
FNE r0 r0 r0
FLT r0 r0 r0
FGT r0 r0 r0
FLE r0 r0 r0
FGE r0 r0 r0

MOV r0 r0
RET r0
ERR b0
RND r0
        ").unwrap();

        assert_eq!(result, test);
    }
}