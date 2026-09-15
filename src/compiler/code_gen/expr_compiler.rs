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

//! # expr_compiler
//!
//! compiles a single expression into bytecode
//!
//! ## Invariants
//!
//! - unary expr_type will always match the type of their sub-expression
//! - in binary expressions, literal sources will have always been converted to match the expr_type (see fold_expr:63)
//!
//! Author: Cole Francis

use std::collections::HashMap;

use super::CodeGen;
use super::intermediate_rep::{Instruction, Source};
use crate::compiler::ast::*;
use crate::compiler::compiled_rel::CompiledRel;
use crate::compiler::symbol::{Symbol, SymbolId, SymbolKind};
use crate::compiler::sem_analyzer::types::Type;
use crate::compiler::diagnostics::{Diagnostics, Span, Diagnostic};

impl<'a> CodeGen<'a> {
    // Returns the bytecode in intermediate representation, the source where the result is stored, and the type
    pub(super) fn compile_expr(&mut self, expr: Expr) -> Option<(Vec<Instruction>, Source, Type)> {
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
                match self.symbol_table[id].kind.clone() {
                    SymbolKind::Variable(ident_type) => {
                        let Some(reg) = self.reg_map.get(&id) else {
                            return None;
                        };
                        (Source::RegVar(*reg as usize), ident_type)
                    },

                    SymbolKind::EntMember {parent, mapping} => {
                        (Source::Int(mapping as i64), Type::Custom(Ident::Symbol(parent)))
                    },

                    _ => return None,
                }
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

                        let src1 = self.coerce_int(&mut bytecode, src1, &left_sub_type)?;
                        let src2 = self.coerce_int(&mut bytecode, src2, &right_sub_type)?;

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

                        let src2 = self.coerce_int(&mut bytecode, src2, &right_sub_type)?;

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

                        let src1 = self.coerce_int(&mut bytecode, src1, &left_sub_type)?;
                        let src2 = self.coerce_int(&mut bytecode, src2, &right_sub_type)?;

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

                        let src2 = self.coerce_int(&mut bytecode, src2, &right_sub_type)?;

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

                        let src1 = self.coerce_real(&mut bytecode, src1, &left_sub_type, false)?;
                        let src2 = self.coerce_real(&mut bytecode, src2, &right_sub_type, false)?;

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

                        let src1 = self.coerce_real(&mut bytecode, src1, &left_sub_type, false)?;
                        let src2 = self.coerce_real(&mut bytecode, src2, &right_sub_type, false)?;

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

                        let src1 = self.coerce_real(&mut bytecode, src1, &left_sub_type, false)?;
                        let src2 = self.coerce_real(&mut bytecode, src2, &right_sub_type, false)?;

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

                        let src1 = self.coerce_real(&mut bytecode, src1, &left_sub_type, true)?;
                        let src2 = self.coerce_real(&mut bytecode, src2, &right_sub_type, true)?;

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

                        let src1 = self.coerce_real(&mut bytecode, src1, &left_sub_type, false)?;
                        let src2 = self.coerce_real(&mut bytecode, src2, &right_sub_type, true)?;

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
                                let src1 = self.coerce_real(&mut bytecode, src1, &left_sub_type, true)?;
                                let src2 = self.coerce_real(&mut bytecode, src2, &right_sub_type, true)?;

                                let dest = self.get_binary_dest(src1, src2)?;

                                bytecode.push(Instruction::FLT {
                                    dest: dest,
                                    src1: src1,
                                    src2: src2,
                                });

                                (Source::RegInter(dest), Type::Bool)
                            }

                            _ => {
                                let src1 = self.coerce_int(&mut bytecode, src1, &left_sub_type)?;
                                let src2 = self.coerce_int(&mut bytecode, src2, &right_sub_type)?;

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
                                let src1 = self.coerce_real(&mut bytecode, src1, &left_sub_type, true)?;
                                let src2 = self.coerce_real(&mut bytecode, src2, &right_sub_type, true)?;

                                let dest = self.get_binary_dest(src1, src2)?;

                                bytecode.push(Instruction::FGT {
                                    dest: dest,
                                    src1: src1,
                                    src2: src2,
                                });

                                (Source::RegInter(dest), Type::Bool)
                            }

                            _ => {
                                let src1 = self.coerce_int(&mut bytecode, src1, &left_sub_type)?;
                                let src2 = self.coerce_int(&mut bytecode, src2, &right_sub_type)?;

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
                                let src1 = self.coerce_real(&mut bytecode, src1, &left_sub_type, true)?;
                                let src2 = self.coerce_real(&mut bytecode, src2, &right_sub_type, true)?;

                                let dest = self.get_binary_dest(src1, src2)?;

                                bytecode.push(Instruction::FLE {
                                    dest: dest,
                                    src1: src1,
                                    src2: src2,
                                });

                                (Source::RegInter(dest), Type::Bool)
                            }

                            _ => {
                                let src1 = self.coerce_int(&mut bytecode, src1, &left_sub_type)?;
                                let src2 = self.coerce_int(&mut bytecode, src2, &right_sub_type)?;

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
                                let src1 = self.coerce_real(&mut bytecode, src1, &left_sub_type, true)?;
                                let src2 = self.coerce_real(&mut bytecode, src2, &right_sub_type, true)?;

                                let dest = self.get_binary_dest(src1, src2)?;

                                bytecode.push(Instruction::FGE {
                                    dest: dest,
                                    src1: src1,
                                    src2: src2,
                                });

                                (Source::RegInter(dest), Type::Bool)
                            }

                            _ => {
                                let src1 = self.coerce_int(&mut bytecode, src1, &left_sub_type)?;
                                let src2 = self.coerce_int(&mut bytecode, src2, &right_sub_type)?;

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

                        let src1 = self.coerce_bool(&mut bytecode, src1, &left_sub_type)?;
                        let src2 = self.coerce_bool(&mut bytecode, src2, &right_sub_type)?;

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

                        let src1 = self.coerce_bool(&mut bytecode, src1, &left_sub_type)?;
                        let src2 = self.coerce_bool(&mut bytecode, src2, &right_sub_type)?;

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

            Expr::Cases(cases) => {
                let mut scrutinee_sources = Vec::new();
                let mut scrutinee_types = Vec::new();

                // Compile scrutinee
                if let Expr::Tuple(tuple_expr) = *cases.scrutinee {
                    for expr in tuple_expr {
                        let (expr_bytecode, src, sub_type) = self.compile_expr(expr)?;

                        bytecode.extend(expr_bytecode);

                        scrutinee_sources.push(src);
                        scrutinee_types.push(sub_type);
                    }
                } else {
                    let (expr_bytecode, src, sub_type) = self.compile_expr(*cases.scrutinee)?;
                    
                    bytecode.extend(expr_bytecode);

                    scrutinee_sources.push(src);
                    scrutinee_types.push(sub_type);
                }

                // preallocate destination reg
                let dest = self.get_next_reg()?;
                let mut exit_jmp_indices: Vec<usize> = Vec::new();
                let mut last_cond_jmp_indices: Vec<usize> = Vec::new();

                for arm in cases.arms {
                    let mut enter_jmp_indices: Vec<usize> = Vec::new();

                    for simple_pattern in arm.pattern {
                        let (sub_bytecode, cond_jmp_indices) = self.compile_pattern_comp(&mut scrutinee_sources, &mut scrutinee_types, simple_pattern)?;

                        last_cond_jmp_indices = cond_jmp_indices.iter().map(|x| x + bytecode.len()).collect();

                        bytecode.extend(sub_bytecode);
                        enter_jmp_indices.push(bytecode.len() - 1);
                    }

                    // pop off last jmp
                    bytecode.pop();
                    enter_jmp_indices.pop();

                    // edit all enter_jmp_indices to land here
                    let target_inst_idx = bytecode.len();
                    for idx in enter_jmp_indices {
                        let new_offset = Self::get_num_bytes(&bytecode[idx + 1..target_inst_idx]);

                        if let Instruction::JMP{ offset } = &mut bytecode[idx] {
                            *offset = new_offset as i16;
                        };
                    }

                    let (mut expr_bytecode, src, sub_type) = self.compile_expr(arm.expr)?;

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

                    // modify last_cond_jmp_indices' offsets to skip arm bytecode, and 
                    let added_length = Self::get_num_bytes(&expr_bytecode) - 3; // subtract 3 bytes for the jmp that was poped off
                    for idx in &last_cond_jmp_indices {
                        Self::update_jmp_offset(&mut bytecode[*idx], added_length as i16);
                    }

                    bytecode.extend(expr_bytecode);
                    exit_jmp_indices.push(bytecode.len()-1);
                }

                // remove last unnecessary jmp
                bytecode.pop();
                exit_jmp_indices.pop();

                // correct last conditional jump's offset to correct for removed JMP
                for idx in &last_cond_jmp_indices {
                    Self::update_jmp_offset(&mut bytecode[*idx], -3);
                }

                // go back and fill in offsets
                let target_inst_idx = bytecode.len();
                for idx in exit_jmp_indices {
                    let new_offset = Self::get_num_bytes(&bytecode[idx + 1..target_inst_idx]);

                    Self::update_jmp_offset(&mut bytecode[idx], new_offset as i16);
                }

                // free registers from scrutinee
                for src in scrutinee_sources {
                    if let Source::RegInter(src_reg) = src {
                        self.reg_used[src_reg] = false;
                    }
                }

                (Source::RegInter(dest), cases.expr_type)
            }

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

                            let src = self.coerce_real(&mut bytecode, src, &sub_type, true)?;

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
                let mut last_comp_jmp_idx = 0;
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

                    last_comp_jmp_idx = bytecode.len() - 1;

                    bytecode.extend(expr_bytecode);
                    jmp_inst_indices.push(bytecode.len()-1);
                }

                // remove last unnecessary jmp
                bytecode.pop();
                jmp_inst_indices.pop();

                // correct last conditional jump's offset to correct for removed JMP
                Self::update_jmp_offset(&mut bytecode[last_comp_jmp_idx], -3);
                
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

    // series of comparison jumps, ending with an unconditional jmp, where their offsets point to right after the jmp
    // if scrutinee value is type converted, scrutinee_types is modified accordingly
    fn compile_pattern_comp(&mut self, scrutinee_sources: &mut [Source], scrutinee_types: &mut [Type], pattern: SimplePattern) -> Option<(Vec<Instruction>, Vec<usize>)> {
        let mut bytecode = Vec::new();
        let mut comp_jmp_incices = Vec::new();

        match pattern {
            // for default pattern, don't do anything
            SimplePattern::Default => {}

            // for literal pattern, if scrutinee value doesn't match literal, then skip jmp
            SimplePattern::Literal(literal) => {
                let (mut lit_src, mut lit_type) = match literal {
                    Literal::Bool(b) => (Source::Bool(b), Type::Bool),

                    Literal::Int(i) => (Source::Int(i), Type::Int),

                    Literal::Real(r) => (Source::Float(r), Type::Real),
                };

                (scrutinee_sources[0], lit_src, scrutinee_types[0]) = self.coerce_equal(&mut bytecode, scrutinee_sources[0], &scrutinee_types[0], lit_src, &lit_type)?;

                match scrutinee_types[0] {
                    Type::Real => {
                        bytecode.push(Instruction::FJNE {
                            offset: 3, // Jump to right past the
                            src1: scrutinee_sources[0],
                            src2: lit_src,
                        });
                    }

                    _ => {
                        bytecode.push(Instruction::IJNE {
                            offset: 3, // Jump to right past the
                            src1: scrutinee_sources[0],
                            src2: lit_src,
                        });
                    }
                }

                comp_jmp_incices.push(bytecode.len() - 1);
            }

            // for ident pattern, if scrutinee value doesn't match ident value, then skip jmp
            SimplePattern::Ident(Ident::Symbol(id)) => {
                let (mut ident_src, ident_type) = match self.symbol_table[id].kind.clone() {
                    SymbolKind::Variable(ident_type) => {
                        let Some(reg) = self.reg_map.get(&id) else {
                            return None;
                        };
                        (Source::RegVar(*reg as usize), ident_type)
                    },

                    SymbolKind::EntMember {mapping, ..} => {
                        (Source::Int(mapping as i64), Type::Int)
                    },

                    _ => return None,
                };

                (scrutinee_sources[0], ident_src, scrutinee_types[0]) = self.coerce_equal(&mut bytecode, scrutinee_sources[0], &scrutinee_types[0], ident_src, &ident_type)?;

                match scrutinee_types[0] {
                    Type::Real => bytecode.push(Instruction::FJNE {
                        offset: 3, // Jump to right past the
                        src1: scrutinee_sources[0],
                        src2: ident_src,
                    }),

                    _ => bytecode.push(Instruction::IJNE {
                        offset: 3, // Jump to right past the
                        src1: scrutinee_sources[0],
                        src2: ident_src,
                    }),
                }

                comp_jmp_incices.push(bytecode.len() - 1);
            }

            // for tuple pattern, there will be several comparisons, and if any arent met then skip jmp
            // nested tuples aren't allowed
            SimplePattern::Tuple(tuple_pattern) => {
                for ((scrutinee_source, scrutinee_type), simple_pattern) in scrutinee_sources.iter_mut().zip(scrutinee_types.iter_mut()).zip(tuple_pattern.into_iter()) {
                    let (mut sub_bytecode, mut indices) = self.compile_pattern_comp(
                        std::slice::from_mut(scrutinee_source), 
                        std::slice::from_mut(scrutinee_type), 
                        simple_pattern,
                    )?;

                    // pop jmp off of sub_bytecode
                    sub_bytecode.pop();

                    // in the current bytecode and list of indices, modify all conditional jumps by adding length in bytes of sub_bytecode
                    let sub_num_bytes = Self::get_num_bytes(&sub_bytecode);
                    for index in &comp_jmp_incices {
                        Self::update_jmp_offset(&mut bytecode[*index], sub_num_bytes as i16);
                    }

                    // update indices in new indices vector by adding length of old bytecode
                    for index in &mut indices {
                        *index += bytecode.len();
                    }

                    // push new indices to old indices vecyor
                    // push new bytecode to old bhtecode
                    comp_jmp_incices.extend(indices);
                    bytecode.extend(sub_bytecode);
                }
            }

            // for comparison pattern, if the scrutinee value doesn't match the comparison, then skip jmp
            SimplePattern::Comparison(comp_pattern) => {
                let (expr_bytecode, mut comp_src, mut comp_type) = self.compile_expr(*comp_pattern.expr)?;

                bytecode.extend(expr_bytecode);

                (scrutinee_sources[0], comp_src, scrutinee_types[0]) = self.coerce_equal(&mut bytecode, scrutinee_sources[0], &scrutinee_types[0], comp_src, &comp_type)?;

                match scrutinee_types[0] {
                    Type::Real => match comp_pattern.op {
                        CompOp::Lt => bytecode.push(Instruction::FJGE {
                            offset: 3, // Jump to right past the jmp inst
                            src1: scrutinee_sources[0],
                            src2: comp_src,
                        }),

                        CompOp::Gt => bytecode.push(Instruction::FJLE {
                            offset: 3, // Jump to right past the jmp inst
                            src1: scrutinee_sources[0],
                            src2: comp_src,
                        }),

                        CompOp::Le => bytecode.push(Instruction::FJGT {
                            offset: 3, // Jump to right past the jmp inst
                            src1: scrutinee_sources[0],
                            src2: comp_src,
                        }),

                        CompOp::Ge => bytecode.push(Instruction::FJLT {
                            offset: 3, // Jump to right past the jmp inst
                            src1: scrutinee_sources[0],
                            src2: comp_src,
                        }),
                    }

                    _ => match comp_pattern.op {
                        CompOp::Lt => bytecode.push(Instruction::IJGE {
                            offset: 3, // Jump to right past the jmp inst
                            src1: scrutinee_sources[0],
                            src2: comp_src,
                        }),

                        CompOp::Gt => bytecode.push(Instruction::IJLE {
                            offset: 3, // Jump to right past the jmp inst
                            src1: scrutinee_sources[0],
                            src2: comp_src,
                        }),

                        CompOp::Le => bytecode.push(Instruction::IJGT {
                            offset: 3, // Jump to right past the jmp inst
                            src1: scrutinee_sources[0],
                            src2: comp_src,
                        }),

                        CompOp::Ge => bytecode.push(Instruction::IJLT {
                            offset: 3, // Jump to right past the jmp inst
                            src1: scrutinee_sources[0],
                            src2: comp_src,
                        }),
                    }
                }

                comp_jmp_incices.push(bytecode.len() - 1);
            }

            _ => {},
        }

        bytecode.push(Instruction::JMP {
            offset: 0,
        });

        Some((bytecode, comp_jmp_incices))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_expr_1() {
        let mut diagnostics = Diagnostics::new();
        let symbol_table = Vec::new();
        let mut compiler = CodeGen {
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
    fn literal_expr_2() {
        // T // second member of ent_t coin = {H, T};
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
        ];
        let mut compiler = CodeGen {
            reg_map: HashMap::new(),
            reg_used: [false; 64],
            rel_symbol_id: 1,
            symbol_table: &symbol_table,
            diagnostics: &mut diagnostics,
        };

        let ir = compiler.compile_expr(Expr::Ident(Ident::Symbol(2)));

        assert_eq!(ir, Some((vec![], Source::Int(1), Type::Custom(Ident::Symbol(0)))));
    }

    #[test]
    fn ident_expr_1() {
        let mut diagnostics = Diagnostics::new();
        let symbol_table = vec![Symbol {
            name: "".to_string(),
            kind: SymbolKind::Variable(Type::Int),
            span: Span{line: 0, col: 0},
        }];
        let mut compiler = CodeGen {
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
    fn ident_expr_2() {
        // c   // type is  ent_t coin = {H, T};
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
                name: "c".to_string(),
                kind: SymbolKind::Variable(Type::Custom(Ident::Symbol(0))),
                span: Span{line: 0, col: 0},
            },
        ];
        let mut compiler = CodeGen {
            reg_map: HashMap::new(),
            reg_used: [false; 64],
            rel_symbol_id: 1,
            symbol_table: &symbol_table,
            diagnostics: &mut diagnostics,
        };
        compiler.reg_map.insert(3, 0);
        compiler.reg_used[0] = true;

        let ir = compiler.compile_expr(Expr::Ident(Ident::Symbol(3)));

        assert_eq!(ir, Some((vec![], Source::RegVar(0), Type::Custom(Ident::Symbol(0)))));
    }

    #[test]
    fn unary_expr_1() {
        // -(3)
        let mut diagnostics = Diagnostics::new();
        let symbol_table = Vec::new();
        let mut compiler = CodeGen {
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
        let mut compiler = CodeGen {
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
        let mut compiler = CodeGen {
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
        let mut compiler = CodeGen {
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
        let mut compiler = CodeGen {
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
            Instruction::IMOD {
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
        let mut compiler = CodeGen {
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
        let mut compiler = CodeGen {
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
            Instruction::IMOD {
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
        let mut compiler = CodeGen {
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
            Instruction::IMOD {
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
        let mut compiler = CodeGen {
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
        let mut compiler = CodeGen {
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
                offset: 37
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
                offset: 26
            },
            Instruction::FJGE {
                offset: 21,
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
        ], Source::RegInter(6), Type::Int)));
        assert_eq!(compiler.reg_used[0], true);  // "n"
        assert_eq!(compiler.reg_used[1], true);  // "a"
        assert_eq!(compiler.reg_used[2], false); // rnd
        assert_eq!(compiler.reg_used[3], false); // cdf[0]
        assert_eq!(compiler.reg_used[4], false); // cdf[1]
        assert_eq!(compiler.reg_used[5], false); // cdf[2]
        assert_eq!(compiler.reg_used[6], true);  // dest
    }

    #[test]
    fn compile_pattern_comp() {
        
        // cases (a, b, c, d, e) {    // a is int reg_var, b is int reg_inter, c is real reg_inter, d is mod(4) reg_inter, e is impuse reg_var
        //     (1, f, >5, 2, true) : ... // f is real, 2 is mod(4)
        // }
        let mut scrutinee_sources = vec![Source::RegVar(1), Source::RegInter(4), Source::RegInter(5), Source::RegInter(6), Source::RegVar(2)];
        let mut scrutinee_types = vec![Type::Int, Type::Int, Type::Real, Type::Mod(4), Type::Impulse];

        let mut diagnostics = Diagnostics::new();
        let symbol_table = vec![
            Symbol {
                name: "a".to_string(),
                kind: SymbolKind::Variable(Type::Int),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "e".to_string(),
                kind: SymbolKind::Variable(Type::Impulse),
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "f".to_string(),
                kind: SymbolKind::Variable(Type::Real),
                span: Span{line: 0, col: 0},
            },
        ];
        let mut compiler = CodeGen {
            reg_map: HashMap::new(),
            reg_used: [false; 64],
            rel_symbol_id: 0,
            symbol_table: &symbol_table,
            diagnostics: &mut diagnostics,
        };
        compiler.reg_used[0] = true; // sim_timestep
        compiler.reg_map.insert(0, 1); // a
        compiler.reg_used[1] = true;
        compiler.reg_map.insert(1, 2); // e
        compiler.reg_used[2] = true;
        compiler.reg_map.insert(2, 3); // f
        compiler.reg_used[3] = true;
        compiler.reg_used[4] = true; // b
        compiler.reg_used[5] = true; // c
        compiler.reg_used[6] = true; // d

        let result = compiler.compile_pattern_comp(&mut scrutinee_sources, &mut scrutinee_types, SimplePattern::Tuple(vec![
            SimplePattern::Literal(Literal::Int(1)),
            SimplePattern::Ident(Ident::Symbol(2)),
            SimplePattern::Comparison(ComparisonPattern {
                op: CompOp::Gt,
                expr: Box::new(Expr::Literal(Literal::Int(5))),
            }),
            SimplePattern::Literal(Literal::Int(2)),
            SimplePattern::Literal(Literal::Bool(true)),
        ]));

        // cases (a, b, c, d, e) {    // a is int reg_var, b is int reg_inter, c is real reg_inter, d is mod(4) reg_inter, e is impuse reg_var
        //     (1, f, >5, 2, true) : ... // f is real, 2 is mod(4)
        // }
        assert_eq!(result, Some((vec![
            Instruction::IJNE {
                offset: 65,
                src1: Source::RegVar(1),
                src2: Source::Int(1),
            },
            Instruction::I2F {
                dest: 4,
                src: Source::RegInter(4),
            },
            Instruction::FJNE {
                offset: 57,
                src1: Source::RegInter(4),
                src2: Source::RegVar(3),
            },
            Instruction::I2F {
                dest: 7,
                src: Source::Int(5),
            },
            Instruction::FJLE {
                offset: 42,
                src1: Source::RegInter(5),
                src2: Source::RegInter(7),
            },
            Instruction::IMOD {
                dest: 6,
                src1: Source::RegInter(6),
                src2: Source::Int(4),
            },
            Instruction::IJNE {
                offset: 19,
                src1: Source::RegInter(6),
                src2: Source::Int(2),
            },
            Instruction::IEQ {
                dest: 8,
                src1: Source::RegVar(2),
                src2: Source::RegVar(0),
            },
            Instruction::IJNE {
                offset: 3,
                src1: Source::RegInter(8),
                src2: Source::Bool(true),
            },
            Instruction::JMP {
                offset: 0
            },
        ], vec![0, 2, 4, 6, 8])));
        assert_eq!(scrutinee_types, vec![Type::Int, Type::Real, Type::Real, Type::Int, Type::Bool]);
    }

    #[test]
    fn cases_1() {
        // cases (a, b*2) {
        //     (1, 2) | (3, 4): 1, 
        //     (5, 6): 2,
        //     _: 3
        // }
        let mut diagnostics = Diagnostics::new();
        let symbol_table = vec![
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
        let mut compiler = CodeGen {
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

        let ir = compiler.compile_expr(Expr::Cases(CasesExpr {
            scrutinee: Box::new(Expr::Tuple(vec![
                Expr::Ident(Ident::Symbol(0)),
                Expr::Binary(BinaryExpr {
                    left: Box::new(Expr::Ident(Ident::Symbol(1))),
                    right: Box::new(Expr::Literal(Literal::Int(2))),
                    op: BinaryOp::Mul,
                    op_span: Span{line: 0, col: 0},
                    expr_type: Type::Int,
                }),
            ])),
            arms: vec![
                CasesArm {
                    pattern: vec![
                        SimplePattern::Tuple(vec![
                            SimplePattern::Literal(Literal::Int(1)),
                            SimplePattern::Literal(Literal::Int(2)),
                        ]),
                        SimplePattern::Tuple(vec![
                            SimplePattern::Literal(Literal::Int(3)),
                            SimplePattern::Literal(Literal::Int(4)),
                        ]),
                    ],
                    expr: Expr::Literal(Literal::Int(1)),
                    arm_span: Span{line: 0, col: 0} 
                },
                CasesArm {
                    pattern: vec![
                        SimplePattern::Tuple(vec![
                            SimplePattern::Literal(Literal::Int(5)),
                            SimplePattern::Literal(Literal::Int(6)),
                        ]),
                    ],
                    expr: Expr::Literal(Literal::Int(2)),
                    arm_span: Span{line: 0, col: 0} 
                },
                CasesArm {
                    pattern: vec![
                        SimplePattern::Default,
                    ],
                    expr: Expr::Literal(Literal::Int(3)),
                    arm_span: Span{line: 0, col: 0} 
                },
            ],
            expr_type: Type::Int,
            span: Span{line: 0, col: 0},
        }));

        assert_eq!(ir, Some((vec![
            Instruction::IMUL {
                dest: 2,
                src1: Source::RegVar(1),
                src2: Source::Int(2),
            },
            Instruction::IJNE {
                offset: 15, // past next JMP
                src1: Source::RegVar(0),
                src2: Source::Int(1),
            },
            Instruction::IJNE {
                offset: 3, // past next JMP
                src1: Source::RegInter(2),
                src2: Source::Int(2),
            },
            Instruction::JMP {
                offset: 24, // past next conditional jumps
            },
            Instruction::IJNE {
                offset: 25, // past next JMP
                src1: Source::RegVar(0),
                src2: Source::Int(3),
            },
            Instruction::IJNE {
                offset: 13, // past next JMP
                src1: Source::RegInter(2),
                src2: Source::Int(4),
            },
            Instruction::MOV {
                dest: 3,
                src: Source::Int(1),
            },
            Instruction::JMP {
                offset: 47, // After last inst
            },
            Instruction::IJNE {
                offset: 25, // past next JMP
                src1: Source::RegVar(0),
                src2: Source::Int(5),
            },
            Instruction::IJNE {
                offset: 13, // past next JMP
                src1: Source::RegInter(2),
                src2: Source::Int(6),
            },
            Instruction::MOV {
                dest: 3,
                src: Source::Int(2),
            },
            Instruction::JMP {
                offset: 10, // After last inst
            },
            Instruction::MOV {
                dest: 3,
                src: Source::Int(3),
            },
        ], Source::RegInter(3), Type::Int)));
        assert_eq!(compiler.reg_used[0], true);
        assert_eq!(compiler.reg_used[1], true);
        assert_eq!(compiler.reg_used[2], false);
        assert_eq!(compiler.reg_used[3], true);
    }

    #[test]
    fn cases_2() {
        // cases c { // type of c is ent_t coin = {H, T};
        //     H : true, 
        //     _ : false,
        // }
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
                name: "c".to_string(),
                kind: SymbolKind::Variable(Type::Custom(Ident::Symbol(0))),
                span: Span{line: 0, col: 0},
            },
        ];
        let mut compiler = CodeGen {
            reg_map: HashMap::new(),
            reg_used: [false; 64],
            rel_symbol_id: 0,
            symbol_table: &symbol_table,
            diagnostics: &mut diagnostics,
        };
        compiler.reg_map.insert(3, 0);
        compiler.reg_used[0] = true;

        let ir = compiler.compile_expr(Expr::Cases(CasesExpr {
            scrutinee: Box::new(Expr::Ident(Ident::Symbol(3))),
            arms: vec![
                CasesArm {
                    pattern: vec![
                        SimplePattern::Ident(Ident::Symbol(1)),
                    ],
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span{line: 0, col: 0} 
                },
                CasesArm {
                    pattern: vec![
                        SimplePattern::Default,
                    ],
                    expr: Expr::Literal(Literal::Bool(false)),
                    arm_span: Span{line: 0, col: 0} 
                },
            ],
            expr_type: Type::Bool,
            span: Span{line: 0, col: 0},
        }));

        assert_eq!(ir, Some((vec![
            Instruction::IJNE {
                offset: 13,
                src1: Source::RegVar(0),
                src2: Source::Int(0),
            },
            Instruction::MOV {
                dest: 1,
                src: Source::Bool(true),
            },
            Instruction::JMP {
                offset: 10,
            },
            Instruction::MOV {
                dest: 1,
                src: Source::Bool(false),
            },
        ], Source::RegInter(1), Type::Bool)));
    }

    #[test]
    fn cases_3() {
        // cases var {
        //     A: sample {
        //         0.5: A,
        //         0.3: B,
        //         _  : C,
        //     },
        //     B: sample {
        //         0.1: A,
        //         0.4: B,
        //         _  : C,
        //     },
        //     C: sample {
        //         0.4: A,
        //         0.0: B,
        //         _  : C,
        //     },
        // }
        let mut diagnostics = Diagnostics::new();
        let symbol_table = vec![
            Symbol {
                name: "trio".to_string(),
                kind: SymbolKind::EntType,
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "A".to_string(),
                kind: SymbolKind::EntMember {
                    parent: 0,
                    mapping: 0,
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "B".to_string(),
                kind: SymbolKind::EntMember {
                    parent: 0,
                    mapping: 1,
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "C".to_string(),
                kind: SymbolKind::EntMember {
                    parent: 0,
                    mapping: 2,
                },
                span: Span{line: 0, col: 0},
            },
            Symbol {
                name: "var".to_string(),
                kind: SymbolKind::Variable(Type::Custom(Ident::Symbol(0))),
                span: Span{line: 0, col: 0},
            },
        ];
        let mut compiler = CodeGen {
            reg_map: HashMap::new(),
            reg_used: [false; 64],
            rel_symbol_id: 0,
            symbol_table: &symbol_table,
            diagnostics: &mut diagnostics,
        };
        compiler.reg_map.insert(4, 0);
        compiler.reg_used[0] = true;

        let ir = compiler.compile_expr(Expr::Cases(CasesExpr {
            scrutinee: Box::new(Expr::Ident(Ident::Symbol(4))),
            arms: vec![
                CasesArm {
                    pattern: vec![
                        SimplePattern::Ident(Ident::Symbol(1)),
                    ],
                    expr: Expr::Sample(SampleExpr {
                        arms: vec![
                            SampleArm {
                                prob: Prob::Expr(Expr::Literal(Literal::Real(0.5))),
                                expr: Expr::Ident(Ident::Symbol(1)),
                                arm_span: Span{line: 0, col: 0},
                            },
                            SampleArm {
                                prob: Prob::Expr(Expr::Literal(Literal::Real(0.3))),
                                expr: Expr::Ident(Ident::Symbol(2)),
                                arm_span: Span{line: 0, col: 0},
                            },
                            SampleArm {
                                prob: Prob::Default,
                                expr: Expr::Ident(Ident::Symbol(3)),
                                arm_span: Span{line: 0, col: 0},
                            },
                        ],
                        expr_type: Type::Custom(Ident::Symbol(0)),
                        span: Span{line: 0, col: 0},
                    }),
                    arm_span: Span{line: 0, col: 0} 
                },
                CasesArm {
                    pattern: vec![
                        SimplePattern::Ident(Ident::Symbol(2)),
                    ],
                    expr: Expr::Sample(SampleExpr {
                        arms: vec![
                            SampleArm {
                                prob: Prob::Expr(Expr::Literal(Literal::Real(0.1))),
                                expr: Expr::Ident(Ident::Symbol(1)),
                                arm_span: Span{line: 0, col: 0},
                            },
                            SampleArm {
                                prob: Prob::Expr(Expr::Literal(Literal::Real(0.4))),
                                expr: Expr::Ident(Ident::Symbol(2)),
                                arm_span: Span{line: 0, col: 0},
                            },
                            SampleArm {
                                prob: Prob::Default,
                                expr: Expr::Ident(Ident::Symbol(3)),
                                arm_span: Span{line: 0, col: 0},
                            },
                        ],
                        expr_type: Type::Custom(Ident::Symbol(0)),
                        span: Span{line: 0, col: 0},
                    }),
                    arm_span: Span{line: 0, col: 0} 
                },
                CasesArm {
                    pattern: vec![
                        SimplePattern::Ident(Ident::Symbol(3)),
                    ],
                    expr: Expr::Sample(SampleExpr {
                        arms: vec![
                            SampleArm {
                                prob: Prob::Expr(Expr::Literal(Literal::Real(0.4))),
                                expr: Expr::Ident(Ident::Symbol(1)),
                                arm_span: Span{line: 0, col: 0},
                            },
                            SampleArm {
                                prob: Prob::Expr(Expr::Literal(Literal::Real(0.0))),
                                expr: Expr::Ident(Ident::Symbol(2)),
                                arm_span: Span{line: 0, col: 0},
                            },
                            SampleArm {
                                prob: Prob::Default,
                                expr: Expr::Ident(Ident::Symbol(3)),
                                arm_span: Span{line: 0, col: 0},
                            },
                        ],
                        expr_type: Type::Custom(Ident::Symbol(0)),
                        span: Span{line: 0, col: 0},
                    }),
                    arm_span: Span{line: 0, col: 0} 
                },
            ],
            expr_type: Type::Custom(Ident::Symbol(0)),
            span: Span{line: 0, col: 0},
        }));

        assert_eq!(ir, Some((vec![
            Instruction::IJNE {
                offset: 105,
                src1: Source::RegVar(0),
                src2: Source::Int(0),
            },
            Instruction::RND {
                dest: 2,
            },
            Instruction::MOV {
                dest: 3,
                src: Source::Float(0.5),
            },
            Instruction::FADD {
                dest: 4,
                src1: Source::Float(0.3),
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
                offset:  13,
                src1: Source::RegInter(2),
                src2: Source::RegInter(3),
            },
            Instruction::MOV {
                dest: 6,
                src: Source::Int(0),
            },
            Instruction::JMP {
                offset: 33,
            },
            Instruction::FJGE {
                offset: 13,
                src1: Source::RegInter(2),
                src2: Source::RegInter(4),
            },
            Instruction::MOV {
                dest: 6,
                src: Source::Int(1),
            },
            Instruction::JMP {
                offset: 15,
            },
            Instruction::FJGE {
                offset: 10,
                src1: Source::RegInter(2),
                src2: Source::RegInter(5),
            },
            Instruction::MOV {
                dest: 6,
                src: Source::Int(2),
            },
            Instruction::MOV {
                dest: 1,
                src: Source::RegInter(6),
            },
            Instruction::JMP {
                offset: 231,
            },


            Instruction::IJNE {
                offset: 105,
                src1: Source::RegVar(0),
                src2: Source::Int(1),
            },
            Instruction::RND {
                dest: 2,
            },
            Instruction::MOV {
                dest: 3,
                src: Source::Float(0.1),
            },
            Instruction::FADD {
                dest: 4,
                src1: Source::Float(0.4),
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
                offset:  13,
                src1: Source::RegInter(2),
                src2: Source::RegInter(3),
            },
            Instruction::MOV {
                dest: 6,
                src: Source::Int(0),
            },
            Instruction::JMP {
                offset: 33,
            },
            Instruction::FJGE {
                offset: 13,
                src1: Source::RegInter(2),
                src2: Source::RegInter(4),
            },
            Instruction::MOV {
                dest: 6,
                src: Source::Int(1),
            },
            Instruction::JMP {
                offset: 15,
            },
            Instruction::FJGE {
                offset: 10,
                src1: Source::RegInter(2),
                src2: Source::RegInter(5),
            },
            Instruction::MOV {
                dest: 6,
                src: Source::Int(2),
            },
            Instruction::MOV {
                dest: 1,
                src: Source::RegInter(6),
            },
            Instruction::JMP {
                offset: 114,
            },
            Instruction::IJNE {
                offset: 102,
                src1: Source::RegVar(0),
                src2: Source::Int(2),
            },
            Instruction::RND {
                dest: 2,
            },
            Instruction::MOV {
                dest: 3,
                src: Source::Float(0.4),
            },
            Instruction::FADD {
                dest: 4,
                src1: Source::Float(0.0),
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
                offset:  13,
                src1: Source::RegInter(2),
                src2: Source::RegInter(3),
            },
            Instruction::MOV {
                dest: 6,
                src: Source::Int(0),
            },
            Instruction::JMP {
                offset: 33,
            },
            Instruction::FJGE {
                offset: 13,
                src1: Source::RegInter(2),
                src2: Source::RegInter(4),
            },
            Instruction::MOV {
                dest: 6,
                src: Source::Int(1),
            },
            Instruction::JMP {
                offset: 15,
            },
            Instruction::FJGE {
                offset: 10,
                src1: Source::RegInter(2),
                src2: Source::RegInter(5),
            },
            Instruction::MOV {
                dest: 6,
                src: Source::Int(2),
            },
            Instruction::MOV {
                dest: 1,
                src: Source::RegInter(6),
            },
        ], Source::RegInter(1), Type::Custom(Ident::Symbol(0)))));
        assert_eq!(compiler.reg_used[0], true);
        assert_eq!(compiler.reg_used[1], true); 
        assert_eq!(compiler.reg_used[2], false);
        assert_eq!(compiler.reg_used[3], false);
        assert_eq!(compiler.reg_used[4], false);
        assert_eq!(compiler.reg_used[5], false);
        assert_eq!(compiler.reg_used[6], false);  
    }

    // test adding two cases together
    // corner cases with mod/impulse/custom type
}