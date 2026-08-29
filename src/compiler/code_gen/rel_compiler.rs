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

//! # rel_compiler
//!
//! compiles a single relation into bytecode
//!
//! ## Invariants
//!
//! - unary expr_type will always match the type of their sub-expression
//! - in binary expressions, literal sources will have always been converted to match the expr_type (see fold_expr:63)
//!
//! Author: Cole Francis

use std::collections::HashMap;

use super::intermediate_rep::{Instruction, Source};
use crate::compiler::ast::*;
use crate::compiler::compiled_rel::CompiledRel;
use crate::compiler::symbol::{Symbol, SymbolId, SymbolKind};
use crate::compiler::sem_analyzer::types::Type;
use crate::compiler::diagnostics::{Diagnostics, Span, CompilerError};

pub struct RelCompiler<'a> {
    reg_map: HashMap<SymbolId, usize>,
    reg_used: [bool; 64],

    rel_symbol_id: SymbolId,
    symbol_table: &'a [Symbol],
    diagnostics: &'a mut Diagnostics,
}

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
                self.reg_used[idx] = true;
            }
        }

        // call compile_expr

        // remove deadcode

        // convert intermediate rep to u8

        Some(CompiledRel {
            complexity: 0,
            bytecode: Vec::new(),
        })
    }

    // Returns the bytecode in intermediate representation, the source where the result is stored, and the type
    fn compile_expr(&mut self, expr: Expr) -> Option<(Vec<Instruction>, Source, Type)> {
        let mut bytecode: Vec<Instruction> = Vec::new();

        // Refer to check_expr verify... functions to make sure all cases are covered
        let (source, ret_type) = match expr {
            Expr::Literal(literal) => {
                match literal {
                    Literal::Bool(b) => (Source::Bool(b), Type::Bool),

                    Literal::Int(i) => (Source::Int(i), Type::Int),

                    Literal::Real(r) => (Source::Float(r), Type::Real),
                }
            }

            Expr::Ident(Ident::Symbol(id)) => {
                let reg = if let Some(reg) = self.reg_map.get(&id) {
                    *reg
                } else {
                    let reg = self.get_next_reg()?;
                    self.reg_map.insert(id, reg);
                    self.reg_used[reg] = true;
                    reg
                };
                let SymbolKind::Variable(ident_type) = self.symbol_table[id].kind.clone() else {
                    return None; // not reachable
                };
                (Source::RegVar(reg as usize), ident_type)
            }

            Expr::Unary(unary) => {
                match (unary.expr_type, unary.op) {
                    // a mod n == (-a) mod n
                    (Type::Mod(modulus), UnaryOp::Neg) => {
                        let (sub_bytecode, src, _) = self.compile_expr(*unary.expr)?;
                        bytecode.extend(sub_bytecode);

                        let dest = match src {
                            Source::RegInter(reg) => reg,
                            _ => {
                                let reg = self.get_next_reg()?;
                                self.reg_used[reg] = true;
                                reg
                            },
                        };

                        bytecode.push(Instruction::IMUL {
                            dest: dest,
                            src1: src,
                            src2: Source::Int(-1),
                        });

                        (Source::RegInter(dest), Type::Mod(modulus))
                    }

                    (Type::Int, UnaryOp::Neg) => {
                        let (sub_bytecode, src, _) = self.compile_expr(*unary.expr)?;
                        bytecode.extend(sub_bytecode);

                        let dest = match src {
                            Source::RegInter(reg) => reg,
                            _ => {
                                let reg = self.get_next_reg()?;
                                self.reg_used[reg] = true;
                                reg
                            },
                        };

                        bytecode.push(Instruction::IMUL {
                            dest: dest,
                            src1: src,
                            src2: Source::Int(-1),
                        });

                        (Source::RegInter(dest), Type::Int)
                    }

                    (Type::Real, UnaryOp::Neg) => {
                        let (sub_bytecode, src, _) = self.compile_expr(*unary.expr)?;
                        bytecode.extend(sub_bytecode);

                        let dest = match src {
                            Source::RegInter(reg) => reg,
                            _ => {
                                let reg = self.get_next_reg()?;
                                self.reg_used[reg] = true;
                                reg
                            },
                        };

                        bytecode.push(Instruction::FMUL {
                            dest: dest,
                            src1: src,
                            src2: Source::Float(-1.0),
                        });

                        (Source::RegInter(dest), Type::Real)
                    }

                    (Type::Bool, UnaryOp::BitNot) => {
                        let (sub_bytecode, src, sup_type) = self.compile_expr(*unary.expr)?;
                        bytecode.extend(sub_bytecode);

                        let dest = match src {
                            Source::RegInter(reg) => reg,
                            _ => {
                                let reg = self.get_next_reg()?;
                                self.reg_used[reg] = true;
                                reg
                            },
                        };

                        // Convert impulse to Bool by inserting bytecode
                        match sup_type {
                            Type::Impulse => {
                                // Combine IEQ with NOT for efficency
                                bytecode.push(Instruction::INE {
                                    dest: dest,
                                    src1: src,
                                    src2: Source::RegInter(0),
                                });

                                return Some((bytecode, Source::RegInter(dest), Type::Bool));
                            }
                            _ => {}
                        }

                        bytecode.push(Instruction::NOT {
                            dest: dest,
                            src: src,
                        });

                        (Source::RegInter(dest), Type::Bool)
                    }


                    _ => return None,
                }
            }

            Expr::Binary(binary) => {
                match (binary.expr_type, binary.op) {
                    (Type::Mod(modulus), BinaryOp::Add) => {
                        let (left_sub_bytecode, src1, _) = self.compile_expr(*binary.left)?;
                        let (right_sub_bytecode, src2, _) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_sub_bytecode);
                        bytecode.extend(right_sub_bytecode);

                        let dest = self.get_binary_dest(src1, src2)?;

                        bytecode.push(Instruction::IADD {
                            dest: dest,
                            src1: src1,
                            src2: src2,
                        });

                        (Source::RegInter(dest), Type::Mod(modulus))
                    }
                    (Type::Mod(modulus), BinaryOp::Sub) => {
                        let (left_sub_bytecode, src1, _) = self.compile_expr(*binary.left)?;
                        let (right_sub_bytecode, src2, _) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_sub_bytecode);
                        bytecode.extend(right_sub_bytecode);

                        let dest = self.get_binary_dest(src1, src2)?;

                        bytecode.push(Instruction::ISUB {
                            dest: dest,
                            src1: src1,
                            src2: src2,
                        });

                        (Source::RegInter(dest), Type::Mod(modulus))
                    }
                    (Type::Mod(modulus), BinaryOp::Mul) => {
                        let (left_sub_bytecode, src1, _) = self.compile_expr(*binary.left)?;
                        let (right_sub_bytecode, src2, _) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_sub_bytecode);
                        bytecode.extend(right_sub_bytecode);

                        let dest = self.get_binary_dest(src1, src2)?;

                        bytecode.push(Instruction::IMUL {
                            dest: dest,
                            src1: src1,
                            src2: src2,
                        });

                        (Source::RegInter(dest), Type::Mod(modulus))
                    }
                    (Type::Mod(modulus), BinaryOp::Div) => {
                        let (left_sub_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_sub_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_sub_bytecode);
                        bytecode.extend(right_sub_bytecode);

                        let src1 = self.coerce_int(&mut bytecode, src1, left_sub_type)?;
                        let src2 = self.coerce_int(&mut bytecode, src2, right_sub_type)?;

                        let dest = self.get_binary_dest(src1, src2)?;

                        bytecode.push(Instruction::IDIV {
                            dest: dest,
                            src1: src1,
                            src2: src2,
                        });

                        (Source::RegInter(dest), Type::Mod(modulus))
                    }
                    (Type::Mod(modulus), BinaryOp::Pow) => {
                        let (left_sub_bytecode, src1, _) = self.compile_expr(*binary.left)?;
                        let (right_sub_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_sub_bytecode);
                        bytecode.extend(right_sub_bytecode);

                        let src2 = self.coerce_int(&mut bytecode, src2, right_sub_type)?;

                        let dest = self.get_binary_dest(src1, src2)?;

                        bytecode.push(Instruction::IDIV {
                            dest: dest,
                            src1: src1,
                            src2: src2,
                        });

                        (Source::RegInter(dest), Type::Mod(modulus))
                    }
                    
                    (Type::Int, BinaryOp::Add) => {
                        let (left_sub_bytecode, src1, _) = self.compile_expr(*binary.left)?;
                        let (right_sub_bytecode, src2, _) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_sub_bytecode);
                        bytecode.extend(right_sub_bytecode);

                        let dest = self.get_binary_dest(src1, src2)?;

                        bytecode.push(Instruction::IADD {
                            dest: dest,
                            src1: src1,
                            src2: src2,
                        });

                        (Source::RegInter(dest), Type::Int)
                    }
                    (Type::Int, BinaryOp::Sub) => {
                        let (left_sub_bytecode, src1, _) = self.compile_expr(*binary.left)?;
                        let (right_sub_bytecode, src2, _) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_sub_bytecode);
                        bytecode.extend(right_sub_bytecode);

                        let dest = self.get_binary_dest(src1, src2)?;

                        bytecode.push(Instruction::ISUB {
                            dest: dest,
                            src1: src1,
                            src2: src2,
                        });

                        (Source::RegInter(dest), Type::Int)
                    }
                    (Type::Int, BinaryOp::Mul) => {
                        let (left_sub_bytecode, src1, _) = self.compile_expr(*binary.left)?;
                        let (right_sub_bytecode, src2, _) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_sub_bytecode);
                        bytecode.extend(right_sub_bytecode);

                        let dest = self.get_binary_dest(src1, src2)?;

                        bytecode.push(Instruction::IMUL {
                            dest: dest,
                            src1: src1,
                            src2: src2,
                        });

                        (Source::RegInter(dest), Type::Int)
                    }
                    (Type::Int, BinaryOp::Div) => {
                        let (left_sub_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_sub_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_sub_bytecode);
                        bytecode.extend(right_sub_bytecode);

                        let src1 = self.coerce_int(&mut bytecode, src1, left_sub_type)?;
                        let src2 = self.coerce_int(&mut bytecode, src2, right_sub_type)?;

                        let dest = self.get_binary_dest(src1, src2)?;

                        bytecode.push(Instruction::IDIV {
                            dest: dest,
                            src1: src1,
                            src2: src2,
                        });

                        (Source::RegInter(dest), Type::Int)
                    }
                    (Type::Int, BinaryOp::Pow) => {
                        let (left_sub_bytecode, src1, _) = self.compile_expr(*binary.left)?;
                        let (right_sub_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_sub_bytecode);
                        bytecode.extend(right_sub_bytecode);

                        let src2 = self.coerce_int(&mut bytecode, src2, right_sub_type)?;

                        let dest = self.get_binary_dest(src1, src2)?;

                        bytecode.push(Instruction::IPOW {
                            dest: dest,
                            src1: src1,
                            src2: src2,
                        });

                        (Source::RegInter(dest), Type::Int)
                    }
                    
                    (Type::Real, BinaryOp::Add) => {
                        let (left_sub_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_sub_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_sub_bytecode);
                        bytecode.extend(right_sub_bytecode);

                        let src1 = self.coerce_real(&mut bytecode, src1, left_sub_type, false)?;
                        let src2 = self.coerce_real(&mut bytecode, src2, right_sub_type, false)?;

                        let dest = self.get_binary_dest(src1, src2)?;

                        bytecode.push(Instruction::FADD {
                            dest: dest,
                            src1: src1,
                            src2: src2,
                        });

                        (Source::RegInter(dest), Type::Real)
                    }
                    (Type::Real, BinaryOp::Sub) => {
                        let (left_sub_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_sub_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_sub_bytecode);
                        bytecode.extend(right_sub_bytecode);

                        let src1 = self.coerce_real(&mut bytecode, src1, left_sub_type, false)?;
                        let src2 = self.coerce_real(&mut bytecode, src2, right_sub_type, false)?;

                        let dest = self.get_binary_dest(src1, src2)?;

                        bytecode.push(Instruction::FSUB {
                            dest: dest,
                            src1: src1,
                            src2: src2,
                        });

                        (Source::RegInter(dest), Type::Real)
                    }
                    (Type::Real, BinaryOp::Mul) => {
                        let (left_sub_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_sub_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_sub_bytecode);
                        bytecode.extend(right_sub_bytecode);

                        let src1 = self.coerce_real(&mut bytecode, src1, left_sub_type, false)?;
                        let src2 = self.coerce_real(&mut bytecode, src2, right_sub_type, false)?;

                        let dest = self.get_binary_dest(src1, src2)?;

                        bytecode.push(Instruction::FMUL {
                            dest: dest,
                            src1: src1,
                            src2: src2,
                        });

                        (Source::RegInter(dest), Type::Real)
                    }
                    (Type::Real, BinaryOp::Div) => {
                        let (left_sub_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_sub_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_sub_bytecode);
                        bytecode.extend(right_sub_bytecode);

                        let src1 = self.coerce_real(&mut bytecode, src1, left_sub_type, true)?;
                        let src2 = self.coerce_real(&mut bytecode, src2, right_sub_type, true)?;

                        let dest = self.get_binary_dest(src1, src2)?;

                        bytecode.push(Instruction::FDIV {
                            dest: dest,
                            src1: src1,
                            src2: src2,
                        });

                        (Source::RegInter(dest), Type::Real)
                    }
                    (Type::Real, BinaryOp::Pow) => {
                        let (left_sub_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_sub_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_sub_bytecode);
                        bytecode.extend(right_sub_bytecode);

                        let src1 = self.coerce_real(&mut bytecode, src1, left_sub_type, false)?;
                        let src2 = self.coerce_real(&mut bytecode, src2, right_sub_type, true)?;

                        let dest = self.get_binary_dest(src1, src2)?;

                        bytecode.push(Instruction::FPOW {
                            dest: dest,
                            src1: src1,
                            src2: src2,
                        });

                        (Source::RegInter(dest), Type::Real)
                    }

                    (Type::Bool, BinaryOp::Lt) => {
                        let (left_sub_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_sub_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_sub_bytecode);
                        bytecode.extend(right_sub_bytecode);

                        match (&left_sub_type, &right_sub_type) {
                            // if one is real, make both real
                            (&Type::Real, _) | (_, &Type::Real) => {
                                let src1 = self.coerce_real(&mut bytecode, src1, left_sub_type, true)?;
                                let src2 = self.coerce_real(&mut bytecode, src2, right_sub_type, true)?;

                                let dest = self.get_binary_dest(src1, src2)?;

                                bytecode.push(Instruction::FLT {
                                    dest: dest,
                                    src1: src1,
                                    src2: src2,
                                });

                                (Source::RegInter(dest), Type::Bool)
                            }

                            _ => {
                                let src1 = self.coerce_int(&mut bytecode, src1, left_sub_type)?;
                                let src2 = self.coerce_int(&mut bytecode, src2, right_sub_type)?;

                                let dest = self.get_binary_dest(src1, src2)?;

                                bytecode.push(Instruction::ILT {
                                    dest: dest,
                                    src1: src1,
                                    src2: src2,
                                });
                                
                                (Source::RegInter(dest), Type::Bool)
                            }
                        }
                    }
                    (Type::Bool, BinaryOp::Gt) => {
                        let (left_sub_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_sub_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_sub_bytecode);
                        bytecode.extend(right_sub_bytecode);

                        match (&left_sub_type, &right_sub_type) {
                            // if one is real, make both real
                            (&Type::Real, _) | (_, &Type::Real) => {
                                let src1 = self.coerce_real(&mut bytecode, src1, left_sub_type, true)?;
                                let src2 = self.coerce_real(&mut bytecode, src2, right_sub_type, true)?;

                                let dest = self.get_binary_dest(src1, src2)?;

                                bytecode.push(Instruction::FGT {
                                    dest: dest,
                                    src1: src1,
                                    src2: src2,
                                });

                                (Source::RegInter(dest), Type::Bool)
                            }

                            _ => {
                                let src1 = self.coerce_int(&mut bytecode, src1, left_sub_type)?;
                                let src2 = self.coerce_int(&mut bytecode, src2, right_sub_type)?;

                                let dest = self.get_binary_dest(src1, src2)?;

                                bytecode.push(Instruction::IGT {
                                    dest: dest,
                                    src1: src1,
                                    src2: src2,
                                });
                                
                                (Source::RegInter(dest), Type::Bool)
                            }
                        }
                    }
                    (Type::Bool, BinaryOp::Le) => {
                        let (left_sub_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_sub_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_sub_bytecode);
                        bytecode.extend(right_sub_bytecode);

                        match (&left_sub_type, &right_sub_type) {
                            // if one is real, make both real
                            (&Type::Real, _) | (_, &Type::Real) => {
                                let src1 = self.coerce_real(&mut bytecode, src1, left_sub_type, true)?;
                                let src2 = self.coerce_real(&mut bytecode, src2, right_sub_type, true)?;

                                let dest = self.get_binary_dest(src1, src2)?;

                                bytecode.push(Instruction::FLE {
                                    dest: dest,
                                    src1: src1,
                                    src2: src2,
                                });

                                (Source::RegInter(dest), Type::Bool)
                            }

                            _ => {
                                let src1 = self.coerce_int(&mut bytecode, src1, left_sub_type)?;
                                let src2 = self.coerce_int(&mut bytecode, src2, right_sub_type)?;

                                let dest = self.get_binary_dest(src1, src2)?;

                                bytecode.push(Instruction::ILE {
                                    dest: dest,
                                    src1: src1,
                                    src2: src2,
                                });
                                
                                (Source::RegInter(dest), Type::Bool)
                            }
                        }
                    }
                    (Type::Bool, BinaryOp::Ge) => {
                        let (left_sub_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_sub_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_sub_bytecode);
                        bytecode.extend(right_sub_bytecode);

                        match (&left_sub_type, &right_sub_type) {
                            // if one is real, make both real
                            (&Type::Real, _) | (_, &Type::Real) => {
                                let src1 = self.coerce_real(&mut bytecode, src1, left_sub_type, true)?;
                                let src2 = self.coerce_real(&mut bytecode, src2, right_sub_type, true)?;

                                let dest = self.get_binary_dest(src1, src2)?;

                                bytecode.push(Instruction::FGE {
                                    dest: dest,
                                    src1: src1,
                                    src2: src2,
                                });

                                (Source::RegInter(dest), Type::Bool)
                            }

                            _ => {
                                let src1 = self.coerce_int(&mut bytecode, src1, left_sub_type)?;
                                let src2 = self.coerce_int(&mut bytecode, src2, right_sub_type)?;

                                let dest = self.get_binary_dest(src1, src2)?;

                                bytecode.push(Instruction::IGE {
                                    dest: dest,
                                    src1: src1,
                                    src2: src2,
                                });
                                
                                (Source::RegInter(dest), Type::Bool)
                            }
                        }
                    }
                    (Type::Bool, BinaryOp::Or) => {
                        let (left_sub_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_sub_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_sub_bytecode);
                        bytecode.extend(right_sub_bytecode);

                        let src1 = self.coerce_bool(&mut bytecode, src1, left_sub_type)?;
                        let src2 = self.coerce_bool(&mut bytecode, src2, right_sub_type)?;

                        let dest = self.get_binary_dest(src1, src2)?;

                        bytecode.push(Instruction::OR {
                            dest,
                            src1,
                            src2,
                        });

                        (Source::RegInter(dest), Type::Bool)
                    }
                    (Type::Bool, BinaryOp::And) => {
                        let (left_sub_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_sub_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_sub_bytecode);
                        bytecode.extend(right_sub_bytecode);

                        let src1 = self.coerce_bool(&mut bytecode, src1, left_sub_type)?;
                        let src2 = self.coerce_bool(&mut bytecode, src2, right_sub_type)?;

                        let dest = self.get_binary_dest(src1, src2)?;

                        bytecode.push(Instruction::AND {
                            dest,
                            src1,
                            src2,
                        });

                        (Source::RegInter(dest), Type::Bool)
                    }

                    _ => return None,
                }
            }

            // Expr::Tuple(tuple) => {}

            // Expr::Block(block) => {}

            // Expr::Cases(cases) => {}

            // Expr::Sample(sample) => {}

            // Expr::Error => return None,

            _ => return None, // Temporary
        };

        Some((bytecode, source, ret_type))
    }

    // reports error and returns none if there are no registers left
    fn get_next_reg(&mut self) -> Option<usize> {
        let idx = self.reg_used.iter().position(|&x| !x);

        if idx.is_none() {
            self.diagnostics.error(CompilerError::TooManySymbols {
                rel_name: self.symbol_table[self.rel_symbol_id].name.clone(),
                rel_span: self.symbol_table[self.rel_symbol_id].span.clone(),
            });
        }

        idx
    }

    fn coerce_int(&mut self, bytecode: &mut Vec<Instruction>, src: Source, ty: Type) -> Option<Source> {
        match ty {
            Type::Mod(n) => {
                let dest = match src {
                    Source::RegInter(reg) => reg,
                    _ => {
                        let reg = self.get_next_reg()?;
                        self.reg_used[reg] = true;
                        reg
                    }
                };

                bytecode.push(Instruction::MOD {
                    dest,
                    src1: src,
                    src2: Source::Int(n),
                });

                Some(Source::RegInter(dest))
            }
            Type::Int => Some(src),
            _ => unreachable!("cannot coerde {:?} to Int", ty)
        }
    }

    // Reduce mod should be false for add/sub/mul because (a + b) mod n == (a mod n + b mod n) mod n but it is not true for div nor the right side of pow
    fn coerce_real(&mut self, bytecode: &mut Vec<Instruction>, src: Source, ty: Type, reduce_mod: bool) -> Option<Source> {
        match ty {
            Type::Mod(n) => {
                let src = if reduce_mod {
                    let dest = match src {
                        Source::RegInter(reg) => reg,
                        _ => {
                            let reg = self.get_next_reg()?;
                            self.reg_used[reg] = true;
                            reg
                        }
                    };

                    bytecode.push(Instruction::MOD {
                        dest,
                        src1: src,
                        src2: Source::Int(n),
                    });

                    Source::RegInter(dest)
                } else {
                    src
                };

                let dest = match src {
                    Source::RegInter(reg) => reg,
                    _ => {
                        let reg = self.get_next_reg()?;
                        self.reg_used[reg] = true;
                        reg
                    },
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
                    _ => {
                        let reg = self.get_next_reg()?;
                        self.reg_used[reg] = true;
                        reg
                    },
                };
                bytecode.push(Instruction::I2F {
                    dest,
                    src,
                });

                Some(Source::RegInter(dest))
            }
            Type::Real => Some(src),
            _ => unreachable!("cannot coerce {:?} to be Real", ty),
        }
    }

    // Converts all impulse type to bool with an IEQ
    fn coerce_bool (&mut self, bytecode: &mut Vec<Instruction>, src: Source, ty: Type, ) -> Option<Source> {
        match ty {
            Type::Impulse => {
                let dest = match src {
                    Source::RegInter(reg) => reg,
                    _ => {
                        let reg = self.get_next_reg()?;
                        self.reg_used[reg] = true;
                        reg
                    },
                };
                bytecode.push(Instruction::IEQ {
                    dest,
                    src1: src,
                    src2: Source::RegVar(0), // sim timestep is always in reg 0
                });
                Some(Source::RegInter(dest))
            }

            Type::Bool => Some(src),
            _ => unreachable!("cannot coerce {:?} into Bool", ty),
        }
    }

    fn get_binary_dest(&mut self, src1: Source, src2: Source) -> Option<usize> {
        match (src1, src2) {
            // When both are available, we should free one after the op
            (Source::RegInter(reg1), Source::RegInter(reg2)) => {
                self.reg_used[reg2] = false;
                Some(reg1)
            }, 
            (Source::RegInter(reg), _) => Some(reg),
            (_, Source::RegInter(reg)) => Some(reg),
            _ => {
                let reg = self.get_next_reg()?;
                self.reg_used[reg] = true;
                Some(reg)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_expr() {
        let mut diagnostics = Diagnostics::new();
        let symbol_table = Vec::new();
        let mut compiler = RelCompiler {
            reg_map: HashMap::new(),
            reg_used: [false; 64],
            rel_symbol_id: 0,
            symbol_table: &symbol_table,
            diagnostics: &mut diagnostics,
        };

        let ir = compiler.compile_expr(Expr::Literal(Literal::Int(3)));

        assert_eq!(ir, Some((vec![], Source::Int(3), Type::Int)));
    }

    #[test]
    fn ident_expr() {
        let mut diagnostics = Diagnostics::new();
        let symbol_table = vec![Symbol {
            name: "".to_string(),
            kind: SymbolKind::Variable(Type::Int),
            span: Span{line: 0, col: 0},
        }];
        let mut compiler = RelCompiler {
            reg_map: HashMap::new(),
            reg_used: [false; 64],
            rel_symbol_id: 1,
            symbol_table: &symbol_table,
            diagnostics: &mut diagnostics,
        };
        compiler.reg_map.insert(0, 0);
        compiler.reg_used[0] = true;

        let ir = compiler.compile_expr(Expr::Ident(Ident::Symbol(0)));

        assert_eq!(ir, Some((vec![], Source::RegVar(0), Type::Int)));
    }

    #[test]
    fn unary_expr_1() {
        // -(3)
        let mut diagnostics = Diagnostics::new();
        let symbol_table = Vec::new();
        let mut compiler = RelCompiler {
            reg_map: HashMap::new(),
            reg_used: [false; 64],
            rel_symbol_id: 0,
            symbol_table: &symbol_table,
            diagnostics: &mut diagnostics,
        };

        let ir = compiler.compile_expr(Expr::Unary(UnaryExpr {
            expr: Box::new(Expr::Literal(Literal::Int(3))),
            op: UnaryOp::Neg,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Int,
        }));

        assert_eq!(ir, Some((vec![
            Instruction::IMUL {
                dest: 0,
                src1: Source::Int(3),
                src2: Source::Int(-1),
            },
        ], Source::RegInter(0), Type::Int)));
    }

    #[test]
    fn unary_expr_2() {
        // ~(false)
        let mut diagnostics = Diagnostics::new();
        let symbol_table = Vec::new();
        let mut compiler = RelCompiler {
            reg_map: HashMap::new(),
            reg_used: [false; 64],
            rel_symbol_id: 0,
            symbol_table: &symbol_table,
            diagnostics: &mut diagnostics,
        };

        let ir = compiler.compile_expr(Expr::Unary(UnaryExpr {
            expr: Box::new(Expr::Literal(Literal::Bool(false))),
            op: UnaryOp::BitNot,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Bool,
        }));

        assert_eq!(ir, Some((vec![
            Instruction::NOT {
                dest: 0,
                src: Source::Bool(false),
            },
        ], Source::RegInter(0), Type::Bool)));
    }

    #[test]
    fn unary_expr_3() {
        // ~(false) (false is impulse)
        let mut diagnostics = Diagnostics::new();
        let symbol_table = vec![Symbol {
            name: "".to_string(),
            kind: SymbolKind::Variable(Type::Impulse),
            span: Span{line: 0, col: 0},
        }];
        let mut compiler = RelCompiler {
            reg_map: HashMap::new(),
            reg_used: [false; 64],
            rel_symbol_id: 0,
            symbol_table: &symbol_table,
            diagnostics: &mut diagnostics,
        };
        compiler.reg_used[0] = true; // storing sim timestep

        let ir = compiler.compile_expr(Expr::Unary(UnaryExpr {
            expr: Box::new(Expr::Ident(Ident::Symbol(0))),
            op: UnaryOp::BitNot,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Bool,
        }));

        assert_eq!(ir, Some((vec![
            Instruction::INE {
                dest: 2,
                src1: Source::RegVar(1),
                src2: Source::RegInter(0),
            },
        ], Source::RegInter(2), Type::Bool)));
    }

    #[test]
    fn binary_add_1() {
        // -(3.0) + -(a)
        let mut diagnostics = Diagnostics::new();
        let symbol_table = vec![Symbol {
            name: "".to_string(),
            kind: SymbolKind::Variable(Type::Int),
            span: Span{line: 0, col: 0},
        }];
        let mut compiler = RelCompiler {
            reg_map: HashMap::new(),
            reg_used: [false; 64],
            rel_symbol_id: 0,
            symbol_table: &symbol_table,
            diagnostics: &mut diagnostics,
        };
        compiler.reg_map.insert(0, 0);
        compiler.reg_used[0] = true;

        let ir = compiler.compile_expr(Expr::Binary(BinaryExpr{
            left: Box::new(Expr::Unary(UnaryExpr {
                expr: Box::new(Expr::Literal(Literal::Real(3.0))),
                op: UnaryOp::Neg,
                op_span: Span{line: 0, col: 0},
                expr_type: Type::Real,
            })),
            right: Box::new(Expr::Unary(UnaryExpr {
                expr: Box::new(Expr::Ident(Ident::Symbol(0))),
                op: UnaryOp::Neg,
                op_span: Span{line: 0, col: 0},
                expr_type: Type::Int,
            })),
            op: BinaryOp::Add,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Real,
        }));
            
        assert_eq!(ir, Some((vec![
            Instruction::FMUL {
                dest: 1,
                src1: Source::Float(3.0),
                src2: Source::Float(-1.0),
            },
            Instruction::IMUL {
                dest: 2,
                src1: Source::RegVar(0),
                src2: Source::Int(-1),
            },
            Instruction::I2F {
                dest: 2,
                src: Source::RegInter(2),
            },
            Instruction::FADD {
                dest: 1,
                src1: Source::RegInter(1),
                src2: Source::RegInter(2),
            },
        ], Source::RegInter(1), Type::Real)));
        assert_eq!(compiler.reg_used[0], true);
        assert_eq!(compiler.reg_used[1], true);
        assert_eq!(compiler.reg_used[2], false);
    }

    #[test]
    fn binary_div_1() {
        //  3 ^ a  (a is a mod 10)
        let mut diagnostics = Diagnostics::new();
        let symbol_table = vec![Symbol {
            name: "".to_string(),
            kind: SymbolKind::Variable(Type::Mod(10)),
            span: Span{line: 0, col: 0},
        }];
        let mut compiler = RelCompiler {
            reg_map: HashMap::new(),
            reg_used: [false; 64],
            rel_symbol_id: 0,
            symbol_table: &symbol_table,
            diagnostics: &mut diagnostics,
        };
        compiler.reg_map.insert(0, 1);
        compiler.reg_used[0] = true;
        compiler.reg_used[1] = true;

        let ir = compiler.compile_expr(Expr::Binary(BinaryExpr{
            left: Box::new(Expr::Literal(Literal::Int(3))),
            right: Box::new(Expr::Ident(Ident::Symbol(0))),
            op: BinaryOp::Pow,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Int,
        }));

        assert_eq!(ir, Some((vec![
            Instruction::MOD {
                dest: 2,
                src1: Source::RegVar(1),
                src2: Source::Int(10),
            },
            Instruction::IPOW {
                dest: 2,
                src1: Source::Int(3),
                src2: Source::RegInter(2),
            },
        ], Source::RegInter(2), Type::Int)));
    }

    #[test]
    fn binary_and_1() {
        // true and a  (a is an impulse)
        let mut diagnostics = Diagnostics::new();
        let symbol_table = vec![Symbol {
            name: "".to_string(),
            kind: SymbolKind::Variable(Type::Impulse),
            span: Span{line: 0, col: 0},
        }];
        let mut compiler = RelCompiler {
            reg_map: HashMap::new(),
            reg_used: [false; 64],
            rel_symbol_id: 0,
            symbol_table: &symbol_table,
            diagnostics: &mut diagnostics,
        };
        compiler.reg_map.insert(0, 1);
        compiler.reg_used[0] = true;
        compiler.reg_used[1] = true;

        let ir = compiler.compile_expr(Expr::Binary(BinaryExpr{
            left: Box::new(Expr::Literal(Literal::Bool(true))),
            right: Box::new(Expr::Ident(Ident::Symbol(0))),
            op: BinaryOp::And,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Bool,
        }));

        assert_eq!(ir, Some((vec![
            Instruction::IEQ {
                dest: 2,
                src1: Source::RegVar(1),
                src2: Source::RegVar(0),
            },
            Instruction::AND {
                dest: 2,
                src1: Source::Bool(true),
                src2: Source::RegInter(2),
            },
        ], Source::RegInter(2), Type::Bool)));
    }

    #[test]
    fn binary_cmp_1() {
        //  3 < a  (a is a mod 10)
        let mut diagnostics = Diagnostics::new();
        let symbol_table = vec![Symbol {
            name: "".to_string(),
            kind: SymbolKind::Variable(Type::Mod(10)),
            span: Span{line: 0, col: 0},
        }];
        let mut compiler = RelCompiler {
            reg_map: HashMap::new(),
            reg_used: [false; 64],
            rel_symbol_id: 0,
            symbol_table: &symbol_table,
            diagnostics: &mut diagnostics,
        };
        compiler.reg_map.insert(0, 1);
        compiler.reg_used[0] = true;
        compiler.reg_used[1] = true;

        let ir = compiler.compile_expr(Expr::Binary(BinaryExpr{
            left: Box::new(Expr::Literal(Literal::Int(3))),
            right: Box::new(Expr::Ident(Ident::Symbol(0))),
            op: BinaryOp::Lt,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Bool,
        }));

        assert_eq!(ir, Some((vec![
            Instruction::MOD {
                dest: 2,
                src1: Source::RegVar(1),
                src2: Source::Int(10),
            },
            Instruction::ILT {
                dest: 2,
                src1: Source::Int(3),
                src2: Source::RegInter(2),
            },
        ], Source::RegInter(2), Type::Bool)));
    }

    #[test]
    fn binary_cmp_2() {
        //  3.0 < a  (a is a mod 10)
        let mut diagnostics = Diagnostics::new();
        let symbol_table = vec![Symbol {
            name: "".to_string(),
            kind: SymbolKind::Variable(Type::Mod(10)),
            span: Span{line: 0, col: 0},
        }];
        let mut compiler = RelCompiler {
            reg_map: HashMap::new(),
            reg_used: [false; 64],
            rel_symbol_id: 0,
            symbol_table: &symbol_table,
            diagnostics: &mut diagnostics,
        };
        compiler.reg_map.insert(0, 1);
        compiler.reg_used[0] = true;
        compiler.reg_used[1] = true;

        let ir = compiler.compile_expr(Expr::Binary(BinaryExpr{
            left: Box::new(Expr::Literal(Literal::Real(3.0))),
            right: Box::new(Expr::Ident(Ident::Symbol(0))),
            op: BinaryOp::Lt,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Bool,
        }));

        assert_eq!(ir, Some((vec![
            Instruction::MOD {
                dest: 2,
                src1: Source::RegVar(1),
                src2: Source::Int(10),
            },
            Instruction::I2F {
                dest: 2,
                src: Source::RegInter(2),
            },
            Instruction::FLT {
                dest: 2,
                src1: Source::Float(3.0),
                src2: Source::RegInter(2),
            },
        ], Source::RegInter(2), Type::Bool)));
    }
}