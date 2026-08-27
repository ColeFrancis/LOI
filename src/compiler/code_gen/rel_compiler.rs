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
use crate::compiler::symbol::SymbolId;
use crate::compiler::sem_analyzer::types::Type;
use crate::compiler::diagnostics::{Diagnostics, Span, CompilerError};

pub struct RelCompiler<'a> {
    reg_map: HashMap<SymbolId, usize>,
    reg_used: [bool; 64],

    rel_name: &'a str,
    rel_span: Span,
    diagnostics: &'a mut Diagnostics,
}

impl<'a> RelCompiler<'a> {
    pub fn compile(relation: RelType, diagnostics: &'a mut Diagnostics) -> Option<CompiledRel> {
        let mut compiler = Self {
            reg_map: HashMap::new(),
            reg_used: [false; 64],
            rel_name: "TODO", // TODO: figure out how to get name
            rel_span: Span{line: 0, col: 0}, // TODO: figure out how to get span
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

            Expr::Ident(Ident::Symbol(id)) => Source::Reg(id as usize), // how to get type?

            Expr::Unary(unary) => {
                match (unary.expr_type, unary.op) {
                    // a mod n == (-a) mod n
                    (Type::Mod(modulus), UnaryOp::Neg) => {
                        let (sub_bytecode, src) = self.compile_expr(*unary.expr)?;
                        bytecode.extend(sub_bytecode);

                        let dest = match src {
                            Source::Reg(reg) => reg,
                            _ => self.get_next_reg()?,
                        };

                        bytecode.push(Instruction::IMUL {
                            dest: dest,
                            src1: src,
                            src2: Source::Int(-1),
                        });

                        (Source::Reg(dest), Type::Mod(modulus))
                    }

                    (Type::Int, UnaryOp::Neg) => {
                        let (sub_bytecode, src) = self.compile_expr(*unary.expr)?;
                        bytecode.extend(sub_bytecode);

                        let dest = match src {
                            Source::Reg(reg) => reg,
                            _ => self.get_next_reg()?,
                        };

                        bytecode.push(Instruction::IMUL {
                            dest: dest,
                            src1: src,
                            src2: Source::Int(-1),
                        });

                        (Source::Reg(dest), Type::Int)
                    }

                    (Type::Real, UnaryOp::Neg) => {
                        let (sub_bytecode, src) = self.compile_expr(*unary.expr)?;
                        bytecode.extend(sub_bytecode);

                        let dest = match src {
                            Source::Reg(reg) => reg,
                            _ => self.get_next_reg()?,
                        };

                        bytecode.push(Instruction::FMUL {
                            dest: dest,
                            src1: src,
                            src2: Source::Float(-1.0),
                        });

                        (Source::Reg(dest), Type::Real)
                    }

                    (Type::Bool, UnaryOp::BitNot) => {
                        let (sub_bytecode, src) = self.compile_expr(*unary.expr)?;
                        bytecode.extend(sub_bytecode);

                        let dest = match src {
                            Source::Reg(reg) => reg,
                            _ => self.get_next_reg()?,
                        };

                        bytecode.push(Instruction::NOT {
                            dest: dest,
                            src: src,
                        });

                        Source::Reg(dest)
                    }
                    
                    // (Type::Impulase, UnaryOp::BitNot) => {}

                    _ => return None,
                }
            }

            // Expr::Binary(binary) => {}

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
                rel_name: self.rel_name.to_string(),
                rel_span: self.rel_span,
            });
        }

        idx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_empty_rel_type() -> RelType {
        RelType {
            name: Ident::Symbol(0),
            params: vec![],
            return_type: Type::Int,
            body: Expr::Error,
        }
    }

    #[test]
    fn compile_literal_expr() {
        let mut diagnostics = Diagnostics::new();
        let mut compiler = RelCompiler {
            reg_map: HashMap::new(),
            reg_used: [false; 64],
            rel_name: "", 
            rel_span: Span{line: 0, col: 0},
            diagnostics: &mut diagnostics,
        };

        let ir = compiler.compile_expr(Expr::Literal(Literal::Int(3)));

        assert_eq!(ir, Some((vec![], Source::Int(3))));
    }

    #[test]
    fn compile_ident_expr() {
        let mut diagnostics = Diagnostics::new();
        let mut compiler = RelCompiler {
            reg_map: HashMap::new(),
            reg_used: [false; 64],
            rel_name: "", 
            rel_span: Span{line: 0, col: 0},
            diagnostics: &mut diagnostics,
        };
        compiler.reg_map.insert(0, 0);
        compiler.reg_used[0] = true;

        let ir = compiler.compile_expr(Expr::Ident(Ident::Symbol(0)));

        assert_eq!(ir, Some((vec![], Source::Reg(0))));
    }

    #[test]
    fn compile_unary_expr_1() {
        let mut diagnostics = Diagnostics::new();
        let mut compiler = RelCompiler {
            reg_map: HashMap::new(),
            reg_used: [false; 64],
            rel_name: "", 
            rel_span: Span{line: 0, col: 0},
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
        ], Source::Reg(0))));
    }

    #[test]
    fn compile_unary_expr_2() {
        let mut diagnostics = Diagnostics::new();
        let mut compiler = RelCompiler {
            reg_map: HashMap::new(),
            reg_used: [false; 64],
            rel_name: "", 
            rel_span: Span{line: 0, col: 0},
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
        ], Source::Reg(0))));
    }
}