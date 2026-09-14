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

//! # core
//!
//! compiles a single relation into bytecode
//!
//! ## Invariants
//!
//! - unary expr_type will always match the type of their sub-expression
//! - in binary expressions, literal sources will have always been converted to match the expr_type (see fold_expr:63)
//! - booleans must be encoded as int 64 with 1 for true, 0 for false (these values matter in coerce_impulse)
//!
//! Author: Cole Francis

use std::collections::HashMap;

use super::RelCompiler;
use crate::compiler::code_gen::intermediate_rep::{Instruction, Source};
use crate::compiler::ast::*;
use crate::compiler::compiled_rel::CompiledRel;
use crate::compiler::symbol::{Symbol, SymbolId, SymbolKind};
use crate::compiler::sem_analyzer::types::Type;
use crate::compiler::diagnostics::{Diagnostics, Span, CompilerError};

impl<'a> RelCompiler<'a> {
    pub fn compile(relation: RelType, symbol_table: &'a [Symbol], diagnostics: &'a mut Diagnostics) -> Option<CompiledRel> {
        let rel_symbol_id = match relation.name {
            Ident::Symbol(id) => id,
            _ => return None,
        };

        let mut compiler = Self {
            reg_map: HashMap::new(),
            reg_used: [false; 64],
            rel_symbol_id,
            symbol_table,
            diagnostics,
        };

        compiler.compile_relation(relation)
    }

    fn compile_relation(&mut self, relation: RelType) -> Option<CompiledRel> {
        // setup initial registers, arguments, etc

        // r0 and r1 are reserved for timestep and delay
        self.reg_used[0] = true;
        self.reg_used[1] = true;

        // reserve registers for params
        for param in relation.params {
            if let Ident::Symbol(symbol_id) = param.name {
                // This call allows us to catch and report errors
                let idx = self.get_next_reg()?;

                self.reg_map.insert(symbol_id, idx);
            }
        }

        let (mut ir_bytecode, mut src, mut result_type) = self.compile_expr(relation.body)?;

        // coerce to match return type
        let src = match relation.return_type {
            Type::Bool => self.coerce_bool(&mut ir_bytecode, src, &result_type)?,

            Type::Impulse => self.coerce_impulse(&mut ir_bytecode, src, &result_type)?,

            Type::Int => self.coerce_int(&mut ir_bytecode, src, &result_type)?,

            Type::Real => self.coerce_real(&mut ir_bytecode, src, &result_type, true)?,

            Type::Mod(n) => self.coerce_int(&mut ir_bytecode, src, &result_type)?, // handles taking modulus

            Type::Custom(Ident::Symbol(id)) => src,

            _ => return None,
        };

        ir_bytecode.push(Instruction::RET {
            src,
        });

        // remove deadcode
            // step backwards through code, if a variable gets used as a src, mark it as alive, once it isdefiend, mark as dead
            //  if a variable is declared while not alive, remove that instruction
            //
            // Also remove all sample/cases arms past defaults?
            //  take care to modify jump offsets if instructsions are removed between the jump and its target

        // convert intermediate rep to u8

        Some(CompiledRel {
            name: self.symbol_table[self.rel_symbol_id].name.clone(),
            complexity: 0,
            bytecode: Vec::new(),
        })
    }

    // Finds next available register, marks it as used, and returns its index
    // reports error and returns none if there are no registers left
    pub fn get_next_reg(&mut self) -> Option<usize> {
        let idx_option = self.reg_used.iter().position(|&x| !x);

        if let Some(idx) = idx_option {
            self.reg_used[idx] = true;

        } else {
            self.diagnostics.error(CompilerError::TooManySymbols {
                rel_name: self.symbol_table[self.rel_symbol_id].name.clone(),
                rel_span: self.symbol_table[self.rel_symbol_id].span.clone(),
            });
        }

        idx_option
    }

    pub fn coerce_int(&mut self, bytecode: &mut Vec<Instruction>, src: Source, ty: &Type) -> Option<Source> {
        match ty {
            Type::Mod(n) => {
                let dest = match src {
                    Source::RegInter(reg) => reg,
                    _ => self.get_next_reg()?,
                };

                bytecode.push(Instruction::MOD {
                    dest,
                    src1: src,
                    src2: Source::Int(*n),
                });

                Some(Source::RegInter(dest))
            }
            Type::Int => Some(src),
            Type::Custom(_) => Some(src), // custom types stored internally as ints which are mapped to the values the type takes
            _ => unreachable!("cannot coerce {:?} to Int", ty.clone())
        }
    }

    // Reduce mod should be false for add/sub/mul because (a + b) mod n == (a mod n + b mod n) mod n but it is not true for div nor the right side of pow
    pub fn coerce_real(&mut self, bytecode: &mut Vec<Instruction>, src: Source, ty: &Type, reduce_mod: bool) -> Option<Source> {
        match ty {
            Type::Mod(n) => {
                let src = if reduce_mod {
                    let dest = match src {
                        Source::RegInter(reg) => reg,
                        _ => self.get_next_reg()?,
                    };

                    bytecode.push(Instruction::MOD {
                        dest,
                        src1: src,
                        src2: Source::Int(*n),
                    });

                    Source::RegInter(dest)
                } else {
                    src
                };

                let dest = match src {
                    Source::RegInter(reg) => reg,
                    _ => self.get_next_reg()?,
                };
                bytecode.push(Instruction::I2F {
                    dest,
                    src,
                });

                Some(Source::RegInter(dest))
            }
            Type::Int => {
                let dest = match src {
                    Source::RegInter(reg) => reg,
                    _ => self.get_next_reg()?,
                };
                bytecode.push(Instruction::I2F {
                    dest,
                    src,
                });

                Some(Source::RegInter(dest))
            }
            Type::Real => Some(src),
            _ => unreachable!("cannot coerce {:?} to be Real", ty.clone()),
        }
    }

    // Converts all impulse type to bool with an IEQ
    pub fn coerce_bool(&mut self, bytecode: &mut Vec<Instruction>, src: Source, ty: &Type, ) -> Option<Source> {
        match ty {
            Type::Impulse => {
                let dest = match src {
                    Source::RegInter(reg) => reg,
                    _ => self.get_next_reg()?,
                };
                bytecode.push(Instruction::IEQ {
                    dest,
                    src1: src,
                    src2: Source::RegVar(0), // sim timestep is always in reg 0
                });
                Some(Source::RegInter(dest))
            }

            Type::Bool => Some(src),
            _ => unreachable!("cannot coerce {:?} into Bool", ty.clone()),
        }
    }

    pub fn coerce_impulse(&mut self, bytecode: &mut Vec<Instruction>, src: Source, ty: &Type, ) -> Option<Source> {
        match ty {
            Type::Impulse => {
                let dest = match src {
                    Source::RegInter(reg) => reg,
                    _ => self.get_next_reg()?,
                };
                bytecode.push(Instruction::IADD {
                    dest,
                    src1: src,
                    src2: Source::RegVar(1), // add relation delay
                });
                Some(Source::RegInter(dest))
            }

            // if true, return relation delay added to current timestep else return 0
            // comparison is simply done through multiplying by src because boolean is encoded as 1 for true, 0 for false
            Type::Bool => {
                let dest = match src {
                    Source::RegInter(reg) => reg,
                    _ => self.get_next_reg()?,
                };
                let time_reg = self.get_next_reg()?;

                bytecode.push(Instruction::IADD {
                    dest: time_reg,
                    src1: Source::RegVar(0),
                    src2: Source::RegVar(1),
                });
                bytecode.push(Instruction::IMUL {
                    dest, 
                    src1: src,
                    src2: Source::RegInter(time_reg),
                });

                Some(Source::RegInter(dest))
            },
            _ => unreachable!("cannot coerce {:?} into Bool", ty.clone()),
        }
    }

    // TODO: make work for impulses as well
    // Used in cases pattern matching to ensure the sources are the same type
    pub fn coerce_equal(&mut self, bytecode: &mut Vec<Instruction>, src_l: Source, type_l: &Type, src_r: Source, type_r: &Type) -> Option<(Source, Source, Type)> {
        let (new_src_l, new_type) = match type_r {
            Type::Real => (self.coerce_real(bytecode, src_l, &type_l, true)?, Type::Real),

            // Need to be careful not to try and coerce real to be int (the next match will take that int to be real)
            Type::Int if *type_l != Type::Real => (self.coerce_int(bytecode, src_l, &type_l)?, Type::Int),

            Type::Mod(n) if (*type_l != Type::Real && *type_l != Type::Int) => (self.coerce_int(bytecode, src_l, &type_l)?, Type::Mod(*n)),

            Type::Bool => (self.coerce_bool(bytecode, src_l, &type_l)?, Type::Bool),

            _ => (src_l, type_l.clone()),
        };

        let new_src_r = match new_type {
            Type::Real => self.coerce_real(bytecode, src_r, &type_r, true)?,

            Type::Int => self.coerce_int(bytecode, src_r, &type_r)?,

            Type::Bool => self.coerce_bool(bytecode, src_r, &type_r)?,

            // Take modulus before comparison
            Type::Mod(_n) => self.coerce_int(bytecode, src_r, &type_r)?,

            _ => src_r,
        };
        
        Some((new_src_l, new_src_r, new_type))
    }

    pub fn get_binary_dest(&mut self, src1: Source, src2: Source) -> Option<usize> {
        match (src1, src2) {
            // When both are available, we should free one after the op
            (Source::RegInter(reg1), Source::RegInter(reg2)) => {
                self.reg_used[reg2] = false;
                Some(reg1)
            }, 
            (Source::RegInter(reg), _) => Some(reg),
            (_, Source::RegInter(reg)) => Some(reg),
            _ => Some(self.get_next_reg()?),
        }
    }

    pub fn get_num_bytes(instrcutions: &[Instruction]) -> usize {
        let mut num = 0;
        for instruction in instrcutions {
            num += match instruction {
                Instruction::IADD{src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::ISUB{src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::IMUL{src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::IDIV{src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::IPOW{src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::IABS{src, ..}         => 2 + Self::get_num_source_bytes(src),
                Instruction::MOD {src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::FADD{src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::FSUB{src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::FMUL{src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::FDIV{src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::FPOW{src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::FABS{src, ..}         => 2 + Self::get_num_source_bytes(src),
                Instruction::AND {src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::OR  {src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::NOT {src, ..}         => 2 + Self::get_num_source_bytes(src),
                Instruction::XOR {src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::I2F {src, ..}         => 2 + Self::get_num_source_bytes(src),
                Instruction::JMP {..}              => 3,
                Instruction::IJEQ{src1, src2, ..}  => 3 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::IJNE{src1, src2, ..}  => 3 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::IJLT{src1, src2, ..}  => 3 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::IJGT{src1, src2, ..}  => 3 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::IJLE{src1, src2, ..}  => 3 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::IJGE{src1, src2, ..}  => 3 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::FJEQ{src1, src2, ..}  => 3 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::FJNE{src1, src2, ..}  => 3 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::FJLT{src1, src2, ..}  => 3 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::FJGT{src1, src2, ..}  => 3 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::FJLE{src1, src2, ..}  => 3 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::FJGE{src1, src2, ..}  => 3 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::IEQ {src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::INE {src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::ILT {src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::IGT {src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::ILE {src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::IGE {src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::FEQ {src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::FNE {src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::FLT {src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::FGT {src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::FLE {src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::FGE {src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::MOV {src, ..}         => 2 + Self::get_num_source_bytes(src),
                Instruction::RET {src, ..}         => 1 + Self::get_num_source_bytes(src),
                Instruction::ERR {src, ..}         => 2 + match src {
                    Some(src) => Self::get_num_source_bytes(src),
                    None      => 0
                },
                Instruction::RND {..}              => 2,
            }
        }

        num
    }

    fn get_num_source_bytes(source: &Source) -> usize {
        match source {
            Source::RegInter(_) | Source::RegVar(_) => 1,
            _ => 8,
        }
    }

    pub fn update_jmp_offset(inst: &mut Instruction, change: i16) {
        match inst {
            Instruction::JMP { offset } => *offset += change,
            
            Instruction::IJEQ { offset, .. } => *offset += change,
            Instruction::IJNE { offset, .. } => *offset += change,
            Instruction::IJLT { offset, .. } => *offset += change,
            Instruction::IJGT { offset, .. } => *offset += change,
            Instruction::IJLE { offset, .. } => *offset += change,
            Instruction::IJGE { offset, .. } => *offset += change,
            
            Instruction::FJEQ { offset, .. } => *offset += change,
            Instruction::FJNE { offset, .. } => *offset += change,
            Instruction::FJLT { offset, .. } => *offset += change,
            Instruction::FJGT { offset, .. } => *offset += change,
            Instruction::FJLE { offset, .. } => *offset += change,
            Instruction::FJGE { offset, .. } => *offset += change,

            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // test a relation that returns impulse for the two cases of passing impulse straight throguh or converting bool to impulse
}
