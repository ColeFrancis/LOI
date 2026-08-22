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

enum Operand{
    Register(u8),
    Int(i64),
    Float(f64),
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
        "IADD" | "ISUB" | "IMUL" | "IDIV" | "IPOW" |
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
            }

            match operands[2] {
                Operand::Register(r) => output.push(r),
                Operand::Int(i) => output.extend(i.to_le_bytes()),
                Operand::Float(f) => output.extend(f.to_le_bytes()),
            }
        }

        
        // -----------------
        // Logical
        // -----------------
        "AND" | "OR" => {
            if operands.len() != 3 {
                return None;
            }

            let operation = match mnemonic {
                "AND" => 0b00,
                "OR"  => 0b01,
                _ => unreachable!(),
            };

            let dest = match operands[0] {
                Operand::Register(r) => r,
                _ => return None,
            };

            let src1_immediate = matches!(operands[1], Operand::Int(_));
            let src2_immediate = matches!(operands[2], Operand::Int(_));

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

            let src_immediate = matches!(operands[1], Operand::Int(_));

            let ss = (src_immediate as u8) << 1;

            let opcode = 
                (0b010010 << 2) |
                ss;

            output.push(opcode);
            output.push(dest);

            match operands[1] {
                Operand::Register(r) => output.push(r),
                Operand::Int(i) => output.extend(i.to_le_bytes()),
                Operand::Float(_) => return None,
            };
        }

        // ------------------------------------------------------------
        // MOD
        // ------------------------------------------------------------

        "MOD" => {
            if operands.len() != 3 {
                return None;
            }

            let dest = match operands[0] {
                Operand::Register(r) => r,
                _ => return None,
            };

            let src_immediate = matches!(operands[1], Operand::Int(_));

            let ss = (src_immediate as u8) << 1;

            let opcode = (0b011000 << 2) | ss;

            output.push(opcode);
            output.push(dest);

            match operands[1] {
                Operand::Register(r) => output.push(r),
                Operand::Int(i) => output.extend(i.to_le_bytes()),
                _ => return None,
            }

            match operands[1] {
                Operand::Int(i) => output.extend(i.to_le_bytes()),
                _ => return None,
            }
        }

        // ------------------------------------------------------------
        // ITOF
        // ------------------------------------------------------------

        "ITOF" => {
            if operands.len() != 2 {
                return None;
            }

            let dest = match operands[0] {
                Operand::Register(r) => r,
                _ => return None,
            };

            let src = match operands[1] {
                Operand::Register(r) => r,
                Operand::Int(i) => {
                    output.push(
                        (0b01 << 6) |
                        (0b1 << 5) |
                        (0b100 << 2) |
                        0b01
                    );

                    output.push(dest);
                    output.extend(i.to_le_bytes());

                    return Some(output);
                }
                Operand::Float(_) => return None,
            };

            output.push(0b01110000);

            match operands[1] {
                Operand::Register(r) => output.push(r),
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
                Operand::Int(i) => i,
                _ => return None,
            };

            output.push(0b10000000);

            output.extend(offset.to_le_bytes());
        }

        "JEQ" | "JLT" | "JLE" | "JGT" | "JGE" => {
            if operands.len() != 3 {
                return None;
            }

            let operation = match mnemonic {
                "JEQ" => 0b001,
                "JLT" => 0b100,
                "JLE" => 0b101,
                "JGT" => 0b110,
                "JGE" => 0b111,
                _ => unreachable!(),
            };

            let offset = match operands[0] {
                Operand::Int(i) => i,
                _ => return None,
            };

            let src1_immediate = matches!(operands[1], Operand::Int(_));
            let src2_immediate = matches!(operands[2], Operand::Int(_));

            let ss =
                (src1_immediate as u8) << 1 |
                (src2_immediate as u8);

            let opcode =
                (0b10 << 6) |
                (operation << 2) |
                ss;

            output.push(opcode);
            output.extend(offset.to_le_bytes());

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

            let opcode = 0b110000;

            output.push(opcode);
            output.push(dest);

            match operands[1] {
                Operand::Register(r) => output.push(r),
                Operand::Int(i) => output.extend(i.to_le_bytes()),
                Operand::Float(f) => output.extend(f.to_le_bytes()),
            }
        }

        "RET" => {
            if operands.len() != 1 {
                return None;
            }

            let opcode = 0b110001;

            output.push(opcode);

            match operands[0] {
                Operand::Register(r) => output.push(r),
                Operand::Int(i) => output.extend(i.to_le_bytes()),
                Operand::Float(f) => output.extend(f.to_le_bytes()),
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

            let opcode = 0b110100;

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
JMP i48
        ");

        assert_eq!(result, Ok(vec![0b00000001, 0x01, 0x00, 0x17, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                                   0b00101001, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xC0,
                                   0b10000000, 0x30, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
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