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
//! Handles expression parsing 
//!
//! ## Invariants
//!
//! - 
//!
//! Author: Cole Francis

use super::Parser;
use super::sync::SyncRule;
use super::ast::*;
use crate::compiler::{
    lexer::token::{Token, TokenKind},
    diagnostics::{CompilerError, Expected},
};

impl<'a> Parser<'a> {
    // Pratt Parser for expressions
    //     If calling to parse expr, use min_bp = 0
    //
    // Error Handling:
    //  If any portion of an expression contians an error, the entire expression will be
    //  treated as Expr::Error. 
    //      ex: (a+2, 1 + (2 + ), 1) is Expr::Error
    //  One "exception" to this is if the return portion of a match or sample expression
    //  contains an error, only the return portion of the match expression will be Expr::Error
    //      ex: match a {1 => 2+, _ => 0} is match a {1 => Expr::Error, _ => 0} not Expr::Error
    //      ex match a {1+ => 1, _ => 0} is Expr::Error
    pub(super) fn parse_expr(&mut self, min_bp: u8) -> Option<Expr> {
        let mut lhs = self.parse_prefix()?;

        loop {
            let token = self.peek().clone();

            let Some((op, left_bp, right_bp)) = 
                self.infix_into(&token)
            else {
                break;
            };

            if left_bp < min_bp {
                break;
            }

            self.next();
            
            let rhs = self.parse_expr(right_bp)?;

            lhs = Expr::Binary(BinaryExpr {
                left: Box::new(lhs),
                op,
                right: Box::new(rhs),
            });
        }

        Some(lhs)
    }

    fn parse_prefix(&mut self) -> Option<Expr> {
        let token = self.peek().clone();

        match token.kind {
            TokenKind::BoolLiteral(n) => {
                self.next();
                Some(Expr::Literal(Literal::Bool(n)))
            }

            TokenKind::IntLiteral(n) => {
                self.next();
                Some(Expr::Literal(Literal::Int(n)))
            }

            TokenKind::RealLiteral(n) => {
                self.next();
                Some(Expr::Literal(Literal::Real(n)))
            }

            TokenKind::Ident(str) => {
                self.next();
                Some(Expr::Ident(str))
            }

            TokenKind::Minus => {
                self.next();
                let rhs = self.parse_expr(25)?;
                Some(Expr::Unary(UnaryExpr {
                    op: UnaryOp::Neg,
                    expr: Box::new(rhs),
                }))
            }

            TokenKind::BitNot => {
                self.next();
                let rhs = self.parse_expr(25)?;
                Some(Expr::Unary(UnaryExpr {
                    op: UnaryOp::BitNot,
                    expr: Box::new(rhs),
                }))
            }

            TokenKind::LParen => {
                self.next();
                let first = self.parse_expr(0);

                let token = self.next();
                match token.kind {
                    TokenKind::RParen => first,

                    TokenKind::Comma => {
                        let mut elements = vec![first?];
                        elements.push(self.parse_expr(0)?);

                        while self.peek().kind == TokenKind::Comma {
                            self.next();
                            elements.push(self.parse_expr(0)?);
                        }

                        self.expect(TokenKind::RParen, &SyncRule::Expr { depth: 0 })?;
                        Some(Expr::Tuple(elements))
                    }

                    other => {
                        self.diagnostics.error(CompilerError::UnexpectedToken {
                            expected: vec![
                                Expected::Token(TokenKind::RParen),
                                Expected::Token(TokenKind::Comma),
                            ],
                            found: other,
                            span: token.span,
                        });

                        self.sync(&SyncRule::Expr { depth: 0 });

                        None
                    }
                }
            }

            TokenKind::Match => {
                self.next();
                self.parse_match()
            }

            TokenKind::Sample => {
                self.next();
                self.parse_sample()
            }

            other => {
                self.diagnostics.error(CompilerError::UnexpectedToken {
                    expected: vec![
                        Expected::Expr,
                    ],
                    found: other,
                    span: token.span,
                });

                self.sync(&SyncRule::Expr { depth: 0 });

                None
            }
        }
    }

    fn infix_into(&mut self, token: &Token) -> Option<(BinaryOp, u8, u8)> {
        match &token.kind {
            TokenKind::Gt       => Some((BinaryOp::Gt,   1,  2)),
            TokenKind::Lt       => Some((BinaryOp::Lt,   1,  2)),
            TokenKind::Ge       => Some((BinaryOp::Ge,   1,  2)),
            TokenKind::Le       => Some((BinaryOp::Le,   1,  2)),
            TokenKind::Plus     => Some((BinaryOp::Add, 10, 11)),
            TokenKind::Minus    => Some((BinaryOp::Sub, 10, 11)),
            TokenKind::Asterisk => Some((BinaryOp::Mul, 20, 21)),
            TokenKind::Slash    => Some((BinaryOp::Div, 20, 21)),
            TokenKind::Caret    => Some((BinaryOp::Pow, 31, 30)),
            _ => None,
        }
    }

    // Match token already consumed
    fn parse_match(&mut self) -> Option<Expr> {
        let scrutinee = self.parse_expr(0)?;

        self.expect(TokenKind::LBrace, &SyncRule::Expr {depth: 0})?;

        let mut arms = Vec::new();

        while self.peek().kind != TokenKind::RBrace {
            arms.push(self.parse_match_arm()?);

            if self.peek().kind == TokenKind::Comma {
                self.next();
            } else {
                break;
            }
        }

        self.expect(TokenKind::RBrace, &SyncRule::Expr {depth: 0})?;

        Some(Expr::Match(MatchExpr {
            scrutinee: Box::new(scrutinee),
            arms,
        }))
    }

    fn parse_match_arm(&mut self) -> Option<MatchArm> {
        let pattern = self.parse_pattern()?;

        self.expect(TokenKind::FatArrow, &SyncRule::Expr {depth: 1})?;

        let expr = match self.parse_expr(0) {
            Some(expr) => expr,
            None => Expr::Error,
        };

        Some(MatchArm {
            pattern,
            expr,
        })
    }

    fn parse_pattern(&mut self) -> Option<Vec<SimplePattern>> {
        let mut patterns = vec![self.parse_simple_pattern()?];

        while self.peek().kind == TokenKind::Pipe {
            self.next();
            patterns.push(self.parse_simple_pattern()?);
        }

        Some(patterns)
    }

    fn parse_simple_pattern(&mut self) -> Option<SimplePattern> {
        let token = self.next();

        match token.kind {
            TokenKind::Underscore => Some(SimplePattern::Default),

            TokenKind::BoolLiteral(n) => Some(SimplePattern::Literal(Literal::Bool(n))),
            TokenKind::IntLiteral(n)  => Some(SimplePattern::Literal(Literal::Int(n))),
            TokenKind::RealLiteral(n) => Some(SimplePattern::Literal(Literal::Real(n))),

            TokenKind::Ident(name) => Some(SimplePattern::Ident(name)),

            TokenKind::LParen => self.parse_tuple_pattern(),

            TokenKind::Gt => self.parse_comparison_pattern(CompOp::Gt),
            TokenKind::Lt => self.parse_comparison_pattern(CompOp::Lt),
            TokenKind::Ge => self.parse_comparison_pattern(CompOp::Ge),
            TokenKind::Le => self.parse_comparison_pattern(CompOp::Le),

            other => {
                self.diagnostics.error(CompilerError::UnexpectedToken {
                    expected: vec![
                        Expected::Pattern,
                    ],
                    found: other,
                    span: token.span,
                });

                self.sync(&SyncRule::Expr {depth: 1});

                None
            }
        }
    }

    fn parse_tuple_pattern(&mut self) -> Option<SimplePattern> {
        let mut items = vec![self.parse_simple_pattern()?];

        self.expect(TokenKind::Comma, &SyncRule::Expr {depth: 1})?;

        items.push(self.parse_simple_pattern()?);

        while self.peek().kind == TokenKind::Comma {
            self.next();
            items.push(self.parse_simple_pattern()?);
        }

        self.expect(TokenKind::RParen, &SyncRule::Expr {depth: 1})?;

        Some(SimplePattern::Tuple(items))
    }

    fn parse_comparison_pattern(&mut self, op: CompOp) -> Option<SimplePattern> {
        let expr = self.parse_expr(0)?;

        Some(SimplePattern::Comparison(ComparisonPattern {
            op,
            expr: Box::new(expr),
        }))
    }

    // Sample token already consumed
    fn parse_sample(&mut self) -> Option<Expr> {
        self.expect(TokenKind::LBrace, &SyncRule::Expr {depth: 0})?;

        let mut arms = Vec::new();

        while self.peek().kind != TokenKind::RBrace {
            arms.push(self.parse_sample_arm()?);
            if self.peek().kind == TokenKind::Comma {
                self.next();
            } else {
                break;
            }
        }

        self.expect(TokenKind::RBrace, &SyncRule::Expr {depth: 0})?;

        Some(Expr::Sample(arms))
    }

    fn parse_sample_arm(&mut self) -> Option<SampleArm> {
        let prob = match &self.peek().kind {
            TokenKind::Underscore => {
                self.next();
                Prob::Default
            },
            _ => Prob::Expr( match self.parse_expr(0) {
                Some(expr) => expr,
                None => Expr::Error,
            }),
        };

        self.expect(TokenKind::FatArrow, &SyncRule::Expr {depth: 1})?;

        let expr = match self.parse_expr(0) {
            Some(expr) => expr,
            None => Expr::Error,
        };

        Some(SampleArm {
            prob,
            expr,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer::token::TokenKind::*;
    use crate::compiler::diagnostics::{Diagnostics, Span};

    fn build_token_vec(tokens: Vec<TokenKind>) -> Vec<Token> {
        tokens
            .into_iter()
            .map(|x| Token {kind: x, span: Span{line: 0, col: 0}})
            .collect()
    }

    fn build_s_expr(expr: &Expr) -> String {
        match expr {
            Expr::Literal(Literal::Int(n)) => n.to_string(),
            Expr::Literal(Literal::Bool(b)) => b.to_string(),
            Expr::Literal(Literal::Real(x)) => x.to_string(),

            Expr::Ident(name) => name.clone(),

            Expr::Unary(unary) => {
                format!(
                    "({} {})",
                    unary_op_to_str(&unary.op),
                    build_s_expr(&unary.expr),
                )
            }

            Expr::Binary(binary) => {
                format!(
                    "({} {} {})",
                    binary_op_to_str(&binary.op),
                    build_s_expr(&binary.left),
                    build_s_expr(&binary.right),
                )
            }

            // (tuple a b c)
            Expr::Tuple(elements) => {
                let elems = elements
                    .iter()
                    .map(build_s_expr)
                    .collect::<Vec<_>>()
                    .join(" ");

                format!("(tuple {})", elems)
            }

            // (match x (arm 0 1) (arm _ 2))
            Expr::Match(match_expr) => {
                let arms = match_expr 
                    .arms
                    .iter()
                    .map(build_match_arm)
                    .collect::<Vec<_>>()
                    .join(" ");

                format!("(match {} {})", build_s_expr(&match_expr.scrutinee), arms)
            }

            // (sample (arm 0.5 1) (arm _ 0))
            Expr::Sample(arms) => {
                let arms = arms
                    .iter()
                    .map(build_sample_arm)
                    .collect::<Vec<_>>()
                    .join(" ");

                format!("(sample {})", arms)
            }

            Expr::Error => {
                format!("(error)")
            }
        }
    }

    fn build_match_arm(arm: &MatchArm) -> String {
        let pattern = if arm.pattern.len() == 1 {
            build_pattern(&arm.pattern[0])
        } else {
            let patterns = arm.pattern
                .iter()
                .map(build_pattern)
                .collect::<Vec<_>>()
                .join(" ");

            format!("(or {})", patterns)
        };

        format!(
            "(arm {} {})",
            pattern,
            build_s_expr(&arm.expr),
        )
    }

    fn build_pattern(pattern: &SimplePattern) -> String {
        match pattern {
            SimplePattern::Default => "_".to_string(),

            SimplePattern::Literal(Literal::Int(n)) => n.to_string(),
            SimplePattern::Literal(Literal::Bool(b)) => b.to_string(),
            SimplePattern::Literal(Literal::Real(x)) => x.to_string(),

            SimplePattern::Ident(ident) => ident.to_string(),

            SimplePattern::Tuple(elements) => {
                let elems = elements
                    .iter()
                    .map(build_pattern)
                    .collect::<Vec<_>>()
                    .join(" ");

                format!("(tuple {})", elems)
            }

            SimplePattern::Comparison(comp) => {
                format!(
                    "({} {})",
                    comp_op_to_str(&comp.op),
                    build_s_expr(&comp.expr),
                )
            }
        }
    }

    fn build_sample_arm(arm: &SampleArm) -> String {
        let prob = match &arm.prob {
            Prob::Default => "_".to_string(),
            Prob::Expr(expr) =>  build_s_expr(expr),
        };

        format!(
            "(arm {} {})",
            prob,
            build_s_expr(&arm.expr),
        )
    }

    fn unary_op_to_str(op: &UnaryOp) -> &'static str {
        match op {
            UnaryOp::Neg    => "-",
            UnaryOp::BitNot => "~",
        }
    }

    fn binary_op_to_str(op: &BinaryOp) -> &'static str {
        match op {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Pow => "^",
            BinaryOp::Gt  => ">",
            BinaryOp::Lt  => "<",
            BinaryOp::Ge  => ">=",
            BinaryOp::Le  => "<=",
        }
    }

    fn comp_op_to_str(op: &CompOp) -> &'static str {
        match op {
            CompOp::Gt  => ">",
            CompOp::Lt  => "<",
            CompOp::Ge  => ">=",
            CompOp::Le  => "<=",
        }
    }

    #[test]
    fn test_build_s_expr() {
        let start = Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Unary(UnaryExpr {
                op: UnaryOp::Neg,
                expr: Box::new(Expr::Literal(Literal::Int(5))),
            })),
            op: BinaryOp::Add,
            right: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Literal(Literal::Int(2))),
                op: BinaryOp::Mul,
                right: Box::new(Expr::Ident("a".to_string())),
            })),
        });

        let result: String = build_s_expr(&start);

        assert_eq!(result, "(+ (- 5) (* 2 a))".to_string());
    }

    #[test]
    fn test_no_expr() {
        let kinds: Vec<TokenKind> = vec![Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_expr(0);

        diagnostics.debug_print();

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn literal_and_ident_expr() {
        let kinds: Vec<TokenKind> = vec![IntLiteral(3), Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_prefix().unwrap();

        assert_eq!(result, Expr::Literal(Literal::Int(3)));
    
        let kinds: Vec<TokenKind> = vec![Ident("hey".to_string()), Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_prefix().unwrap();

        assert_eq!(result, Expr::Ident("hey".to_string()));
    }
    
    #[test]
    fn unary_expr() {
        // ---6
        let kinds: Vec<TokenKind> = vec![Minus, Minus, Minus, IntLiteral(6), Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);
        
        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_prefix().unwrap();
        
        let result_str: String = build_s_expr(&result);

        assert_eq!(result_str, "(- (- (- 6)))".to_string());
    }

    #[test]
    fn binary_expr() {
        // -5 + 2 * a + b
        let kinds: Vec<TokenKind> = vec![Minus, IntLiteral(5), Plus, 
            IntLiteral(2), Asterisk, Ident("a".to_string()), Plus, 
            Ident("b".to_string()), Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_expr(0).unwrap();

        let result_str: String = build_s_expr(&result);

        assert_eq!(result_str, "(+ (+ (- 5) (* 2 a)) b)".to_string()); 
    
        // (9 + 10) * 5
        let kinds: Vec<TokenKind> = vec![LParen, IntLiteral(9), Plus, 
        IntLiteral(10), RParen, Asterisk, IntLiteral(5), Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_expr(0).unwrap();

        let result_str: String = build_s_expr(&result);

        assert_eq!(result_str, "(* (+ 9 10) 5)".to_string()); 
    
        //-3^(-7)^(8-2-4/-1)
        let kinds: Vec<TokenKind> = vec![Minus, IntLiteral(3), Caret, LParen, 
            Minus, IntLiteral(7), RParen, Caret, LParen, IntLiteral(8), Minus,
            IntLiteral(2), Minus, IntLiteral(4), Slash, Minus, IntLiteral(1), 
            RParen, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_expr(0).unwrap();

        let result_str: String = build_s_expr(&result);

        assert_eq!(result_str, "(- (^ 3 (^ (- 7) (- (- 8 2) (/ 4 (- 1))))))".to_string()); 
    }
    
    #[test]
    fn tuple_expr_1() {
        // (1, 1+5, 3)
        let kinds: Vec<TokenKind> = vec![LParen, IntLiteral(1), Comma, 
            IntLiteral(1), Plus, IntLiteral(5), Comma,
            IntLiteral(3), RParen, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result: Expr = parser.parse_expr(0).unwrap();

        let result_str: String = build_s_expr(&result);

        assert_eq!(result_str, "(tuple 1 (+ 1 5) 3)".to_string()); 
    }

    #[test]
    fn tuple_expr_2() {    
        // (1, (2, 3))
        let kinds: Vec<TokenKind> = vec![LParen, IntLiteral(1), Comma, 
            LParen, IntLiteral(2), Comma,
            IntLiteral(3), RParen, RParen, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result= parser.parse_expr(0).unwrap();

        let result_str: String = build_s_expr(&result);

        assert_eq!(result_str, "(tuple 1 (tuple 2 3))".to_string()); 
    }

    #[test]
    fn match_expr() {
        // match a {
        //     1 => 1+a,
        //     _ => a,
        // } - 2
        let kinds: Vec<TokenKind> = vec![Match, Ident("a".to_string()), LBrace, 
            IntLiteral(1), FatArrow, IntLiteral(1), Plus, Ident("a".to_string()), Comma,
            Underscore, FatArrow, Ident("a".to_string()), Comma,
            RBrace, Minus, IntLiteral(2), Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result= parser.parse_expr(0).unwrap();

        let result_str: String = build_s_expr(&result);

        assert_eq!(result_str, "(- (match a (arm 1 (+ 1 a)) (arm _ a)) 2)".to_string());

        // match (a, b) {
        //     (1, 0) | (0, 1) | (1, 1) => 1,
        //     _ => 0,
        // }
        let kinds: Vec<TokenKind> = vec![Match, LParen, Ident("a".to_string()), Comma,  Ident("b".to_string()), RParen, LBrace,
                LParen, IntLiteral(1), Comma, IntLiteral(0), RParen, Pipe, 
                LParen, IntLiteral(0), Comma, IntLiteral(1), RParen, Pipe,
                LParen, IntLiteral(1), Comma, IntLiteral(1), RParen,
                FatArrow, IntLiteral(1), Comma,
            Underscore, FatArrow, IntLiteral(0),
            RBrace, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result= parser.parse_expr(0).unwrap();

        let result_str: String = build_s_expr(&result);

        assert_eq!(result_str, "(match (tuple a b) (arm (or (tuple 1 0) (tuple 0 1) (tuple 1 1)) 1) (arm _ 0))".to_string());
    }

    #[test]
    fn sample_expr() {
        // sample {
        //     0.5 => 1,
        //     _ => 0,
        // }
        let kinds: Vec<TokenKind> = vec![Sample, LBrace,
            RealLiteral(0.5), FatArrow, IntLiteral(1), Comma,
            Underscore, FatArrow, IntLiteral(0), Comma,
            RBrace, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result= parser.parse_expr(0).unwrap();

        let result_str: String = build_s_expr(&result);

        assert_eq!(result_str, "(sample (arm 0.5 1) (arm _ 0))".to_string());

        // match coin {
        //     H => sample {
        //         0.1 => H,
        //         _ => T
        //     },
        //     T => sample {
        //         0.8 => H,
        //         _ => T
        //     },
        // }
        let kinds: Vec<TokenKind> = vec![Match, Ident("coin".to_string()), LBrace,
            Ident("H".to_string()), FatArrow, Sample, LBrace,
                RealLiteral(0.1), FatArrow, Ident("H".to_string()), Comma,
                Underscore, FatArrow, Ident("T".to_string()),
            RBrace, Comma,
            Ident("T".to_string()), FatArrow, Sample, LBrace,
                RealLiteral(0.8), FatArrow, Ident("H".to_string()), Comma,
                Underscore, FatArrow, Ident("T".to_string()),
            RBrace, Comma,
            RBrace, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result= parser.parse_expr(0).unwrap();

        let result_str: String = build_s_expr(&result);

        assert_eq!(result_str, "(match coin (arm H (sample (arm 0.1 H) (arm _ T))) (arm T (sample (arm 0.8 H) (arm _ T))))".to_string());

        // sample {}
        let kinds: Vec<TokenKind> = vec![Sample, LBrace, RBrace, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_expr(0);

        assert_eq!(result, Some(Expr::Sample(vec![])));

        // sample {
        //     a => sample {
        //        a => b        // no fat arrow
        //     },
        //     _ => c
        // }
        let kinds: Vec<TokenKind> = vec![Sample, LBrace,
            Ident("a".to_string()), FatArrow, Sample, LBrace, 
                Ident("a".to_string()), FatArrow, Ident("b".to_string()),
            RBrace, Comma,
            Underscore, FatArrow, Ident("c".to_string()), Comma,
            RBrace, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_expr(0);

        diagnostics.debug_print();

        assert_eq!(result, Some(Expr::Sample(vec![
            SampleArm {
                prob: Prob::Expr(Expr::Ident("a".to_string())),
                expr: Expr::Sample(vec![SampleArm {
                    prob: Prob::Expr(Expr::Ident("a".to_string())),
                    expr: Expr::Ident("b".to_string()),
                }]),
            },
            SampleArm {
                prob: Prob::Default,
                expr: Expr::Ident("c".to_string()),
            },
        ])));
    }

    #[test]
    fn bad_unary_expr() {
        // @b
        let kinds: Vec<TokenKind> = vec![ErrorToken, Ident("b".to_string()), Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_expr(0);

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn bad_tuple_expr() {
        // (1, , 3)
        let kinds: Vec<TokenKind> = vec![LParen, IntLiteral(1), Comma, Comma, IntLiteral(3), RParen, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_expr(0);

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn bad_match_1() {
        //     match { // missing expression after match
        //         a => 1,
        //         _ => 0,
        //     }
        // }
        let kinds: Vec<TokenKind> = vec![Match, LBrace,
            Ident("a".to_string()), FatArrow, IntLiteral(1), Comma, 
            Underscore, FatArrow, IntLiteral(0), Comma,
            RBrace, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_expr(0);

        diagnostics.debug_print();

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn bad_match_2() {
        //     match a { 
        //         @ => 1, // unknown token
        //         _ => 0,
        //     }
        // }
        let kinds: Vec<TokenKind> = vec![Match, Ident("a".to_string()), LBrace,
            ErrorToken, FatArrow, IntLiteral(1), Comma, 
            Underscore, FatArrow, IntLiteral(0), Comma,
            RBrace, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_expr(0);

        diagnostics.debug_print();

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn bad_match_3() {
        //     match a { 
        //         => 1, // missing expr
        //         _ => 0,
        //     }
        // }
        let kinds: Vec<TokenKind> = vec![Match, Ident("a".to_string()), LBrace,
            FatArrow, IntLiteral(1), Comma, 
            Underscore, FatArrow, IntLiteral(0), Comma,
            RBrace, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_expr(0);

        diagnostics.debug_print();

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn bad_match_4() {
        //     match a { 
        //         b => , // missing expr
        //         _ => 0,
        //     }
        // }
        let kinds: Vec<TokenKind> = vec![Match, Ident("a".to_string()), LBrace,
            Ident("b".to_string()), FatArrow, Comma, 
            Underscore, FatArrow, IntLiteral(0), Comma,
            RBrace, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_expr(0);

        diagnostics.debug_print();

        assert_eq!(result, Some(Expr::Match(MatchExpr {
            scrutinee: Box::new(Expr::Ident("a".to_string())),
            arms: vec![
                MatchArm {
                    pattern: vec![SimplePattern::Ident("b".to_string())],
                    expr: Expr::Error,
                },
                MatchArm {
                    pattern: vec![SimplePattern::Default],
                    expr: Expr::Literal(Literal::Int(0)),
                }
            ],
        })));
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn bad_sample_expr_1() {
        // sample {
        //     b 1,  // no fat arrow
        //     _ c,  // no fat arrow
        // }
        let kinds: Vec<TokenKind> = vec![Sample, LBrace,
            Ident("b".to_string()), IntLiteral(1), Comma,
            Underscore, Ident("c".to_string()), Comma,
            RBrace, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_expr(0);

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn bad_sample_expr_2() {
        // sample {
        //     b => 1   // no comma
        //     _ => c
        // }
        let kinds: Vec<TokenKind> = vec![Sample, LBrace,
            Ident("b".to_string()), FatArrow, IntLiteral(1),
            Underscore, FatArrow, Ident("c".to_string()), 
            RBrace];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_expr(0);

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn bad_sample_expr_3() {
        // sample {
        //     a => sample {, // No closing brace causes last } to be mistaken for the second samples closing.
        //     _ => c
        // }
        let kinds: Vec<TokenKind> = vec![Sample, LBrace,
            Ident("a".to_string()), FatArrow, Sample, LBrace, Comma,
            Underscore, FatArrow, Ident("c".to_string()), Comma,
            RBrace, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_expr(0);

        assert_eq!(result, None);
    }

    #[test]
    fn bad_sample_expr_4() {
        // sample {
        //     a => sample {
        //        a b        // no fat arrow
        //     },
        //     _ => c
        // }
        let kinds: Vec<TokenKind> = vec![Sample, LBrace,
            Ident("a".to_string()), FatArrow, Sample, LBrace, 
                Ident("a".to_string()), Ident("b".to_string()),
            RBrace, Comma,
            Underscore, FatArrow, Ident("c".to_string()), Comma,
            RBrace, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_expr(0);

        assert_eq!(result, Some(Expr::Sample(vec![
            SampleArm {
                prob: Prob::Expr(Expr::Ident("a".to_string())),
                expr: Expr::Error,
            },
            SampleArm {
                prob: Prob::Default,
                expr: Expr::Ident("c".to_string()),
            },
        ])));
        assert_eq!(diagnostics.num_errors(), 1);
    }
}