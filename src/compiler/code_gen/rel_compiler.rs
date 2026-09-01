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

            // TODO: custom types appear as idents
            Expr::Ident(Ident::Symbol(id)) => {
                let Some(reg) = self.reg_map.get(&id) else {
                    return None;
                };
                let SymbolKind::Variable(ident_type) = self.symbol_table[id].kind.clone() else {
                    return None; // TODO: custom types appear as idents and will go to here
                };
                (Source::RegVar(*reg as usize), ident_type)
            }

            Expr::Unary(unary) => {
                match (unary.expr_type, unary.op) {
                    // a mod n == (-a) mod n
                    (Type::Mod(modulus), UnaryOp::Neg) => {
                        let (expr_bytecode, src, _) = self.compile_expr(*unary.expr)?;
                        bytecode.extend(expr_bytecode);

                        let dest = match src {
                            Source::RegInter(reg) => reg,
                            _ => self.get_next_reg()?,
                        };

                        bytecode.push(Instruction::IMUL {
                            dest: dest,
                            src1: src,
                            src2: Source::Int(-1),
                        });

                        (Source::RegInter(dest), Type::Mod(modulus))
                    }

                    (Type::Int, UnaryOp::Neg) => {
                        let (expr_bytecode, src, _) = self.compile_expr(*unary.expr)?;
                        bytecode.extend(expr_bytecode);

                        let dest = match src {
                            Source::RegInter(reg) => reg,
                            _ => self.get_next_reg()?,
                        };

                        bytecode.push(Instruction::IMUL {
                            dest: dest,
                            src1: src,
                            src2: Source::Int(-1),
                        });

                        (Source::RegInter(dest), Type::Int)
                    }

                    (Type::Real, UnaryOp::Neg) => {
                        let (expr_bytecode, src, _) = self.compile_expr(*unary.expr)?;
                        bytecode.extend(expr_bytecode);

                        let dest = match src {
                            Source::RegInter(reg) => reg,
                            _ => self.get_next_reg()?,
                        };

                        bytecode.push(Instruction::FMUL {
                            dest: dest,
                            src1: src,
                            src2: Source::Float(-1.0),
                        });

                        (Source::RegInter(dest), Type::Real)
                    }

                    (Type::Bool, UnaryOp::BitNot) => {
                        let (expr_bytecode, src, sup_type) = self.compile_expr(*unary.expr)?;
                        bytecode.extend(expr_bytecode);

                        let dest = match src {
                            Source::RegInter(reg) => reg,
                            _ => self.get_next_reg()?,
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
                        let (left_expr_bytecode, src1, _) = self.compile_expr(*binary.left)?;
                        let (right_expr_bytecode, src2, _) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_expr_bytecode);
                        bytecode.extend(right_expr_bytecode);

                        let dest = self.get_binary_dest(src1, src2)?;

                        bytecode.push(Instruction::IADD {
                            dest: dest,
                            src1: src1,
                            src2: src2,
                        });

                        (Source::RegInter(dest), Type::Mod(modulus))
                    }
                    (Type::Mod(modulus), BinaryOp::Sub) => {
                        let (left_expr_bytecode, src1, _) = self.compile_expr(*binary.left)?;
                        let (right_expr_bytecode, src2, _) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_expr_bytecode);
                        bytecode.extend(right_expr_bytecode);

                        let dest = self.get_binary_dest(src1, src2)?;

                        bytecode.push(Instruction::ISUB {
                            dest: dest,
                            src1: src1,
                            src2: src2,
                        });

                        (Source::RegInter(dest), Type::Mod(modulus))
                    }
                    (Type::Mod(modulus), BinaryOp::Mul) => {
                        let (left_expr_bytecode, src1, _) = self.compile_expr(*binary.left)?;
                        let (right_expr_bytecode, src2, _) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_expr_bytecode);
                        bytecode.extend(right_expr_bytecode);

                        let dest = self.get_binary_dest(src1, src2)?;

                        bytecode.push(Instruction::IMUL {
                            dest: dest,
                            src1: src1,
                            src2: src2,
                        });

                        (Source::RegInter(dest), Type::Mod(modulus))
                    }
                    (Type::Mod(modulus), BinaryOp::Div) => {
                        let (left_expr_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_expr_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_expr_bytecode);
                        bytecode.extend(right_expr_bytecode);

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
                        let (left_expr_bytecode, src1, _) = self.compile_expr(*binary.left)?;
                        let (right_expr_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_expr_bytecode);
                        bytecode.extend(right_expr_bytecode);

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
                        let (left_expr_bytecode, src1, _) = self.compile_expr(*binary.left)?;
                        let (right_expr_bytecode, src2, _) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_expr_bytecode);
                        bytecode.extend(right_expr_bytecode);

                        let dest = self.get_binary_dest(src1, src2)?;

                        bytecode.push(Instruction::IADD {
                            dest: dest,
                            src1: src1,
                            src2: src2,
                        });

                        (Source::RegInter(dest), Type::Int)
                    }
                    (Type::Int, BinaryOp::Sub) => {
                        let (left_expr_bytecode, src1, _) = self.compile_expr(*binary.left)?;
                        let (right_expr_bytecode, src2, _) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_expr_bytecode);
                        bytecode.extend(right_expr_bytecode);

                        let dest = self.get_binary_dest(src1, src2)?;

                        bytecode.push(Instruction::ISUB {
                            dest: dest,
                            src1: src1,
                            src2: src2,
                        });

                        (Source::RegInter(dest), Type::Int)
                    }
                    (Type::Int, BinaryOp::Mul) => {
                        let (left_expr_bytecode, src1, _) = self.compile_expr(*binary.left)?;
                        let (right_expr_bytecode, src2, _) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_expr_bytecode);
                        bytecode.extend(right_expr_bytecode);

                        let dest = self.get_binary_dest(src1, src2)?;

                        bytecode.push(Instruction::IMUL {
                            dest: dest,
                            src1: src1,
                            src2: src2,
                        });

                        (Source::RegInter(dest), Type::Int)
                    }
                    (Type::Int, BinaryOp::Div) => {
                        let (left_expr_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_expr_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_expr_bytecode);
                        bytecode.extend(right_expr_bytecode);

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
                        let (left_expr_bytecode, src1, _) = self.compile_expr(*binary.left)?;
                        let (right_expr_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_expr_bytecode);
                        bytecode.extend(right_expr_bytecode);

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
                        let (left_expr_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_expr_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_expr_bytecode);
                        bytecode.extend(right_expr_bytecode);

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
                        let (left_expr_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_expr_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_expr_bytecode);
                        bytecode.extend(right_expr_bytecode);

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
                        let (left_expr_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_expr_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_expr_bytecode);
                        bytecode.extend(right_expr_bytecode);

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
                        let (left_expr_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_expr_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_expr_bytecode);
                        bytecode.extend(right_expr_bytecode);

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
                        let (left_expr_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_expr_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_expr_bytecode);
                        bytecode.extend(right_expr_bytecode);

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
                        let (left_expr_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_expr_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_expr_bytecode);
                        bytecode.extend(right_expr_bytecode);

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
                        let (left_expr_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_expr_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_expr_bytecode);
                        bytecode.extend(right_expr_bytecode);

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
                        let (left_expr_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_expr_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_expr_bytecode);
                        bytecode.extend(right_expr_bytecode);

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
                        let (left_expr_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_expr_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_expr_bytecode);
                        bytecode.extend(right_expr_bytecode);

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
                        let (left_expr_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_expr_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_expr_bytecode);
                        bytecode.extend(right_expr_bytecode);

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
                        let (left_expr_bytecode, src1, left_sub_type) = self.compile_expr(*binary.left)?;
                        let (right_expr_bytecode, src2, right_sub_type) = self.compile_expr(*binary.right)?;

                        bytecode.extend(left_expr_bytecode);
                        bytecode.extend(right_expr_bytecode);

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

            Expr::Block(block) => {
                for statement in block.statements {
                    let Statement::Let(let_statement) = statement else {
                        return None;
                    };
                    let Ident::Symbol(id) = let_statement.name else {
                        return None;
                    };

                    let (expr_bytecode, src, sub_type) = self.compile_expr(let_statement.expr)?;

                    bytecode.extend(expr_bytecode);

                    match src {
                        Source::RegVar(reg) | Source::RegInter(reg) => {
                            self.reg_map.insert(id, reg);
                        },
                        _ => {
                            let dest = self.get_next_reg()?;
                            self.reg_map.insert(id, dest);

                            bytecode.push(Instruction::MOV {
                                dest,
                                src,
                            });
                        },
                    }
                }

                let (expr_bytecode, src, sub_type) = self.compile_expr(*block.expr)?;

                bytecode.extend(expr_bytecode);

                (src, sub_type)
            }

            // Expr::Cases(cases) => {
            //     let mut srcuitnee_sources = Vec::new();
            //     let mut scrutinee_types = Vec::new();

            //     if let Expr::Tuple(tuple_expr) = *cases.scrutinee {
            //         for expr in tuple_expr {
            //             let (expr_bytecode, src, sub_type) = self.compile_expr(expr)?;

            //             bytecode.extend(expr_bytecode);

            //             scrutinee_sources.push(src);
            //             scrutinee_types.push(sub_type);
            //         }
            //     } else {
            //         let (expr_bytecode, src, sub_type) = self.compile_expr(*cases.scrutinee)?;

            //         bytecode.extend(expr_bytecode);

            //         scrutinee_sources.push(src);
            //         scrutinee_types.push(sub_type);
            //     }

            //     for arm in cases.arms {
            //         // loop through all patterns. then append JMP inst to each and all patterns in the same arm jump to the same instruction
            //         for simple_pattern in arm.pattern {
            //             match simple_pattern {
            //                 SimplePattern::Tuple(tuple_pattern) => {},
            //                 _ => {},
            //             }
            //         }
            //     }

            //     // for something like  
            //     //     cases (a, b) {
            //     //         (2, 3) | (3, 4): 1, 
            //     //         (5, 6): 2,
            //     //         _: 3
            //     //     }, 
            //     // compile to this pseudocode:
            //     //     case_1:
            //     //         JNE case_2 a 2
            //     //         JNE case_2 b 3
            //     //         JMP shared_arm_code
            //     //     case_2:
            //     //         JNE case_3 a 3
            //     //         JNE case_3 b 4
            //     //     shared_arm_code:
            //     //         ...
            //     //         JMP end
            //     //     case_3:
            //     //         JNE end a 5
            //     //         JNE end b 6
            //     //         ...
            //     //         JMP end
            //     //     end:
            //     //         ...
            // }

            Expr::Sample(sample) => {
                // Generate random value
                let rnd = self.get_next_reg()?;

                bytecode.push(Instruction::RND {
                    dest: rnd,
                });

                // Because of ownership issues, we need to move arm expressions out
                //  so we can later loop through them
                let mut arm_exprs = Vec::with_capacity(sample.arms.len());

                // Build CDF, 
                let mut cdf: Vec<usize> = Vec::new();
                let mut last_prob_reg: Option<usize> = None;

                for arm in sample.arms {
                    let SampleArm {prob, expr, ..} = arm;
                    arm_exprs.push(expr);

                    let dest = match prob {
                        Prob::Expr(expr) => {
                            // probability expressions are not expected to be large, so we clone to avoid ownership issues
                            let (expr_bytecode, src, sub_type) = self.compile_expr(expr)?; 

                            bytecode.extend(expr_bytecode);

                            let src = self.coerce_real(&mut bytecode, src, sub_type, true)?;

                            match last_prob_reg {
                                Some(last_reg) => {
                                    let dest = match src {
                                        Source::RegInter(reg) => reg,
                                        _ => self.get_next_reg()?,
                                    };
                                    bytecode.push(Instruction::FADD {
                                        dest,
                                        src1: src,
                                        src2: Source::RegInter(last_reg),
                                    });

                                    dest
                                }
                                None => {
                                    match src {
                                        Source::RegInter(reg) => reg,
                                        _ => {
                                            let reg = self.get_next_reg()?;

                                            bytecode.push(Instruction::MOV {
                                                dest: reg,
                                                src,
                                            });

                                            reg
                                        }
                                    }
                                }
                            }
                        }
                        Prob::Default => {
                            let dest = self.get_next_reg()?;
                            bytecode.push(Instruction::MOV {
                                dest,
                                src: Source::Float(1.0),
                            });
                            dest
                        }
                    };

                    cdf.push(dest);
                    last_prob_reg = Some(dest);
                }

                // Verify cumulative prob is == 1.0
                let Some(last_prob_reg) = last_prob_reg else {
                    return None;
                };
                bytecode.push(Instruction::FJEQ {
                    offset: 3,
                    src1: Source::RegInter(last_prob_reg),
                    src2: Source::Float(1.0),
                });
                bytecode.push(Instruction::ERR {
                    code: 4,
                    src: Some(Source::RegInter(last_prob_reg)),
                });

                // preallocate destination reg
                let dest = self.get_next_reg()?;

                // compile arms one by one, skipping them if that arm is not chosen
                // select arm if rnd < cdf[i]
                // offsets are set to zero and afterwards we go back and fill in the offsets.
                let mut jmp_inst_indices: Vec<usize> = Vec::new();
                for (expr, cdf_val_reg) in arm_exprs.into_iter().zip(cdf.iter()) {
                    let (mut expr_bytecode, src, sub_type) = self.compile_expr(expr)?;

                    // free register moved into dest
                    if let Source::RegInter(src_reg) = src {
                        self.reg_used[src_reg] = false;
                    }
                    expr_bytecode.push(Instruction::MOV {
                        dest,
                        src,
                    });
                    expr_bytecode.push(Instruction::JMP {
                        offset: 0, // calculated later using jmp_inst_indices
                    });

                    let arm_expr_len = Self::get_num_bytes(&expr_bytecode);

                    bytecode.push(Instruction::FJGE {
                        offset: arm_expr_len as i16,
                        src1: Source::RegInter(rnd),
                        src2: Source::RegInter(*cdf_val_reg),
                    });

                    bytecode.extend(expr_bytecode);
                    jmp_inst_indices.push(bytecode.len()-1);
                }
                
                // go back and fill in offsets
                let target_inst_idx = bytecode.len();
                for idx in jmp_inst_indices {
                    let new_offset = Self::get_num_bytes(&bytecode[idx + 1..target_inst_idx]);

                    if let Instruction::JMP{ offset } = &mut bytecode[idx] {
                        *offset = new_offset as i16;
                    };
                }

                // release registers
                self.reg_used[rnd] = false;
                for reg in cdf {
                    self.reg_used[reg] = false;
                }

                (Source::RegInter(dest), sample.expr_type)
            }

            Expr::Error => return None,
            Expr::Tuple(_) => return None, // Only appear in cases where they're handled specially

            _ => return None, // Temporary
        };

        Some((bytecode, source, ret_type))
    }

    // Finds next available register, marks it as used, and returns its index
    // reports error and returns none if there are no registers left
    fn get_next_reg(&mut self) -> Option<usize> {
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

    fn coerce_int(&mut self, bytecode: &mut Vec<Instruction>, src: Source, ty: Type) -> Option<Source> {
        match ty {
            Type::Mod(n) => {
                let dest = match src {
                    Source::RegInter(reg) => reg,
                    _ => self.get_next_reg()?,
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
                        _ => self.get_next_reg()?,
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
            _ => unreachable!("cannot coerce {:?} to be Real", ty),
        }
    }

    // Converts all impulse type to bool with an IEQ
    fn coerce_bool (&mut self, bytecode: &mut Vec<Instruction>, src: Source, ty: Type, ) -> Option<Source> {
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
            _ => Some(self.get_next_reg()?),
        }
    }

    fn get_num_bytes(instrcutions: &[Instruction]) -> usize {
        let mut num = 0;
        for instruction in instrcutions {
            num += match instruction {
                Instruction::IADD{src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::ISUB{src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::IMUL{src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::IDIV{src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::IPOW{src1, src2, ..}  => 2 + Self::get_num_source_bytes(src1) + Self::get_num_source_bytes(src2),
                Instruction::IABS{src, ..}        => 2 + Self::get_num_source_bytes(src),
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
        compiler.reg_map.insert(0, 1);
        compiler.reg_used[0] = true; // storing sim timestep
        compiler.reg_used[1] = true;

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

    #[test]
    fn block_1() {
        // {
        //     let a = n; // n is a rel_t parameter
        //     let b = 1;

        //     a + b
        // }
        let mut diagnostics = Diagnostics::new();
        let symbol_table = vec![
            Symbol {
                name: "n".to_string(),
                kind: SymbolKind::Variable(Type::Int),
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
        let mut compiler = RelCompiler {
            reg_map: HashMap::new(),
            reg_used: [false; 64],
            rel_symbol_id: 0,
            symbol_table: &symbol_table,
            diagnostics: &mut diagnostics,
        };
        compiler.reg_map.insert(0, 0);
        compiler.reg_used[0] = true;

        let ir = compiler.compile_expr(Expr::Block(BlockExpr {
            statements: vec![
                Statement::Let(LetStatement {
                    name: Ident::Symbol(1),
                    expr: Expr::Ident(Ident::Symbol(0)),
                }),
                Statement::Let(LetStatement {
                    name: Ident::Symbol(2),
                    expr: Expr::Literal(Literal::Int(1)),
                }),
            ],
            expr: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Ident(Ident::Symbol(1))),
                right: Box::new(Expr::Ident(Ident::Symbol(2))),
                op: BinaryOp::Add,
                op_span: Span{line: 0, col: 0},
                expr_type: Type::Int,
            })),
            expr_type: Type::Int,
        }));

        assert_eq!(ir, Some((vec![
            Instruction::MOV {
                dest: 1,
                src: Source::Int(1),
            },
            Instruction::IADD {
                dest: 2,
                src1: Source::RegVar(0),
                src2: Source::RegVar(1),
            },
        ], Source::RegInter(2), Type::Int)));
    }

    #[test]
    fn sample_1() {
        // sample {
        //     a + 0.2 : 2
        //     0.6 : n
        //     _ : 4 - 2
        // }
        let mut diagnostics = Diagnostics::new();
        let symbol_table = vec![
            Symbol {
                name: "n".to_string(),
                kind: SymbolKind::Variable(Type::Int),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "a".to_string(),
                kind: SymbolKind::Variable(Type::Int),
                span: Span{line: 0, col: 0},
            },
        ];
        let mut compiler = RelCompiler {
            reg_map: HashMap::new(),
            reg_used: [false; 64],
            rel_symbol_id: 0,
            symbol_table: &symbol_table,
            diagnostics: &mut diagnostics,
        };
        compiler.reg_map.insert(0, 0);
        compiler.reg_used[0] = true;
        compiler.reg_map.insert(1, 1);
        compiler.reg_used[1] = true;

        let ir = compiler.compile_expr(Expr::Sample(SampleExpr {
            arms: vec![
                SampleArm {
                    prob: Prob::Expr(Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::Ident(Ident::Symbol(1))),
                        right: Box::new(Expr::Literal(Literal::Real(0.2))),
                        op: BinaryOp::Add,
                        op_span: Span{line: 0, col: 0},
                        expr_type: Type::Real,
                    })),
                    expr: Expr::Literal(Literal::Int(2)),
                    arm_span: Span{line: 0, col: 0},
                },
                SampleArm {
                    prob: Prob::Expr(Expr::Literal(Literal::Real(0.6))),
                    expr: Expr::Ident(Ident::Symbol(0)),
                    arm_span: Span{line: 0, col: 0},
                },
                SampleArm {
                    prob: Prob::Default,
                    expr: Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::Literal(Literal::Int(4))),
                        right: Box::new(Expr::Literal(Literal::Int(2))),
                        op: BinaryOp::Sub,
                        op_span: Span{line: 0, col: 0},
                        expr_type: Type::Int,
                    }),
                    arm_span: Span{line: 0, col: 0},
                },
            ],
            expr_type: Type::Int,
            span: Span{line: 0, col: 0},
        }));

        assert_eq!(ir, Some((vec![
            Instruction::RND {
                dest: 2
            },
            Instruction::I2F {
                dest: 3,
                src: Source::RegVar(1),
            },
            Instruction::FADD {
                dest: 3,
                src1: Source::RegInter(3),
                src2: Source::Float(0.2),
            },
            Instruction::FADD {
                dest: 4,
                src1: Source::Float(0.6),
                src2: Source::RegInter(3),
            },
            Instruction::MOV {
                dest: 5,
                src: Source::Float(1.0),
            },
            Instruction::FJEQ {
                offset: 3,
                src1: Source::RegInter(5),
                src2: Source::Float(1.0),
            },
            Instruction::ERR {
                code: 4,
                src: Some(Source::RegInter(5)),
            },
            Instruction::FJGE {
                offset: 13,
                src1: Source::RegInter(2),
                src2: Source::RegInter(3),
            },
            Instruction::MOV {
                dest: 6,
                src: Source::Int(2),
            },
            Instruction::JMP {
                offset: 40
            },
            Instruction::FJGE {
                offset: 6,
                src1: Source::RegInter(2),
                src2: Source::RegInter(4),
            },
            Instruction::MOV {
                dest: 6,
                src: Source::RegVar(0),
            },
            Instruction::JMP {
                offset: 29
            },
            Instruction::FJGE {
                offset: 24,
                src1: Source::RegInter(2),
                src2: Source::RegInter(5),
            },
            Instruction::ISUB {
                dest: 7,
                src1: Source::Int(4),
                src2: Source::Int(2),
            },
            Instruction::MOV {
                dest: 6,
                src: Source::RegInter(7),
            },
            Instruction::JMP {
                offset: 0
            },
        ], Source::RegInter(6), Type::Int)));
        assert_eq!(compiler.reg_used[0], true);  // "n"
        assert_eq!(compiler.reg_used[1], true);  // "a"
        assert_eq!(compiler.reg_used[2], false); // rnd
        assert_eq!(compiler.reg_used[3], false); // cdf[0]
        assert_eq!(compiler.reg_used[4], false); // cdf[1]
        assert_eq!(compiler.reg_used[5], false); // cdf[2]
        assert_eq!(compiler.reg_used[6], true);  // dest
    }
}