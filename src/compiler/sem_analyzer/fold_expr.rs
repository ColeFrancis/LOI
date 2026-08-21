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

//! # fold_expr
//!
//! Handles folding of compile-time constants in expressions. 
//! Verifies there are no duplicate patterns (including defaults) in cases arms
//! Verifies there are no more than one default arms in samples and that literals in probabilities do not sum to greater than 1
//!
//! ## Invariants:
//!
//! - If an expression is folded into a literal, its type must match the original expr_type
//!
//! Author: Cole Francis

use rand::Rng;

use super::SemAnalyzer;
use super::types::Type;
use crate::compiler::{
    ast::*,
    symbol::SymbolKind,
    diagnostics::{Span, CompilerError},
};

impl <'a> SemAnalyzer<'a> {
    pub(super) fn fold_expr (&mut self, expr: Expr, fold_sample: bool) -> Expr {
        match expr {
            Expr::Literal(literal) => Expr::Literal(literal),

            Expr::Ident(Ident::Symbol(id)) => match &self.symbols[id].kind {
                SymbolKind::Const(literal) => Expr::Literal(literal.clone()),
                _ => Expr::Ident(Ident::Symbol(id)),
            }

            Expr::Unary(mut unary) => {
                unary.expr = Box::new(self.fold_expr(*unary.expr, fold_sample));

                if let Expr::Literal(literal) = &*unary.expr {
                    if let Some(result) = self.eval_unary(&unary.op, literal) {
                        return Expr::Literal(result);
                    }
                }

                Expr::Unary(unary)
            }

            Expr::Binary(mut binary) => {
                let mut expr_left = self.fold_expr(*binary.left, fold_sample);
                let mut expr_right = self.fold_expr(*binary.right, fold_sample);

                // if you have int + real convert to real + real to simplify eval_binary
                if binary.expr_type == Type::Real {
                    if let Expr::Literal(Literal::Int(val)) = expr_left {
                        expr_left = Expr::Literal(Literal::Real(val as f64));
                    }

                    if let Expr::Literal(Literal::Int(val)) = expr_right {
                        expr_right = Expr::Literal(Literal::Real(val as f64));
                    }
                }

                binary.left = Box::new(expr_left);
                binary.right = Box::new(expr_right);

                if let (Expr::Literal(left), Expr::Literal(right)) = (&*binary.left, &*binary.right) {
                    if let Some(result) = self.eval_binary(&binary.op, left, right,binary.op_span) {
                        return Expr::Literal(result);
                    }
                }

                Expr::Binary(binary)
            }

            Expr::Tuple(mut exprs) => {
                for expr in &mut exprs {
                    let owned_expr = std::mem::replace(expr, Expr::Error);

                    *expr = self.fold_expr(owned_expr, fold_sample);
                }

                Expr::Tuple(exprs)
            }

            Expr::Block(mut block_expr) => {
                let owned_statements = std::mem::take(&mut block_expr.statements);

                for statement in owned_statements {
                    if let Statement::Let(let_statement) = statement {
                        if let Some(folded_let_statement) = self.fold_let(let_statement, fold_sample) {
                            block_expr.statements.push(Statement::Let(folded_let_statement));
                        }
                    }
                }

                match self.fold_expr(*block_expr.expr, fold_sample) {
                    Expr::Literal(literal) => Expr::Literal(literal),

                    other => {
                        block_expr.expr = Box::new(other);

                        Expr::Block(block_expr)
                    }
                }
            }

            Expr::Cases(mut cases_expr) => {
                // Check for duplicate arms
                let mut has_errors = false;
                let mut seen_patterns: Vec<(&SimplePattern, Span)> = Vec::new();
                for arm in &mut cases_expr.arms {
                    for pattern in &mut arm.pattern {
                        let owned_pattern = std::mem::replace(pattern, SimplePattern::Error);
                        *pattern = self.fold_pattern(owned_pattern, fold_sample);

                        if let Some((_, old_span)) = seen_patterns.iter().find(|(p, _)| *p == pattern) {
                            self.diagnostics.error(CompilerError::DuplicatePattern {
                                old_arm_span: old_span.clone(),
                                arm_span: arm.arm_span,
                            });

                            has_errors = true;
                        }

                        seen_patterns.push((pattern, arm.arm_span));
                    }
                }
                if has_errors {
                    return Expr::Error;
                }

                cases_expr.scrutinee = Box::new(self.fold_expr(*cases_expr.scrutinee, fold_sample));

                let mut foldable = true;
                for arm in &mut cases_expr.arms {
                    let owned_expr = std::mem::replace(&mut arm.expr, Expr::Error);
                    arm.expr = self.fold_expr(owned_expr, fold_sample);

                    let matches = arm.pattern.iter_mut().any(|pattern| {
                        match Self::expr_matches_pattern(&cases_expr.scrutinee, pattern){
                            Some(res) => res,
                            None => {
                                foldable = false;
                                false
                            }
                        }
                    });

                    if matches && foldable {
                        return std::mem::replace(&mut arm.expr, Expr::Error);
                    }
                }

                Expr::Cases(cases_expr)
            }

            Expr::Sample(mut sample_expr) => {
                let mut has_errors = false;
                let mut has_default = false;
                let mut foldable = true;
                let mut running_prob = 0.0;
                for arm in &mut sample_expr.arms {
                    let owned_expr = std::mem::replace(&mut arm.expr, Expr::Error);
                    arm.expr = self.fold_expr(owned_expr, fold_sample);

                    match &mut arm.prob {
                        Prob::Expr(expr) => {
                            let owned_expr = std::mem::replace(expr, Expr::Error);
                            let mut folded_expr = self.fold_expr(owned_expr, fold_sample);
                            // Convert prob to real number 
                            if let Expr::Literal(Literal::Int(int)) = folded_expr {
                                println!("reached");
                                folded_expr = Expr::Literal(Literal::Real(int as f64));
                            }
                            *expr = folded_expr;

                            match expr {
                                Expr::Literal(Literal::Real(prob)) => {
                                    if *prob < 0.0 || *prob > 1.0 {
                                        self.diagnostics.error(CompilerError::ProbOutOfRange {
                                            total_prob: false,
                                            val: *prob,
                                            span: arm.arm_span.clone(),
                                        });

                                        has_errors = true;
                                    }

                                    running_prob += *prob;
                                }

                                _ => foldable = false,
                            }
                        }

                        Prob::Default => {
                            if has_default {
                                self.diagnostics.error(CompilerError::MultipleDefaultProb {
                                    arm_span: arm.arm_span.clone(),
                                });
                                has_errors = true;
                            }
                            else {
                                has_default = true;
                            }
                        }
                    }
                }
                if running_prob < 0.0 || running_prob > 1.0 {
                    self.diagnostics.error(CompilerError::ProbOutOfRange {
                        total_prob: true,
                        val: running_prob,
                        span: sample_expr.span,
                    });
                    has_errors = true;
                }

                if has_errors {
                    return Expr::Error;
                }
                if !foldable || !fold_sample {
                    return Expr::Sample(sample_expr);
                }
                
                let mut rng = rand::rng();
                let random_val: f64 = rng.random();
                running_prob = 0.0;
                for arm in sample_expr.arms {
                    if let Prob::Expr(Expr::Literal(Literal::Real(prob))) = arm.prob {
                        running_prob += prob;
                    
                        if random_val < running_prob {
                            return arm.expr;
                        }
                    }
                    else if let Prob::Default = arm.prob {
                        return arm.expr;
                    }
                }

                Expr::Error // Not reachable
            }

            _ => Expr::Error,
        }
    }

    fn eval_unary(&self, op: &UnaryOp, literal: &Literal) -> Option<Literal> {
        match (op, literal) {
            (UnaryOp::BitNot, Literal::Bool(x)) => Some(Literal::Bool(!x)),

            (UnaryOp::Neg, Literal::Int(x)) => Some(Literal::Int(-x)),

            (UnaryOp::Neg, Literal::Real(x)) => Some(Literal::Real(-x)),

            _ => None,
        }
    }

    fn eval_binary(&mut self, op: &BinaryOp, left: &Literal, right: &Literal, op_span: Span) -> Option<Literal> {
        match (op, left, right) {
            (BinaryOp::Or, Literal::Bool(a), Literal::Bool(b)) => Some(Literal::Bool(*a || *b)),
            (BinaryOp::And, Literal::Bool(a), Literal::Bool(b)) => Some(Literal::Bool(*a && *b)),

            (BinaryOp::Lt,  Literal::Int(a), Literal::Int(b)) => Some(Literal::Bool(a<b)),
            (BinaryOp::Gt,  Literal::Int(a), Literal::Int(b)) => Some(Literal::Bool(a>b)),
            (BinaryOp::Le,  Literal::Int(a), Literal::Int(b)) => Some(Literal::Bool(a<=b)),
            (BinaryOp::Ge,  Literal::Int(a), Literal::Int(b)) => Some(Literal::Bool(a>=b)),
            (BinaryOp::Add, Literal::Int(a), Literal::Int(b)) => Some(Literal::Int(a+b)),
            (BinaryOp::Sub, Literal::Int(a), Literal::Int(b)) => Some(Literal::Int(a-b)),
            (BinaryOp::Mul, Literal::Int(a), Literal::Int(b)) => Some(Literal::Int(a*b)),
            (BinaryOp::Div, Literal::Int(a), Literal::Int(b)) => 
                if *b != 0 {
                    Some(Literal::Int(a / b))
                } else {
                    self.diagnostics.error(CompilerError::DivideByZero {
                        op_span, 
                    });

                    None
                },
            (BinaryOp::Pow, Literal::Int(a), Literal::Int(b)) => 
                if *b >= 0 {
                    Some(Literal::Int(a.pow(*b as u32)))
                } else { // negative exponent not allowed for ints
                    self.diagnostics.error(CompilerError::NegExpOnInt {
                        op_span,
                    });

                    None
                },

            (BinaryOp::Lt,  Literal::Real(a), Literal::Real(b)) => Some(Literal::Bool(a<b)),
            (BinaryOp::Gt,  Literal::Real(a), Literal::Real(b)) => Some(Literal::Bool(a>b)),
            (BinaryOp::Le,  Literal::Real(a), Literal::Real(b)) => Some(Literal::Bool(a<=b)),
            (BinaryOp::Ge,  Literal::Real(a), Literal::Real(b)) => Some(Literal::Bool(a>=b)),
            (BinaryOp::Add, Literal::Real(a), Literal::Real(b)) => Some(Literal::Real(a+b)),
            (BinaryOp::Sub, Literal::Real(a), Literal::Real(b)) => Some(Literal::Real(a-b)),
            (BinaryOp::Mul, Literal::Real(a), Literal::Real(b)) => Some(Literal::Real(a*b)),
            (BinaryOp::Div, Literal::Real(a), Literal::Real(b)) => 
                if *b != 0.0 {
                    Some(Literal::Real(a / b))
                } else {
                    self.diagnostics.error(CompilerError::DivideByZero {
                        op_span, 
                    });

                    None
                },
            (BinaryOp::Pow, Literal::Real(a), Literal::Real(b)) => Some(Literal::Real(a.powf(*b))),

            _ => None,
        }
    }

    fn fold_pattern(&mut self, pattern: SimplePattern, fold_sample: bool) -> SimplePattern {
        match pattern {
            SimplePattern::Default => SimplePattern::Default,

            SimplePattern::Literal(literal) => SimplePattern::Literal(literal),

            SimplePattern::Ident(Ident::Symbol(id)) => match &self.symbols[id].kind {
                SymbolKind::Const(literal) => SimplePattern::Literal(literal.clone()),
                _ => SimplePattern::Ident(Ident::Symbol(id)),
            }

            SimplePattern::Tuple(mut patterns) => {
                for pattern in &mut patterns {
                    let owned_pattern = std::mem::replace(pattern, SimplePattern::Error);

                    *pattern = self.fold_pattern(owned_pattern, fold_sample);
                }

                SimplePattern::Tuple(patterns)
            }

            SimplePattern::Comparison(mut comp_pattern) => {
                comp_pattern.expr = Box::new(self.fold_expr(*comp_pattern.expr, fold_sample));

                SimplePattern::Comparison(comp_pattern)
            }

            SimplePattern::Error => SimplePattern::Error,

            _ => SimplePattern::Error, // Ident::Str unreachable
        }
    }

    // expr and pattern already folded. Returns none if there is an expr or pattern that prevents the outer cases expression from being folded
    fn expr_matches_pattern(expr: &Expr, pattern: &SimplePattern) -> Option<bool> {
        match expr {
            Expr::Literal(expr_literal) => match pattern {
                SimplePattern::Default => Some(true),

                SimplePattern::Literal(pattern_literal) => Some(expr_literal == pattern_literal), 

                SimplePattern::Comparison(comp_pattern) => {
                    if let Expr::Literal(pattern_literal) = *comp_pattern.expr {
                        match comp_pattern.op {
                            CompOp::Lt => Some(*expr_literal < pattern_literal),

                            CompOp::Gt => Some(*expr_literal > pattern_literal),

                            CompOp::Le => Some(*expr_literal <= pattern_literal),

                            CompOp::Ge => Some(*expr_literal >= pattern_literal),
                        } 
                    }
                    else {
                        None
                    }
                }
                

                _ => None,
            }

            Expr::Tuple(expr_tuple) => match pattern {
                // default is true on tuple if the tuple is matchable (all literals)
                SimplePattern::Default => {
                    for expr in expr_tuple {
                        match expr {
                            Expr::Literal(_) => {}
                            _ => return None,
                        }
                    }

                    Some(true)
                }

                SimplePattern::Tuple(pattern_tuple) => {
                    for (expr, pattern) in expr_tuple.iter().zip(pattern_tuple.iter()) {
                        match Self::expr_matches_pattern(expr, pattern) {
                            Some(true) => {}
                            Some(false) => return Some(false),
                            None => return None, 
                        }
                    }
                    Some(true)
                }

                _ => None,
            }

            _ => None // only literals can match
        }
    }

    fn pattern_matches_literal(pattern: &SimplePattern, literal: &Literal) -> bool {
        match pattern {
            SimplePattern::Literal(pattern_literal) => 
                literal == pattern_literal,

            SimplePattern::Ident(_) => false,

            SimplePattern::Tuple(_) => false,

            SimplePattern::Comparison(_) => false,

            SimplePattern::Default => true,

            SimplePattern::Error => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::compiler::sem_analyzer::scope::Scope;
    use crate::compiler::symbol::Symbol;
    use crate::compiler::diagnostics::Diagnostics;

    #[test]
    fn test_const () {
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Const(Literal::Int(1)),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("a".to_string(), 0),
                    ])
                },
            ],

            diagnostics: &mut Diagnostics::new(),
        };

        let result = sem_analyzer.fold_expr(Expr::Ident(Ident::Symbol(0)), true);

        assert_eq!(result, Expr::Literal(Literal::Int(1)));
    }

    #[test]
    fn test_unary_1() {
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer::new(
            Program {items: Vec::new()},
            &mut diagnostics
        );

        let result = sem_analyzer.fold_expr(Expr::Unary(UnaryExpr {
            expr: Box::new(Expr::Literal(Literal::Bool(false))),
            op: UnaryOp::BitNot,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Bool,
        }), true);

        assert_eq!(result, Expr::Literal(Literal::Bool(true)));
    }

    #[test]
    fn test_unary_2() {
        // -a
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Const(Literal::Real(1.0)),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("a".to_string(), 0),
                    ])
                },
            ],

            diagnostics: &mut Diagnostics::new(),
        };

        let result = sem_analyzer.fold_expr(Expr::Unary(UnaryExpr {
            expr: Box::new(Expr::Ident(Ident::Symbol(0))),
            op: UnaryOp::Neg,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Real,
        }), true);

        assert_eq!(result, Expr::Literal(Literal::Real(-1.0)));
    }

    #[test]
    fn test_binary_1() {
        // 1.0 + 2^3
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer::new(
            Program {items: Vec::new()},
            &mut diagnostics
        );

        let result = sem_analyzer.fold_expr(Expr::Binary(BinaryExpr{
            left: Box::new(Expr::Literal(Literal::Real(1.0))),
            right: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Literal(Literal::Int(2))),
                right: Box::new(Expr::Literal(Literal::Int(3))),
                op: BinaryOp::Pow,
                op_span: Span{line: 0, col: 1},
                expr_type: Type::Int,
            })),
            op: BinaryOp::Add,
            op_span: Span {line: 0, col: 0},
            expr_type: Type::Real,
        }), true);

        assert_eq!(result, Expr::Literal(Literal::Real(9.0)));
    }

    #[test]
    fn test_binary_2() {
        // 1.0 + 2^(-3) // Neg exp on integer error
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer::new(
            Program {items: Vec::new()},
            &mut diagnostics
        );

        let _result = sem_analyzer.fold_expr(Expr::Binary(BinaryExpr{
            left: Box::new(Expr::Literal(Literal::Real(1.0))),
            right: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Literal(Literal::Int(2))),
                right: Box::new(Expr::Unary(UnaryExpr {
                    expr: Box::new(Expr::Literal(Literal::Int(3))),
                    op: UnaryOp::Neg,
                    op_span: Span{line: 0, col:2},
                    expr_type: Type::Int,
                })),
                op: BinaryOp::Pow,
                op_span: Span{line: 0, col: 1},
                expr_type: Type::Int,
            })),
            op: BinaryOp::Add,
            op_span: Span {line: 0, col: 0},
            expr_type: Type::Real,
        }), true);

        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn test_binary_3() {
        // 1.0 + 2.0^(-3)
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer::new(
            Program {items: Vec::new()},
            &mut diagnostics
        );

        let result = sem_analyzer.fold_expr(Expr::Binary(BinaryExpr{
            left: Box::new(Expr::Literal(Literal::Real(1.0))),
            right: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Literal(Literal::Real(2.0))),
                right: Box::new(Expr::Unary(UnaryExpr {
                    expr: Box::new(Expr::Literal(Literal::Int(3))),
                    op: UnaryOp::Neg,
                    op_span: Span{line: 0, col:2},
                    expr_type: Type::Int,
                })),
                op: BinaryOp::Pow,
                op_span: Span{line: 0, col: 1},
                expr_type: Type::Real,
            })),
            op: BinaryOp::Add,
            op_span: Span {line: 0, col: 0},
            expr_type: Type::Real,
        }), true);

        assert_eq!(result, Expr::Literal(Literal::Real(1.125)));
    }

    #[test]
    fn test_binary_4() {
        // n + (a + 1) where n is an unknown variable and a is a folded constant
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "n".to_string(),
                    kind: SymbolKind::Variable(Type::Int),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Const(Literal::Int(1)),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("n".to_string(), 0),
                        ("a".to_string(), 1),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.fold_expr(Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Ident(Ident::Symbol(0))),
            right: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Ident(Ident::Symbol(1))),
                right: Box::new(Expr::Literal(Literal::Int(1))),
                op: BinaryOp::Add,
                op_span: Span{line: 0, col: 0},
                expr_type: Type::Int,
            })),
            op: BinaryOp::Add,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Int,
        }), true);

        assert_eq!(result, Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Ident(Ident::Symbol(0))),
            right: Box::new(Expr::Literal(Literal::Int(2))),
            op: BinaryOp::Add,
            op_span: Span{line: 0, col: 0},
            expr_type: Type::Int,
        }));
    }

    #[test]
    fn test_binary_5() {
        // 1 < 2
        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer::new(
            Program {items: Vec::new()},
            &mut diagnostics
        );

        let result = sem_analyzer.fold_expr(Expr::Binary(BinaryExpr{
            left: Box::new(Expr::Literal(Literal::Int(1))),
            right: Box::new(Expr::Literal(Literal::Int(2))),
            op: BinaryOp::Lt,
            op_span: Span {line: 0, col: 0},
            expr_type: Type::Int,
        }), true);

        assert_eq!(result, Expr::Literal(Literal::Bool(true)));
    }

    #[test]
    fn test_block_1() {
        // {
        //     let n = 2;
        //     n-1
        // }

        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "n".to_string(),
                    kind: SymbolKind::Variable(Type::Int),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("n".to_string(), 0),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.fold_expr(Expr::Block(BlockExpr {
            statements: vec![
                Statement::Let(LetStatement {
                    name: Ident::Symbol(0),
                    expr: Expr::Literal(Literal::Int(2)),
                }),
            ],
            expr: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Ident(Ident::Symbol(0))),
                right: Box::new(Expr::Literal(Literal::Int(1))),
                op: BinaryOp::Sub,
                op_span: Span{line: 0, col: 0},
                expr_type: Type::Int,
            })),
            expr_type: Type::Int
        }), true);

        assert_eq!(result, Expr::Literal(Literal::Int(1)));
    }

    #[test]
    fn test_block_2() {
        // {
        //     let n = 1 + bool; // wrong types result in Expr::Error
        //     n-1
        // }

        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "n".to_string(),
                    kind: SymbolKind::Variable(Type::Int),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("n".to_string(), 0),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.fold_expr(Expr::Block(BlockExpr {
            statements: vec![
                Statement::Let(LetStatement {
                    name: Ident::Symbol(0),
                    expr: Expr::Error,
                }),
            ],
            expr: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Ident(Ident::Symbol(0))),
                right: Box::new(Expr::Literal(Literal::Int(1))),
                op: BinaryOp::Sub,
                op_span: Span{line: 0, col: 0},
                expr_type: Type::Int,
            })),
            expr_type: Type::Int
        }), true);

        assert_eq!(result, Expr::Block(BlockExpr {
            statements: vec![
                Statement::Let(LetStatement {
                    name: Ident::Symbol(0),
                    expr: Expr::Error,
                }),
            ],
            expr: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Ident(Ident::Symbol(0))),
                right: Box::new(Expr::Literal(Literal::Int(1))),
                op: BinaryOp::Sub,
                op_span: Span{line: 0, col: 0},
                expr_type: Type::Int,
            })),
            expr_type: Type::Int
        }));
    }

    #[test]
    fn test_block_3() {
        // {
        //     let n =  bool; 
        //     n-1 // Expr error
        // }

        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "n".to_string(),
                    kind: SymbolKind::Variable(Type::Bool),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("n".to_string(), 0),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.fold_expr(Expr::Block(BlockExpr {
            statements: vec![
                Statement::Let(LetStatement {
                    name: Ident::Symbol(0),
                    expr: Expr::Literal(Literal::Bool(true)),
                }),
            ],
            expr: Box::new(Expr::Error),
            expr_type: Type::Int
        }), true);

        assert_eq!(result, Expr::Block(BlockExpr {
            statements: vec![],
            expr: Box::new(Expr::Error),
            expr_type: Type::Int
        }));
        assert_eq!(sem_analyzer.symbols, vec![
            Symbol {
                name: "n".to_string(),
                kind: SymbolKind::Const(Literal::Bool(true)),
                span: Span{line: 0, col: 0},
            },
        ]);
    }

    #[test]
    fn test_cases_1() {
        // cases (a, b) {    // a=1, b=1 constants
        //     (c, 1) : 1+1, // c=1 constant
        //     _ : 2*3,
        // }

        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Const(Literal::Int(1)),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Const(Literal::Int(1)),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "c".to_string(),
                    kind: SymbolKind::Const(Literal::Int(1)),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("a".to_string(), 0),
                        ("b".to_string(), 1),
                        ("c".to_string(), 2),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.fold_expr(Expr::Cases(CasesExpr {
            scrutinee: Box::new(Expr::Tuple(vec![
                Expr::Ident(Ident::Symbol(0)),
                Expr::Ident(Ident::Symbol(1)),
            ])),
            arms: vec![
                CasesArm {
                    pattern: vec![
                        SimplePattern::Tuple(vec![
                            SimplePattern::Ident(Ident::Symbol(2)),
                            SimplePattern::Literal(Literal::Int(1)),
                        ]),
                    ],
                    expr: Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::Literal(Literal::Int(1))),
                        right: Box::new(Expr::Literal(Literal::Int(1))),
                        op: BinaryOp::Add,
                        op_span: Span{line: 0, col: 0},
                        expr_type: Type::Int,
                    }),
                    arm_span: Span{line: 0, col: 0},
                },
                CasesArm {
                    pattern: vec![
                        SimplePattern::Default,
                    ],
                    expr: Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::Literal(Literal::Int(2))),
                        right: Box::new(Expr::Literal(Literal::Int(3))),
                        op: BinaryOp::Mul,
                        op_span: Span{line: 0, col: 0},
                        expr_type: Type::Int,
                    }),
                    arm_span: Span{line: 0, col: 0},
                },
            ],
            expr_type: Type::Int,
            span: Span{line: 0, col: 0},
        }), true);

        assert_eq!(result, Expr::Literal(Literal::Int(2)));
    }

    #[test]
    fn test_cases_2() {
        // cases (a, b) {    // a=2, b=1 constants
        //     (c, 1) : 1+1, // c=1 constant
        //     _ : 2*3,
        // }

        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Const(Literal::Int(2)),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Const(Literal::Int(1)),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "c".to_string(),
                    kind: SymbolKind::Const(Literal::Int(1)),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("a".to_string(), 0),
                        ("b".to_string(), 1),
                        ("c".to_string(), 2),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.fold_expr(Expr::Cases(CasesExpr {
            scrutinee: Box::new(Expr::Tuple(vec![
                Expr::Ident(Ident::Symbol(0)),
                Expr::Ident(Ident::Symbol(1)),
            ])),
            arms: vec![
                CasesArm {
                    pattern: vec![
                        SimplePattern::Tuple(vec![
                            SimplePattern::Ident(Ident::Symbol(2)),
                            SimplePattern::Literal(Literal::Int(1)),
                        ]),
                    ],
                    expr: Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::Literal(Literal::Int(1))),
                        right: Box::new(Expr::Literal(Literal::Int(1))),
                        op: BinaryOp::Add,
                        op_span: Span{line: 0, col: 0},
                        expr_type: Type::Int,
                    }),
                    arm_span: Span{line: 0, col: 0},
                },
                CasesArm {
                    pattern: vec![
                        SimplePattern::Default,
                    ],
                    expr: Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::Literal(Literal::Int(2))),
                        right: Box::new(Expr::Literal(Literal::Int(3))),
                        op: BinaryOp::Mul,
                        op_span: Span{line: 0, col: 0},
                        expr_type: Type::Int,
                    }),
                    arm_span: Span{line: 0, col: 0},
                },
            ],
            expr_type: Type::Int,
            span: Span{line: 0, col: 0},
        }), true);

        assert_eq!(result, Expr::Literal(Literal::Int(6)));
    }

    #[test]
    fn test_cases_3() {
        // cases (a, b+1) {    // b=1 constant, a is unknown
        //     (c, 1) : 1+1, // c=1 constant
        //     _ : 2*3,
        // }

        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Variable(Type::Int),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Const(Literal::Int(1)),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "c".to_string(),
                    kind: SymbolKind::Const(Literal::Int(1)),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("a".to_string(), 0),
                        ("b".to_string(), 1),
                        ("c".to_string(), 2),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.fold_expr(Expr::Cases(CasesExpr {
            scrutinee: Box::new(Expr::Tuple(vec![
                Expr::Ident(Ident::Symbol(0)),
                Expr::Binary(BinaryExpr {
                    left: Box::new(Expr::Ident(Ident::Symbol(1))),
                    right: Box::new(Expr::Literal(Literal::Int(1))),
                    op: BinaryOp::Add,
                    op_span: Span{line: 0, col: 0},
                    expr_type: Type::Int,
                }),
            ])),
            arms: vec![
                CasesArm {
                    pattern: vec![
                        SimplePattern::Tuple(vec![
                            SimplePattern::Ident(Ident::Symbol(2)),
                            SimplePattern::Literal(Literal::Int(1)),
                        ]),
                    ],
                    expr: Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::Literal(Literal::Int(1))),
                        right: Box::new(Expr::Literal(Literal::Int(1))),
                        op: BinaryOp::Add,
                        op_span: Span{line: 0, col: 0},
                        expr_type: Type::Int,
                    }),
                    arm_span: Span{line: 0, col: 0},
                },
                CasesArm {
                    pattern: vec![
                        SimplePattern::Default,
                    ],
                    expr: Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::Literal(Literal::Int(2))),
                        right: Box::new(Expr::Literal(Literal::Int(3))),
                        op: BinaryOp::Mul,
                        op_span: Span{line: 0, col: 0},
                        expr_type: Type::Int,
                    }),
                    arm_span: Span{line: 0, col: 0},
                },
            ],
            expr_type: Type::Int,
            span: Span{line: 0, col: 0},
        }), true);

        assert_eq!(result, Expr::Cases(CasesExpr {
            scrutinee: Box::new(Expr::Tuple(vec![
                Expr::Ident(Ident::Symbol(0)),
                Expr::Literal(Literal::Int(2)),
            ])),
            arms: vec![
                CasesArm {
                    pattern: vec![
                        SimplePattern::Tuple(vec![
                            SimplePattern::Literal(Literal::Int(1)),
                            SimplePattern::Literal(Literal::Int(1)),
                        ]),
                    ],
                    expr: Expr::Literal(Literal::Int(2)),
                    arm_span: Span{line: 0, col: 0},
                },
                CasesArm {
                    pattern: vec![
                        SimplePattern::Default,
                    ],
                    expr: Expr::Literal(Literal::Int(6)),
                    arm_span: Span{line: 0, col: 0},
                },
            ],
            expr_type: Type::Int,
            span: Span{line: 0, col: 0},
        }));
    }

    #[test]
    fn test_cases_4() {
        // cases (a, b+1) {    // a=1, b=1 constants
        //     (c, 1) : 1+1, // c is unknown
        //     _ : 2*3,
        // }

        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Const(Literal::Int(1)),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Const(Literal::Int(1)),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "c".to_string(),
                    kind: SymbolKind::Variable(Type::Int),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("a".to_string(), 0),
                        ("b".to_string(), 1),
                        ("c".to_string(), 2),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.fold_expr(Expr::Cases(CasesExpr {
            scrutinee: Box::new(Expr::Tuple(vec![
                Expr::Ident(Ident::Symbol(0)),
                Expr::Binary(BinaryExpr {
                    left: Box::new(Expr::Ident(Ident::Symbol(1))),
                    right: Box::new(Expr::Literal(Literal::Int(1))),
                    op: BinaryOp::Add,
                    op_span: Span{line: 0, col: 0},
                    expr_type: Type::Int,
                }),
            ])),
            arms: vec![
                CasesArm {
                    pattern: vec![
                        SimplePattern::Tuple(vec![
                            SimplePattern::Ident(Ident::Symbol(2)),
                            SimplePattern::Literal(Literal::Int(1)),
                        ]),
                    ],
                    expr: Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::Literal(Literal::Int(1))),
                        right: Box::new(Expr::Literal(Literal::Int(1))),
                        op: BinaryOp::Add,
                        op_span: Span{line: 0, col: 0},
                        expr_type: Type::Int,
                    }),
                    arm_span: Span{line: 0, col: 0},
                },
                CasesArm {
                    pattern: vec![
                        SimplePattern::Default,
                    ],
                    expr: Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::Literal(Literal::Int(2))),
                        right: Box::new(Expr::Literal(Literal::Int(3))),
                        op: BinaryOp::Mul,
                        op_span: Span{line: 0, col: 0},
                        expr_type: Type::Int,
                    }),
                    arm_span: Span{line: 0, col: 0},
                },
            ],
            expr_type: Type::Int,
            span: Span{line: 0, col: 0},
        }), true);

        assert_eq!(result, Expr::Cases(CasesExpr {
            scrutinee: Box::new(Expr::Tuple(vec![
                Expr::Literal(Literal::Int(1)),
                Expr::Literal(Literal::Int(2)),
            ])),
            arms: vec![
                CasesArm {
                    pattern: vec![
                        SimplePattern::Tuple(vec![
                            SimplePattern::Ident(Ident::Symbol(2)),
                            SimplePattern::Literal(Literal::Int(1)),
                        ]),
                    ],
                    expr: Expr::Literal(Literal::Int(2)),
                    arm_span: Span{line: 0, col: 0},
                },
                CasesArm {
                    pattern: vec![
                        SimplePattern::Default,
                    ],
                    expr: Expr::Literal(Literal::Int(6)),
                    arm_span: Span{line: 0, col: 0},
                },
            ],
            expr_type: Type::Int,
            span: Span{line: 0, col: 0},
        }));
    }

    #[test]
    fn test_cases_5() {
        // cases (a, b) {    // a=1, b=1 constants
        //     (c, 2) | (1, 1) : 1+1, // c=1 constant
        //     _ : 2*3,
        // }

        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Const(Literal::Int(1)),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Const(Literal::Int(1)),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "c".to_string(),
                    kind: SymbolKind::Const(Literal::Int(1)),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("a".to_string(), 0),
                        ("b".to_string(), 1),
                        ("c".to_string(), 2),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.fold_expr(Expr::Cases(CasesExpr {
            scrutinee: Box::new(Expr::Tuple(vec![
                Expr::Ident(Ident::Symbol(0)),
                Expr::Ident(Ident::Symbol(1)),
            ])),
            arms: vec![
                CasesArm {
                    pattern: vec![
                        SimplePattern::Tuple(vec![
                            SimplePattern::Ident(Ident::Symbol(2)),
                            SimplePattern::Literal(Literal::Int(2)),
                        ]),
                        SimplePattern::Tuple(vec![
                            SimplePattern::Literal(Literal::Int(1)),
                            SimplePattern::Literal(Literal::Int(1)),
                        ]),
                    ],
                    expr: Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::Literal(Literal::Int(1))),
                        right: Box::new(Expr::Literal(Literal::Int(1))),
                        op: BinaryOp::Add,
                        op_span: Span{line: 0, col: 0},
                        expr_type: Type::Int,
                    }),
                    arm_span: Span{line: 0, col: 0},
                },
                CasesArm {
                    pattern: vec![
                        SimplePattern::Default,
                    ],
                    expr: Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::Literal(Literal::Int(2))),
                        right: Box::new(Expr::Literal(Literal::Int(3))),
                        op: BinaryOp::Mul,
                        op_span: Span{line: 0, col: 0},
                        expr_type: Type::Int,
                    }),
                    arm_span: Span{line: 0, col: 0},
                },
            ],
            expr_type: Type::Int,
            span: Span{line: 0, col: 0},
        }), true);

        assert_eq!(result, Expr::Literal(Literal::Int(2)));
    }

    #[test]
    fn test_cases_6() {
        // cases (a, b) {   
        //     (c, 1) | (1, 1) : 1+1, // duplicate patterns
        //     _ : 2*3,
        // }

        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Const(Literal::Int(1)),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "a".to_string(),
                    kind: SymbolKind::Const(Literal::Int(1)),
                    span: Span{line: 0, col: 0},
                },
                Symbol {
                    name: "c".to_string(),
                    kind: SymbolKind::Const(Literal::Int(1)),
                    span: Span{line: 0, col: 0},
                },
            ],
            scopes: vec![
                Scope {
                    symbols: HashMap::from([
                        ("a".to_string(), 0),
                        ("b".to_string(), 1),
                        ("c".to_string(), 2),
                    ])
                },
            ],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.fold_expr(Expr::Cases(CasesExpr {
            scrutinee: Box::new(Expr::Tuple(vec![
                Expr::Ident(Ident::Symbol(0)),
                Expr::Ident(Ident::Symbol(1)),
            ])),
            arms: vec![
                CasesArm {
                    pattern: vec![
                        SimplePattern::Tuple(vec![
                            SimplePattern::Ident(Ident::Symbol(2)),
                            SimplePattern::Literal(Literal::Int(1)),
                        ]),
                        SimplePattern::Tuple(vec![
                            SimplePattern::Literal(Literal::Int(1)),
                            SimplePattern::Literal(Literal::Int(1)),
                        ]),
                    ],
                    expr: Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::Literal(Literal::Int(1))),
                        right: Box::new(Expr::Literal(Literal::Int(1))),
                        op: BinaryOp::Add,
                        op_span: Span{line: 0, col: 0},
                        expr_type: Type::Int,
                    }),
                    arm_span: Span{line: 0, col: 0},
                },
                CasesArm {
                    pattern: vec![
                        SimplePattern::Default,
                    ],
                    expr: Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::Literal(Literal::Int(2))),
                        right: Box::new(Expr::Literal(Literal::Int(3))),
                        op: BinaryOp::Mul,
                        op_span: Span{line: 0, col: 0},
                        expr_type: Type::Int,
                    }),
                    arm_span: Span{line: 0, col: 0},
                },
            ],
            expr_type: Type::Int,
            span: Span{line: 0, col: 0},
        }), true);

        assert_eq!(result, Expr::Error);
    }

    #[test]
    fn test_cases_7() {
        // cases 3.5 {    
        //     >= 3: true,
        //     _ : false,
        // }

        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![],
            scopes: vec![],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.fold_expr(Expr::Cases(CasesExpr {
            scrutinee: Box::new(Expr::Literal(Literal::Real(3.5))),
            arms: vec![
                CasesArm {
                    pattern: vec![
                        SimplePattern::Comparison(ComparisonPattern {
                            op: CompOp::Ge,
                            expr: Box::new(Expr::Literal(Literal::Int(3))),
                        }),
                    ],
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span{line: 0, col: 0},
                },
                CasesArm {
                    pattern: vec![
                        SimplePattern::Default,
                    ],
                    expr: Expr::Literal(Literal::Bool(false)),
                    arm_span: Span{line: 0, col: 0},
                },
            ],
            expr_type: Type::Int,
            span: Span{line: 0, col: 0},
        }), true);

        assert_eq!(result, Expr::Literal(Literal::Bool(true)));
    }

    #[test]
    fn test_sample_1() {
        // sample {
        //     0+0 => true,
        //     _ => false,
        // }

        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![],
            scopes: vec![],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.fold_expr(Expr::Sample(SampleExpr {
            arms: vec![
                SampleArm {
                    prob: Prob::Expr(Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::Literal(Literal::Int(0))),
                        right: Box::new(Expr::Literal(Literal::Int(2))),
                        op: BinaryOp::Mul,
                        op_span: Span{line: 0, col: 0},
                        expr_type: Type::Int,
                    })),
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span{line: 0, col: 0},
                },
                SampleArm {
                    prob: Prob::Default,
                    expr: Expr::Literal(Literal::Bool(false)),
                    arm_span: Span{line: 0, col: 0},
                },
            ],
            expr_type: Type::Bool,
            span: Span{line: 0, col: 0},
        }), true);
        
        diagnostics.debug_print();

        assert_eq!(result, Expr::Literal(Literal::Bool(false)));
    }

    #[test]
    fn test_sample_2() {
        // sample {    // fold_sample set to false
        //     0+0 => true,
        //     _ => false,
        // }

        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![],
            scopes: vec![],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.fold_expr(Expr::Sample(SampleExpr {
            arms: vec![
                SampleArm {
                    prob: Prob::Expr(Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::Literal(Literal::Int(0))),
                        right: Box::new(Expr::Literal(Literal::Int(2))),
                        op: BinaryOp::Mul,
                        op_span: Span{line: 0, col: 0},
                        expr_type: Type::Int,
                    })),
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span{line: 0, col: 0},
                },
                SampleArm {
                    prob: Prob::Default,
                    expr: Expr::Literal(Literal::Bool(false)),
                    arm_span: Span{line: 0, col: 0},
                },
            ],
            expr_type: Type::Bool,
            span: Span{line: 0, col: 0},
        }), false);

        assert_eq!(result, Expr::Sample(SampleExpr {
            arms: vec![
                SampleArm {
                    prob: Prob::Expr(Expr::Literal(Literal::Real(0.0))),
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span{line: 0, col: 0},
                },
                SampleArm {
                    prob: Prob::Default,
                    expr: Expr::Literal(Literal::Bool(false)),
                    arm_span: Span{line: 0, col: 0},
                },
            ],
            expr_type: Type::Bool,
            span: Span{line: 0, col: 0},
        }));
    }

    #[test]
    fn test_sample_3() {
        // sample {
        //     1.2 => true, // prob greater than 1
        //     -0.1 => true, // prob less than 0
        //     _ => false, // total prob greater than 1
        // }

        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![],
            scopes: vec![],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.fold_expr(Expr::Sample(SampleExpr {
            arms: vec![
                SampleArm {
                    prob: Prob::Expr(Expr::Literal(Literal::Real(1.2))),
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span{line: 0, col: 0},
                },
                SampleArm {
                    prob: Prob::Expr(Expr::Literal(Literal::Real(-0.1))),
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span{line: 0, col: 0},
                },
                SampleArm {
                    prob: Prob::Default,
                    expr: Expr::Literal(Literal::Bool(false)),
                    arm_span: Span{line: 0, col: 0},
                },
            ],
            expr_type: Type::Bool,
            span: Span{line: 0, col: 0},
        }), true);
        
        diagnostics.debug_print();

        assert_eq!(result, Expr::Error);
        assert_eq!(diagnostics.num_errors(), 3);
    }

    #[test]
    fn test_sample_4() {
        // sample {
        //     0+0 => true,
        //     _ => false,
        //     _ => true,
        // }

        let mut diagnostics = Diagnostics::new();
        let mut sem_analyzer = SemAnalyzer {
            ast: Program {items: Vec::new()},
            symbols: vec![],
            scopes: vec![],

            diagnostics: &mut diagnostics,
        };

        let result = sem_analyzer.fold_expr(Expr::Sample(SampleExpr {
            arms: vec![
                SampleArm {
                    prob: Prob::Expr(Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::Literal(Literal::Int(0))),
                        right: Box::new(Expr::Literal(Literal::Int(2))),
                        op: BinaryOp::Mul,
                        op_span: Span{line: 0, col: 0},
                        expr_type: Type::Int,
                    })),
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span{line: 0, col: 0},
                },
                SampleArm {
                    prob: Prob::Default,
                    expr: Expr::Literal(Literal::Bool(false)),
                    arm_span: Span{line: 0, col: 0},
                },
                
                SampleArm {
                    prob: Prob::Default,
                    expr: Expr::Literal(Literal::Bool(true)),
                    arm_span: Span{line: 0, col: 0},
                },
            ],
            expr_type: Type::Bool,
            span: Span{line: 0, col: 0},
        }), true);
        
        diagnostics.debug_print();

        assert_eq!(result, Expr::Error);
        assert_eq!(diagnostics.num_errors(), 1);
    }
}