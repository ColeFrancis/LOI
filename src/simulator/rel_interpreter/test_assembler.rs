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

//! # test_assembler
//!
//! Minimal bytecode assembler purely for testing the vm and compiler
//!
//! ## Invariants
//!
//! - Must match description in bytecode_design.txt
//!
//! Author: Cole Francis

enum Operand {
    Register(u8),
    Int(i64),
    Float(f64),
    Offset(i16),
    Byte(u8),
}

pub fn assemble(source: &str) -> Result<Vec<u8>, usize> {
    let mut bytecode = Vec::new();

    for (line, text) in source.lines().enumerate() {
        let text = text.trim();
        
        if text.is_empty() || text.starts_with('#') {
            continue;
        }

        if let Some(bytes) = assemble_line(text) {
            bytecode.extend(bytes);
        } 
        else {
            return Err(line + 1);
        }
    }

    Ok(bytecode)
}

fn assemble_line(text: &str) -> Option<Vec<u8>> {
    let mut parts = text.split_whitespace();

    let mnemonic = parts.next()?;

    let operands = parts
        .map(parse_operand)
        .collect::<Option<Vec<Operand>>>()?;

    let mut output: Vec<u8> = Vec::new();

    match mnemonic {
        // -----------------
        // Arithmetic
        // -----------------
        "IADD" | "ISUB" | "IMUL" | "IDIV" | "IPOW" | "IMOD" |
        "FADD" | "FSUB" | "FMUL" | "FDIV" | "FPOW" => {
            if operands.len() != 3 {
                return None;
            }

            let operation = match mnemonic {
                "IADD" | "FADD"  => 0b000,
                "ISUB" | "FSUB"  => 0b001,
                "IMUL" | "FMUL"  => 0b010,
                "IDIV" | "FDIV"  => 0b011,
                "IPOW" | "FPOW"  => 0b100,
                "IMOD"            => 0b110,
                _ => unreachable!(),
            };

            let float = mnemonic.starts_with('F');

            let dest = match operands[0] {
                Operand::Register(r) => r,
                _ => return None,
            };

            let src1_immediate = matches!(operands[1], Operand::Int(_) | Operand::Float(_));
            let src2_immediate = matches!(operands[2], Operand::Int(_) | Operand::Float(_));

            let ss = 
                (src1_immediate as u8) << 1 | 
                (src2_immediate as u8);

            let opcode =
                (0b00 << 6) |
                ((float as u8) << 5) |
                (operation << 2) |
                ss;

            output.push(opcode);
            output.push(dest);

            match operands[1] {
                Operand::Register(r) => output.push(r),
                Operand::Int(i) => output.extend(i.to_le_bytes()),
                Operand::Float(f) => output.extend(f.to_le_bytes()),
                _ => return None,
            }

            match operands[2] {
                Operand::Register(r) => output.push(r),
                Operand::Int(i) => output.extend(i.to_le_bytes()),
                Operand::Float(f) => output.extend(f.to_le_bytes()),
                _ => return None,
            }
        }

        "IABS" | "FABS" => {
            if operands.len() != 2{
                return None;
            }

            let float = mnemonic.starts_with('F');

            let dest = match operands[0] {
                Operand::Register(r) => r,
                _ => return None,
            };

            let src_immediate = matches!(operands[1], Operand::Int(_) | Operand::Float(_));
            
            let ss = 
                (src_immediate as u8) << 1;

            let opcode =
                (0b00 << 6) |
                ((float as u8) << 5) |
                (0b101 << 2) |
                ss;
                
            output.push(opcode);
            output.push(dest);

            match operands[1] {
                Operand::Register(r) => output.push(r),
                Operand::Int(i) => output.extend(i.to_le_bytes()),
                Operand::Float(f) => output.extend(f.to_le_bytes()),
                _ => return None,
            }
        }

        
        // -----------------
        // Logical
        // -----------------
        "AND" | "OR" | "XOR" => {
            if operands.len() != 3 {
                return None;
            }

            let operation = match mnemonic {
                "AND" => 0b00,
                "OR"  => 0b01,
                "XOR" => 0b11,
                _ => unreachable!(),
            };

            let dest = match operands[0] {
                Operand::Register(r) => r,
                _ => return None,
            };

            let src1_immediate = matches!(operands[1], Operand::Int(_) | Operand::Float(_));
            let src2_immediate = matches!(operands[2], Operand::Int(_) | Operand::Float(_));

            let ss =
                (src1_immediate as u8) << 1 |
                (src2_immediate as u8);

            let opcode =
                (0b0100 << 4) |
                (operation << 2) |
                ss;

            output.push(opcode);
            output.push(dest);

            match operands[1] {
                Operand::Register(r) => output.push(r),
                Operand::Int(i) => output.extend(i.to_le_bytes()),
                _ => return None,
            }

            match operands[2] {
                Operand::Register(r) => output.push(r),
                Operand::Int(i) => output.extend(i.to_le_bytes()),
                _ => return None,
            }
        }

        "NOT" => {
            if operands.len() != 2 {
                return None;
            }

            let dest = match operands[0] {
                Operand::Register(r) => r,
                _ => return None,
            };

            let src_immediate = matches!(operands[1], Operand::Int(_) | Operand::Float(_));

            let ss = (src_immediate as u8) << 1;

            let opcode = 
                (0b010010 << 2) |
                ss;

            output.push(opcode);
            output.push(dest);

            match operands[1] {
                Operand::Register(r) => output.push(r),
                Operand::Int(i) => output.extend(i.to_le_bytes()),
                _ => return None,
            };
        }

        // ------------------------------------------------------------
        // ITOF
        // ------------------------------------------------------------

        "I2F" => {
            if operands.len() != 2 {
                return None;
            }

            let dest = match operands[0] {
                Operand::Register(r) => r,
                _ => return None,
            };

            

            let src_immediate = matches!(operands[1], Operand::Int(_) | Operand::Float(_));

            let ss = (src_immediate as u8) << 1;

            let opcode =
                (0b010110) << 2 |
                ss;

            output.push(opcode);
            output.push(dest);

            match operands[1] {
                Operand::Register(r) => output.push(r),
                Operand::Int(i) => output.extend(i.to_le_bytes()),
                _ => return None,
            }
        }

        // ------------------------------------------------------------
        // Jumps
        // ------------------------------------------------------------

        "JMP" => {
            if operands.len() != 1 {
                return None;
            }

            let offset = match operands[0] {
                Operand::Offset(o) => o,
                _ => return None,
            };

            output.push(0b10000000);

            output.extend(offset.to_le_bytes());
        }

        "IJEQ" | "IJNE" | "IJLT" | "IJLE" | "IJGT" | "IJGE" |
        "FJEQ" | "FJNE" | "FJLT" | "FJLE" | "FJGT" | "FJGE" => {
            if operands.len() != 3 {
                return None;
            }

            let operation = match mnemonic {
                "IJEQ" | "FJEQ" => 0b010,
                "IJNE" | "FJNE" => 0b011,
                "IJLT" | "FJLT" => 0b100,
                "IJLE" | "FJLE" => 0b101,
                "IJGT" | "FJGT" => 0b110,
                "IJGE" | "FJGE" => 0b111,
                _ => unreachable!(),
            };

            let float = mnemonic.starts_with('F');

            let offset = match operands[0] {
                Operand::Offset(o) => o,
                _ => return None,
            };

            let src1_immediate = matches!(operands[1], Operand::Int(_) | Operand::Float(_));
            let src2_immediate = matches!(operands[2], Operand::Int(_) | Operand::Float(_));

            let ss =
                (src1_immediate as u8) << 1 |
                (src2_immediate as u8);

            let opcode =
                (0b10 << 6) |
                ((float as u8) << 5) |
                (operation << 2) |
                ss;

            output.push(opcode);
            output.extend(offset.to_le_bytes());

            match operands[1] {
                Operand::Register(r) => output.push(r),
                Operand::Int(i) => output.extend(i.to_le_bytes()),
                Operand::Float(f) => output.extend(f.to_le_bytes()),
                _ => return None,
            }

            match operands[2] {
                Operand::Register(r) => output.push(r),
                Operand::Int(i) => output.extend(i.to_le_bytes()),
                Operand::Float(f) => output.extend(f.to_le_bytes()),
                _ => return None,
            }
        }

        // -----------------
        // Comparisons
        // -----------------

        "IEQ" | "INE" | "ILT" | "IGT" | "ILE" | "IGE" |
        "FEQ" | "FNE" | "FLT" | "FGT" | "FLE" | "FGE" => {
            if operands.len() != 3 {
                return None;
            }

            let operation = match mnemonic {
                "IEQ" | "FEQ" => 0b010,
                "INE" | "FNE" => 0b011,
                "ILT" | "FLT" => 0b100,
                "ILE" | "FLE" => 0b101,
                "IGT" | "FGT" => 0b110,
                "IGE" | "FGE" => 0b111,
                _ => unreachable!(),
            };

            let float = mnemonic.starts_with('F');

            let dest_val = match operands[0] {
                Operand::Register(r) => r,
                _ => return None,
            };

            let src1_immediate = matches!(operands[1], Operand::Int(_) | Operand::Float(_));
            let src2_immediate = matches!(operands[2], Operand::Int(_) | Operand::Float(_));

            let ss =
                (src1_immediate as u8) << 1 |
                (src2_immediate as u8);

            let opcode =
                (0b11 << 6) |
                ((float as u8) << 5) |
                (operation << 2) |
                ss;

            output.push(opcode);
            output.extend(dest_val.to_le_bytes());

            match operands[1] {
                Operand::Register(r) => output.push(r),
                Operand::Int(i) => output.extend(i.to_le_bytes()),
                Operand::Float(f) => output.extend(f.to_le_bytes()),
                _ => return None,
            }

            match operands[2] {
                Operand::Register(r) => output.push(r),
                Operand::Int(i) => output.extend(i.to_le_bytes()),
                Operand::Float(f) => output.extend(f.to_le_bytes()),
                _ => return None,
            }
        }

        // ------------------------------------------------------------
        // Other
        // ------------------------------------------------------------

        "MOV" => {
            if operands.len() != 2 {
                return None;
            }

            let dest = match operands[0] {
                Operand::Register(r) => r,
                _ => return None,
            };

            let src_immediate = matches!(operands[1], Operand::Int(_) | Operand::Float(_));

            let ss = (src_immediate as u8) << 1;

            let opcode = 
                (0b110000) << 2 |
                ss;

            output.push(opcode);
            output.push(dest);

            match operands[1] {
                Operand::Register(r) => output.push(r),
                Operand::Int(i) => output.extend(i.to_le_bytes()),
                Operand::Float(f) => output.extend(f.to_le_bytes()),
                _ => return None,
            }
        }

        "RET" => {
            if operands.len() != 1 {
                return None;
            }

            let src_immediate = matches!(operands[0], Operand::Int(_) | Operand::Float(_));

            let ss = (src_immediate as u8) << 1;

            let opcode = 
                (0b101000) << 2 |
                ss;

            output.push(opcode);

            match operands[0] {
                Operand::Register(r) => output.push(r),
                Operand::Int(i) => output.extend(i.to_le_bytes()),
                Operand::Float(f) => output.extend(f.to_le_bytes()),
                _ => return None,
            }
        }

        "ERR" => {
            if operands.len() != 1 && operands.len() != 2 {
                return None;
            }

            let has_info = operands.len() == 2;

            let src_immediate = if has_info {
                matches!(operands[1], Operand::Int(_) | Operand::Float(_))
            } else {
                false
            };

            let ss = (has_info as u8) << 1 | (src_immediate as u8);

            let opcode = 
                (0b101001) << 2 |
                ss;

            output.push(opcode);

            let code = match operands[0] {
                Operand::Byte(b) => b,
                _ => return None,
            };

            output.push(code);

            if has_info {
                match operands[1] {
                    Operand::Register(r) => output.push(r),
                    Operand::Int(i) => output.extend(i.to_le_bytes()),
                    Operand::Float(f) => output.extend(f.to_le_bytes()),
                    _ => return None,
                }
            }
        }

        "RND" => {
            if operands.len() != 1 {
                return None;
            }

            let dest = match operands[0] {
                Operand::Register(r) => r,
                _ => return None,
            };

            let opcode = 0b11100000;

            output.push(opcode);
            output.push(dest);
        }

        _ => return None,
    }

    Some(output)
}

fn parse_operand(text: &str) -> Option<Operand> {
    let (prefix, value) = text.split_at_checked(1)?;

    match prefix {
        "r" => {
            let reg = value.parse::<u8>().ok()?;

            if reg >= 64 {
                return None;
            }

            Some(Operand::Register(reg))
        }

        "i" => Some(Operand::Int(value.parse().ok()?)),

        "f" => Some(Operand::Float(value.parse().ok()?)),

        "o" => Some(Operand::Offset(value.parse::<i16>().ok()?)),

        "b" => Some(Operand::Byte(value.parse::<u8>().ok()?)),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assembler_1() {
        let result = assemble("
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
        ");

        assert_eq!(result, Ok(vec![0b00000001, 0x01, 0x00, 0x17, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                                   0b00101001, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xC0,
                                   0b00010100, 0x00, 0x01,
                                   0b01000000, 0x00, 0x01, 0x02,
                                   0b01001010, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                                   0b01011000, 0x00, 0x01,
                                   0b10000000, 0x30, 0x00,
                                   0b00011001, 0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                                   0b11001000, 0x00, 0x01, 0x02,
                                   0b10111100, 0x30, 0x00, 0x00, 0x01,
                                   0b11000010, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x40,
                                   0b10100000, 0x00,
                                   0b10100010, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                                   0b11100000, 0x00,
                                   0b10100111, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                                   0b10100100, 0x03,
                                   ]));
    }

    #[test]
    fn test_assembler_2() {
        let result = assemble("
# test
AND r1 r0 f23
FMUL r2 r1 f-2.25
JMP i48
        ");

        assert_eq!(result, Err(3));
    }

    #[test]
    fn test_assembler_3() {
        let result = assemble("
# test
IADD r1 r0 
FMUL r2 r1 f-2.25
JMP i48
        ");

        assert_eq!(result, Err(3));
    }
}