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

//! # fold_again
//!
//! lightweight version of fold_expr from sem_analyzer for re-folding expressions after algebraic transformation
//!
//! ## Invariants
//!
//! - Returned expression must always have the same behavior as inputed function (excepting neglegable differences in outputted floating point values)
//!
//! Author: Cole Francis

use super::CodeGen;
use crate::compiler::ast::*;
    use crate::compiler::diagnostics::Span;
use crate::compiler::compiled_rel::CompiledRel;
use crate::compiler::sem_analyzer::SemAnalyzer;

impl CodeGen {
    // pub(super) fn fold_expr (&mut self, expr: Expr, fold_sample: bool) -> Expr {
    //     match expr {
    //         Expr::Literal(literal) => Expr::Literal(literal),

    //         Expr::Ident(Ident::Symbol(id)) => match &self.symbols[id].kind {
    //             SymbolKind::Const(literal) => Expr::Literal(literal.clone()),
    //             _ => Expr::Ident(Ident::Symbol(id)),
    //         }

    //         Expr::Unary(mut unary) => {
    //             unary.expr = Box::new(self.fold_expr(*unary.expr, fold_sample));

    //             if let Expr::Literal(literal) = &*unary.expr {
    //                 if let Some(result) = self.eval_unary(&unary.op, literal) {
    //                     return Expr::Literal(result);
    //                 }
    //             }

    //             Expr::Unary(unary)
    //         }

    //         Expr::Binary(mut binary) => {
    //             let mut expr_left = self.fold_expr(*binary.left, fold_sample);
    //             let mut expr_right = self.fold_expr(*binary.right, fold_sample);

    //             // if you have int + real convert to real + real to simplify eval_binary
    //             if binary.expr_type == Type::Real {
    //                 if let Expr::Literal(Literal::Int(val)) = expr_left {
    //                     expr_left = Expr::Literal(Literal::Real(val as f64));
    //                 }

    //                 if let Expr::Literal(Literal::Int(val)) = expr_right {
    //                     expr_right = Expr::Literal(Literal::Real(val as f64));
    //                 }
    //             }

    //             binary.left = Box::new(expr_left);
    //             binary.right = Box::new(expr_right);

    //             if let (Expr::Literal(left), Expr::Literal(right)) = (&*binary.left, &*binary.right) {
    //                 if let Some(result) = self.eval_binary(&binary.op, left, right,binary.op_span) {
    //                     return Expr::Literal(result);
    //                 }
    //             }

    //             Expr::Binary(binary)
    //         }

    //         Expr::Tuple(mut exprs) => {
    //             for expr in &mut exprs {
    //                 let owned_expr = std::mem::replace(expr, Expr::Error);

    //                 *expr = self.fold_expr(owned_expr, fold_sample);
    //             }

    //             Expr::Tuple(exprs)
    //         }

    //         Expr::Block(mut block_expr) => {
    //             let owned_statements = std::mem::take(&mut block_expr.statements);

    //             for statement in owned_statements {
    //                 if let Statement::Let(let_statement) = statement {
    //                     if let Some(folded_let_statement) = self.fold_let(let_statement, fold_sample) {
    //                         block_expr.statements.push(Statement::Let(folded_let_statement));
    //                     }
    //                 }
    //             }

    //             match self.fold_expr(*block_expr.expr, fold_sample) {
    //                 Expr::Literal(literal) => Expr::Literal(literal),

    //                 other => {
    //                     block_expr.expr = Box::new(other);

    //                     Expr::Block(block_expr)
    //                 }
    //             }
    //         }

    //         Expr::Cases(mut cases_expr) => {
    //             // Check for duplicate arms
    //             let mut has_errors = false;
    //             let mut seen_patterns: Vec<(&SimplePattern, Span)> = Vec::new();
    //             for arm in &mut cases_expr.arms {
    //                 for pattern in &mut arm.pattern {
    //                     let owned_pattern = std::mem::replace(pattern, SimplePattern::Error);
    //                     *pattern = self.fold_pattern(owned_pattern, fold_sample);

    //                     if let Some((_, old_span)) = seen_patterns.iter().find(|(p, _)| *p == pattern) {
    //                         self.diagnostics.error(CompilerError::DuplicatePattern {
    //                             old_arm_span: old_span.clone(),
    //                             arm_span: arm.arm_span,
    //                         });

    //                         has_errors = true;
    //                     }

    //                     seen_patterns.push((pattern, arm.arm_span));
    //                 }
    //             }
    //             if has_errors {
    //                 return Expr::Error;
    //             }

    //             cases_expr.scrutinee = Box::new(self.fold_expr(*cases_expr.scrutinee, fold_sample));

    //             let mut foldable = true;
    //             for arm in &mut cases_expr.arms {
    //                 let owned_expr = std::mem::replace(&mut arm.expr, Expr::Error);
    //                 arm.expr = self.fold_expr(owned_expr, fold_sample);

    //                 let matches = arm.pattern.iter_mut().any(|pattern| {
    //                     match Self::expr_matches_pattern(&cases_expr.scrutinee, pattern){
    //                         Some(res) => res,
    //                         None => {
    //                             foldable = false;
    //                             false
    //                         }
    //                     }
    //                 });

    //                 if matches && foldable {
    //                     return std::mem::replace(&mut arm.expr, Expr::Error);
    //                 }
    //             }

    //             Expr::Cases(cases_expr)
    //         }

    //         Expr::Sample(mut sample_expr) => {
    //             let mut has_errors = false;
    //             let mut has_default = false;
    //             let mut foldable = true;
    //             let mut running_prob = 0.0;
    //             for arm in &mut sample_expr.arms {
    //                 let owned_expr = std::mem::replace(&mut arm.expr, Expr::Error);
    //                 arm.expr = self.fold_expr(owned_expr, fold_sample);

    //                 match &mut arm.prob {
    //                     Prob::Expr(expr) => {
    //                         let owned_expr = std::mem::replace(expr, Expr::Error);
    //                         let mut folded_expr = self.fold_expr(owned_expr, fold_sample);
    //                         // Convert prob to real number 
    //                         if let Expr::Literal(Literal::Int(int)) = folded_expr {
    //                             println!("reached");
    //                             folded_expr = Expr::Literal(Literal::Real(int as f64));
    //                         }
    //                         *expr = folded_expr;

    //                         match expr {
    //                             Expr::Literal(Literal::Real(prob)) => {
    //                                 if *prob < 0.0 || *prob > 1.0 {
    //                                     self.diagnostics.error(CompilerError::ProbOutOfRange {
    //                                         total_prob: false,
    //                                         val: *prob,
    //                                         span: arm.arm_span.clone(),
    //                                     });

    //                                     has_errors = true;
    //                                 }

    //                                 running_prob += *prob;
    //                             }

    //                             _ => foldable = false,
    //                         }
    //                     }

    //                     Prob::Default => {
    //                         if has_default {
    //                             self.diagnostics.error(CompilerError::MultipleDefaultProb {
    //                                 arm_span: arm.arm_span.clone(),
    //                             });
    //                             has_errors = true;
    //                         }
    //                         else {
    //                             has_default = true;
    //                         }
    //                     }
    //                 }
    //             }
    //             if running_prob < 0.0 || running_prob > 1.0 {
    //                 self.diagnostics.error(CompilerError::ProbOutOfRange {
    //                     total_prob: true,
    //                     val: running_prob,
    //                     span: sample_expr.span,
    //                 });
    //                 has_errors = true;
    //             }

    //             if has_errors {
    //                 return Expr::Error;
    //             }
    //             if !foldable || !fold_sample {
    //                 return Expr::Sample(sample_expr);
    //             }
                
    //             let mut rng = rand::rng();
    //             let random_val: f64 = rng.random();
    //             running_prob = 0.0;
    //             for arm in sample_expr.arms {
    //                 if let Prob::Expr(Expr::Literal(Literal::Real(prob))) = arm.prob {
    //                     running_prob += prob;
                    
    //                     if random_val < running_prob {
    //                         return arm.expr;
    //                     }
    //                 }
    //                 else if let Prob::Default = arm.prob {
    //                     return arm.expr;
    //                 }
    //             }

    //             Expr::Error // Not reachable
    //         }

    //         _ => Expr::Error,
    //     }
    // }

    // fn eval_unary(&self, op: &UnaryOp, literal: &Literal) -> Option<Literal> {
    //     match (op, literal) {
    //         (UnaryOp::BitNot, Literal::Bool(x)) => Some(Literal::Bool(!x)),

    //         (UnaryOp::Neg, Literal::Int(x)) => Some(Literal::Int(-x)),

    //         (UnaryOp::Neg, Literal::Real(x)) => Some(Literal::Real(-x)),

    //         _ => None,
    //     }
    // }

    // fn eval_binary(&mut self, op: &BinaryOp, left: &Literal, right: &Literal, op_span: Span) -> Option<Literal> {
    //     match (op, left, right) {
    //         (BinaryOp::Or, Literal::Bool(a), Literal::Bool(b)) => Some(Literal::Bool(*a || *b)),
    //         (BinaryOp::And, Literal::Bool(a), Literal::Bool(b)) => Some(Literal::Bool(*a && *b)),

    //         (BinaryOp::Lt,  Literal::Int(a), Literal::Int(b)) => Some(Literal::Bool(a<b)),
    //         (BinaryOp::Gt,  Literal::Int(a), Literal::Int(b)) => Some(Literal::Bool(a>b)),
    //         (BinaryOp::Le,  Literal::Int(a), Literal::Int(b)) => Some(Literal::Bool(a<=b)),
    //         (BinaryOp::Ge,  Literal::Int(a), Literal::Int(b)) => Some(Literal::Bool(a>=b)),
    //         (BinaryOp::Add, Literal::Int(a), Literal::Int(b)) => Some(Literal::Int(a+b)),
    //         (BinaryOp::Sub, Literal::Int(a), Literal::Int(b)) => Some(Literal::Int(a-b)),
    //         (BinaryOp::Mul, Literal::Int(a), Literal::Int(b)) => Some(Literal::Int(a*b)),
    //         (BinaryOp::Div, Literal::Int(a), Literal::Int(b)) => 
    //             if *b != 0 {
    //                 Some(Literal::Int(a / b))
    //             } else {
    //                 self.diagnostics.error(CompilerError::DivideByZero {
    //                     op_span, 
    //                 });

    //                 None
    //             },
    //         (BinaryOp::Pow, Literal::Int(a), Literal::Int(b)) => 
    //             if *b >= 0 {
    //                 Some(Literal::Int(a.pow(*b as u32)))
    //             } else { // negative exponent not allowed for ints
    //                 self.diagnostics.error(CompilerError::NegExpOnInt {
    //                     op_span,
    //                 });

    //                 None
    //             },

    //         (BinaryOp::Lt,  Literal::Real(a), Literal::Real(b)) => Some(Literal::Bool(a<b)),
    //         (BinaryOp::Gt,  Literal::Real(a), Literal::Real(b)) => Some(Literal::Bool(a>b)),
    //         (BinaryOp::Le,  Literal::Real(a), Literal::Real(b)) => Some(Literal::Bool(a<=b)),
    //         (BinaryOp::Ge,  Literal::Real(a), Literal::Real(b)) => Some(Literal::Bool(a>=b)),
    //         (BinaryOp::Add, Literal::Real(a), Literal::Real(b)) => Some(Literal::Real(a+b)),
    //         (BinaryOp::Sub, Literal::Real(a), Literal::Real(b)) => Some(Literal::Real(a-b)),
    //         (BinaryOp::Mul, Literal::Real(a), Literal::Real(b)) => Some(Literal::Real(a*b)),
    //         (BinaryOp::Div, Literal::Real(a), Literal::Real(b)) => 
    //             if *b != 0.0 {
    //                 Some(Literal::Real(a / b))
    //             } else {
    //                 self.diagnostics.error(CompilerError::DivideByZero {
    //                     op_span, 
    //                 });

    //                 None
    //             },
    //         (BinaryOp::Pow, Literal::Real(a), Literal::Real(b)) => Some(Literal::Real(a.powf(*b))),

    //         _ => None,
    //     }
    // }

    // fn fold_pattern(&mut self, pattern: SimplePattern, fold_sample: bool) -> SimplePattern {
    //     match pattern {
    //         SimplePattern::Default => SimplePattern::Default,

    //         SimplePattern::Literal(literal) => SimplePattern::Literal(literal),

    //         SimplePattern::Ident(Ident::Symbol(id)) => match &self.symbols[id].kind {
    //             SymbolKind::Const(literal) => SimplePattern::Literal(literal.clone()),
    //             _ => SimplePattern::Ident(Ident::Symbol(id)),
    //         }

    //         SimplePattern::Tuple(mut patterns) => {
    //             for pattern in &mut patterns {
    //                 let owned_pattern = std::mem::replace(pattern, SimplePattern::Error);

    //                 *pattern = self.fold_pattern(owned_pattern, fold_sample);
    //             }

    //             SimplePattern::Tuple(patterns)
    //         }

    //         SimplePattern::Comparison(mut comp_pattern) => {
    //             comp_pattern.expr = Box::new(self.fold_expr(*comp_pattern.expr, fold_sample));

    //             SimplePattern::Comparison(comp_pattern)
    //         }

    //         SimplePattern::Error => SimplePattern::Error,

    //         _ => SimplePattern::Error, // Ident::Str unreachable
    //     }
    // }

    // // expr and pattern already folded. Returns none if there is an expr or pattern that prevents the outer cases expression from being folded
    // pub fn expr_matches_pattern(expr: &Expr, pattern: &SimplePattern) -> Option<bool> {
    //     match expr {
    //         Expr::Literal(expr_literal) => match pattern {
    //             SimplePattern::Default => Some(true),

    //             SimplePattern::Literal(pattern_literal) => Some(expr_literal == pattern_literal), 

    //             SimplePattern::Comparison(comp_pattern) => {
    //                 if let Expr::Literal(pattern_literal) = *comp_pattern.expr {
    //                     match comp_pattern.op {
    //                         CompOp::Lt => Some(*expr_literal < pattern_literal),

    //                         CompOp::Gt => Some(*expr_literal > pattern_literal),

    //                         CompOp::Le => Some(*expr_literal <= pattern_literal),

    //                         CompOp::Ge => Some(*expr_literal >= pattern_literal),
    //                     } 
    //                 }
    //                 else {
    //                     None
    //                 }
    //             }
                

    //             _ => None,
    //         }

    //         Expr::Tuple(expr_tuple) => match pattern {
    //             // default is true on tuple if the tuple is matchable (all literals)
    //             SimplePattern::Default => {
    //                 for expr in expr_tuple {
    //                     match expr {
    //                         Expr::Literal(_) => {}
    //                         _ => return None,
    //                     }
    //                 }

    //                 Some(true)
    //             }

    //             SimplePattern::Tuple(pattern_tuple) => {
    //                 for (expr, pattern) in expr_tuple.iter().zip(pattern_tuple.iter()) {
    //                     match Self::expr_matches_pattern(expr, pattern) {
    //                         Some(true) => {}
    //                         Some(false) => return Some(false),
    //                         None => return None, 
    //                     }
    //                 }
    //                 Some(true)
    //             }

    //             _ => None,
    //         }

    //         _ => None // only literals can match
    //     }
    // }

    // pub fn pattern_matches_literal(pattern: &SimplePattern, literal: &Literal) -> bool {
    //     match pattern {
    //         SimplePattern::Literal(pattern_literal) => 
    //             literal == pattern_literal,

    //         SimplePattern::Ident(_) => false,

    //         SimplePattern::Tuple(_) => false,

    //         SimplePattern::Comparison(_) => false,

    //         SimplePattern::Default => true,

    //         SimplePattern::Error => false,
    //     }
    // }
}