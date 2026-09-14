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
use super::intermediate_rep::{Instruction, Source};
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
        // TODO: algebraically optimize relation body:
        
        // r0 and r1 are reserved for timestep and delay, respectively
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

        // TODO: remove deadcode
            // step backwards through code, if a variable gets used as a src, mark it as alive, once it isdefiend, mark as dead
            //  if a variable is declared while not alive, remove that instruction
            //
            // Also remove all sample/cases arms past defaults?
            //  take care to modify jump offsets if instructsions are removed between the jump and its target

        let bytecode = Self::lower_ir(ir_bytecode);

        Some(CompiledRel {
            name: self.symbol_table[self.rel_symbol_id].name.clone(),
            complexity: 0,
            bytecode,
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

                bytecode.push(Instruction::IMOD {
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

                    bytecode.push(Instruction::IMOD {
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
                Instruction::IMOD{src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
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
    use crate::compiler::{
        lexer::Lexer,
        parser::Parser,
        sem_analyzer::SemAnalyzer,
    };
    use crate::simulator::rel_interpreter::{RelInterpreter, test_assembler::assemble};

    #[test]
    fn simple_adder() {
        // rel_t ADD: (a: Int, b: Int) -> Int {
        //     a + b
        // };
        let mut diagnostics = Diagnostics::new();
        let symbol_table = vec![
            Symbol {
                name: "ADD".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Int, Type::Int],
                    return_type: Type::Int,
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "a".to_string(),
                kind: SymbolKind::Variable(Type::Int),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "b".to_string(),
                kind: SymbolKind::Variable(Type::Int),
                span: Span{line: 0, col: 0},
            },
        ];

        let relation = RelType {
            name: Ident::Symbol(0),
            params: vec![
                Param {
                    name: Ident::Symbol(1),
                    param_type: Type::Int,
                },
                Param {
                    name: Ident::Symbol(2),
                    param_type: Type::Int,
                },
            ],
            return_type: Type::Int,
            body: Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Ident(Ident::Symbol(1))),
                right: Box::new(Expr::Ident(Ident::Symbol(2))),
                op: BinaryOp::Add,
                op_span: Span {line: 0, col: 0},
                expr_type: Type::Int,
            }),
        };

        let result = RelCompiler::compile(relation, &symbol_table, &mut diagnostics);

        let bytecode = assemble("
            IADD r4 r2 r3
            RET r4
        ").unwrap();

        assert_eq!(result, Some(CompiledRel {
            name: "ADD".to_string(),
            complexity: 0,
            bytecode,
        }));
    }

    #[test]
    fn impulse_delay() {
        // rel_t DELAY: (a: Impulse) -> Impulse {
        //     a
        // };
        let mut diagnostics = Diagnostics::new();
        let symbol_table = vec![
            Symbol {
                name: "DELAY".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Impulse],
                    return_type: Type::Impulse,
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "a".to_string(),
                kind: SymbolKind::Variable(Type::Impulse),
                span: Span{line: 0, col: 0},
            },
        ];

        let relation = RelType {
            name: Ident::Symbol(0),
            params: vec![
                Param {
                    name: Ident::Symbol(1),
                    param_type: Type::Impulse,
                },
            ],
            return_type: Type::Impulse,
            body: Expr::Ident(Ident::Symbol(1)),
        };

        let result = RelCompiler::compile(relation, &symbol_table, &mut diagnostics);

        let bytecode = assemble("
            IADD r3 r2 r1
            RET r3
        ").unwrap();

        assert_eq!(result, Some(CompiledRel {
            name: "DELAY".to_string(),
            complexity: 0,
            bytecode,
        }));
    }

    #[test]
    fn impulse_and() {
        // rel_t DELAY: (a: Impulse, b: Impulse) -> Impulse {
        //     a & b
        // };
        let mut diagnostics = Diagnostics::new();
        let symbol_table = vec![
            Symbol {
                name: "AND".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Impulse, Type::Impulse],
                    return_type: Type::Impulse,
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "a".to_string(),
                kind: SymbolKind::Variable(Type::Impulse),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "b".to_string(),
                kind: SymbolKind::Variable(Type::Impulse),
                span: Span{line: 0, col: 0},
            },
        ];

        let relation = RelType {
            name: Ident::Symbol(0),
            params: vec![
                Param {
                    name: Ident::Symbol(1),
                    param_type: Type::Impulse,
                },
                Param {
                    name: Ident::Symbol(2),
                    param_type: Type::Impulse,
                },
            ],
            return_type: Type::Impulse,
            body: Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Ident(Ident::Symbol(1))),
                right: Box::new(Expr::Ident(Ident::Symbol(2))),
                op: BinaryOp::And,
                op_span: Span {line: 0, col: 0},
                expr_type: Type::Bool,
            }),
        };

        let result = RelCompiler::compile(relation, &symbol_table, &mut diagnostics);

        let bytecode = assemble("
            IEQ r4 r2 r0
            IEQ r5 r3 r0
            AND r4 r4 r5
            IADD r5 r0 r1
            IMUL r4 r4 r5
            RET r4
        ").unwrap();

        assert_eq!(result, Some(CompiledRel {
            name: "AND".to_string(),
            complexity: 0,
            bytecode,
        }));
    }

    #[test]
    fn custom_cases() {
        // ent_t coin = {H, T};
        // rel_t REVERSE : (a: coin) -> coin = cases a { 
        //     H : T,
        //     _ : H,
        // };
        let mut diagnostics = Diagnostics::new();
        let symbol_table = vec![
            Symbol {
                name: "coin".to_string(),
                kind: SymbolKind::EntType,
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "H".to_string(),
                kind: SymbolKind::EntMember {
                    parent: 0,
                    mapping: 0,
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "T".to_string(),
                kind: SymbolKind::EntMember {
                    parent: 0,
                    mapping: 1,
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "REVERSE".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Custom(Ident::Symbol(0))],
                    return_type: Type::Custom(Ident::Symbol(0)),
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "a".to_string(),
                kind: SymbolKind::Variable(Type::Custom(Ident::Symbol(0))),
                span: Span{line: 0, col: 0},
            },
        ];

        let relation = RelType {
            name: Ident::Symbol(3),
            params: vec![
                Param {
                    name: Ident::Symbol(4),
                    param_type: Type::Custom(Ident::Symbol(0)),
                },
            ],
            return_type: Type::Custom(Ident::Symbol(0)),
            body: Expr::Cases(CasesExpr {
                scrutinee: Box::new(Expr::Ident(Ident::Symbol(4))),
                arms: vec![
                    CasesArm {
                        pattern: vec![
                            SimplePattern::Ident(Ident::Symbol(1)),
                        ],
                        expr: Expr::Ident(Ident::Symbol(2)),
                        arm_span: Span{line: 0, col: 0},
                    },
                    CasesArm {
                        pattern: vec![
                            SimplePattern::Default,
                        ],
                        expr: Expr::Ident(Ident::Symbol(1)),
                        arm_span: Span{line: 0, col: 0},
                    },
                ],
                expr_type: Type::Custom(Ident::Symbol(0)),
                span: Span{line: 0, col: 0},
            }),
        };

        let result = RelCompiler::compile(relation, &symbol_table, &mut diagnostics);

        let bytecode = assemble("
            IJNE o13 r2 i0
            MOV r3 i1
            JMP o10
            MOV r3 i0
            RET r3
        ").unwrap();

        assert_eq!(result, Some(CompiledRel {
            name: "REVERSE".to_string(),
            complexity: 0,
            bytecode,
        }));
    }

    #[test]
    fn block_and_sample() {
        // rel_t RND : (a: Int) -> Real {
        //     let b = 1;
        //     let c = (a + sample {
        //         0.5 : 3,
        //         _   : 4
        //     }) + b;

        //     c / 1.5;
        // };
        let mut diagnostics = Diagnostics::new();
        let symbol_table = vec![
            Symbol {
                name: "RND".to_string(),
                kind: SymbolKind::Rel_t {
                    input_types: vec![Type::Int],
                    return_type: Type::Real,
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "a".to_string(),
                kind: SymbolKind::Variable(Type::Int),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "b".to_string(),
                kind: SymbolKind::Variable(Type::Int),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "c".to_string(),
                kind: SymbolKind::Variable(Type::Int),
                span: Span{line: 0, col: 0},
            },
        ];

        let relation = RelType {
            name: Ident::Symbol(0),
            params: vec![
                Param {
                    name: Ident::Symbol(1),
                    param_type: Type::Int,
                },
            ],
            return_type: Type::Real,
            body: Expr::Block(BlockExpr {
                statements: vec![
                    Statement::Let(LetStatement {
                        name: Ident::Symbol(2),
                        expr: Expr::Literal(Literal::Int(1)),
                    }),
                    Statement::Let(LetStatement {
                        name: Ident::Symbol(3),
                        expr: Expr::Binary(BinaryExpr {
                            left: Box::new(Expr::Binary(BinaryExpr {
                                left: Box::new(Expr::Ident(Ident::Symbol(1))),
                                right: Box::new(Expr::Sample(SampleExpr {
                                    arms: vec![
                                        SampleArm {
                                            prob: Prob::Expr(Expr::Literal(Literal::Real(0.5))),
                                            expr: Expr::Literal(Literal::Int(3)),
                                            arm_span: Span{line: 0, col: 0},
                                        },
                                        SampleArm {
                                            prob: Prob::Default,
                                            expr: Expr::Literal(Literal::Int(4)),
                                            arm_span: Span{line: 0, col: 0},
                                        },
                                    ],
                                    expr_type: Type::Int,
                                    span: Span{line: 0, col: 0},
                                })),
                                op: BinaryOp::Add,
                                op_span: Span{line: 0, col: 0},
                                expr_type: Type::Int,
                            })),
                            right: Box::new(Expr::Ident(Ident::Symbol(2))),
                            op: BinaryOp::Add,
                            op_span: Span{line: 0, col: 0},
                            expr_type: Type::Int,
                        }),
                    }),
                ],
                expr: Box::new(Expr::Binary(BinaryExpr {
                    left: Box::new(Expr::Ident(Ident::Symbol(3))),
                    right: Box::new(Expr::Literal(Literal::Real(1.5))),
                    op: BinaryOp::Div,
                    op_span: Span{line: 0, col: 0},
                    expr_type: Type::Real,
                })),
                expr_type: Type::Real,
            }),
        };

        let result = RelCompiler::compile(relation, &symbol_table, &mut diagnostics);

        // r0: timestamp
        // r1: delay
        // r2: a
        // r3: b
        // r4: random var, c
        // r5: 1st element of cdf
        // r6: 2nd element of cdf
        // r7: ret of sample
        let bytecode = assemble("
            # let b = 1;
            MOV r3 i1

            # sample {}
            RND r4
            MOV r5 f0.5
            MOV r6 f1.0
            FJEQ o3 r6 f1.0
            ERR b4 r6
            FJGE o13 r4 r5
            MOV r7 i3
            JMP o15
            FJGE o10 r4 r6
            MOV r7 i4

            # (a + sample) + b
            IADD r7 r2 r7
            IADD r7 r7 r3

            # c / 1.5
            I2F r4 r7
            FDIV r4 r4 f1.5

            RET r4
        ").unwrap();

        assert_eq!(result, Some(CompiledRel {
            name: "RND".to_string(),
            complexity: 0,
            bytecode,
        }));
    }

    #[test]
    fn integrate_frontent_and_interpreter() {
        let code = "
        ent_t SET = {A, B, C};

        rel_t ROTATE: (in: SET) -> SET = {
            cases in {
                A : B,
                B : C,
                _ : A,
            }
        };

        rel_t NAND: (a: Bool, b: Bool) -> Bool = {
            let c = a & b;

            ~c
        };
        ";
        let mut diagnostics = Diagnostics::new();

        // Front end
        let tokens = Lexer::new(code, &mut diagnostics).tokenize();
        let program = Parser::new(tokens, &mut diagnostics).parse();
        let (validated_program, symbols) = SemAnalyzer::new(program, &mut diagnostics).analyze();

        // Back End
        let mut compiled_relations = Vec::new();
        for item in validated_program.items {
            if let Item::Rel(relation) = item {
                let compiled_relation = RelCompiler::compile(relation, &symbols, &mut diagnostics).unwrap();

                compiled_relations.push(compiled_relation);
            }
        } 
        
        // Interpreter
        let mut interpreter = RelInterpreter::new(compiled_relations);

        // ROTATE args
        let args_1 = vec![0];
        let args_2 = vec![1];
        let args_3 = vec![2];
        
        // NAND args
        let args_4 = vec![false as u64, false as u64];
        let args_5 = vec![true as u64, false as u64];
        let args_6 = vec![false as u64, true as u64];
        let args_7 = vec![true as u64, true as u64];
    

        let result_1 = interpreter.evaluate(0, &args_1, 0, 1);
        let result_2 = interpreter.evaluate(0, &args_2, 0, 1);
        let result_3 = interpreter.evaluate(0, &args_3, 0, 1);
        let result_4 = interpreter.evaluate(1, &args_4, 0, 1);
        let result_5 = interpreter.evaluate(1, &args_5, 0, 1);
        let result_6 = interpreter.evaluate(1, &args_6, 0, 1);
        let result_7 = interpreter.evaluate(1, &args_7, 0, 1);

        assert_eq!(result_1, 1);
        assert_eq!(result_2, 2);
        assert_eq!(result_3, 0);

        assert_eq!(result_4, true as u64);
        assert_eq!(result_5, true as u64);
        assert_eq!(result_6, true as u64);
        assert_eq!(result_7, false as u64);
    }
}
