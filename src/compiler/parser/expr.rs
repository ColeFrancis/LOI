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
//! - Expr types will always be unknown until type checking
//!
//! Author: Cole Francis

use super::Parser;
use super::sync::SyncRule;
use crate::compiler::sem_analyzer::types::Type;
use crate::compiler::{
    ast::*,
    lexer::token::{Token, TokenKind},
    diagnostics::{CompilerError, Expected, Span},
};

impl<'a> Parser<'a> {
    // Pratt Parser for expressions
    //     If calling to parse expr, use min_bp = 0
    //
    // Error Handling:
    //  If any portion of an expression contians an error, the entire expression will be
    //  treated as Expr::Error. 
    //      ex: (a+2, 1 + (2 + ), 1) is Expr::Error because nothing is added to 2
    //  Three "exceptions" to this are:
    //      if one of the expressions in a block expression is an error, only that expression is Expr::Error not the entire block.
    //      if one part of a tuple expression is an error, only that portion is Expr::Error and not the whole tuple expression.
    //      if the return portion of a cases or sample expression contains an error, 
    //     only the return portion of the cases expression will be Expr::Error and no the entire cases expression.
    //          ex: cases a {1 : 2+, _ : 0} is cases a {1 : Expr::Error, _ : 0} not Expr::Error
    //          ex cases a {1+ : 1, _ : 0} is Expr::Error
    pub(super) fn parse_expr(&mut self, min_bp: u8) -> Option<Expr> {
        let mut lhs = self.parse_prefix()?;

        loop {
            let token = self.peek().clone();

            let Some((op, op_span, left_bp, right_bp)) = 
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
                op_span,
                expr_type: Type::Unknown,
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
                Some(Expr::Ident(Ident::Str{ val: str, span: token.span }))
            }

            TokenKind::Minus => {
                self.next();
                let rhs = self.parse_expr(25)?;
                Some(Expr::Unary(UnaryExpr {
                    op: UnaryOp::Neg,
                    expr: Box::new(rhs),
                    op_span: token.span,
                    expr_type: Type::Unknown,
                }))
            }

            TokenKind::BitNot => {
                self.next();
                let rhs = self.parse_expr(25)?;
                Some(Expr::Unary(UnaryExpr {
                    op: UnaryOp::BitNot,
                    expr: Box::new(rhs),
                    op_span: token.span,
                    expr_type: Type::Unknown,
                }))
            }

            TokenKind::LParen => {
                self.next();
                let first = self.parse_expr(0);

                let token = self.next();
                match token.kind {
                    TokenKind::RParen => first,

                    TokenKind::Comma => {
                        let mut elements = vec![match first {
                            Some(expr) => expr,
                            None => Expr::Error,
                        }];

                        elements.push( match self.parse_expr(0) {
                            Some(expr) => expr,
                            None => Expr::Error,
                        });

                        while self.peek().kind == TokenKind::Comma {
                            self.next();
                            elements.push( match self.parse_expr(0) {
                                Some(expr) => expr,
                                None => Expr::Error,
                            });
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

            TokenKind::LBrace => {
                self.next();
                self.parse_block_expr()
            }

            TokenKind::Cases => {
                let span = token.span;
                self.next();
                self.parse_cases(span)
            }

            TokenKind::Sample => {
                let span = token.span;
                self.next();
                self.parse_sample(span)
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

    fn infix_into(&mut self, token: &Token) -> Option<(BinaryOp, Span, u8, u8)> {
        match &token.kind {
            TokenKind::Gt       => Some((BinaryOp::Gt,  token.span.clone(),  1,  2)),
            TokenKind::Lt       => Some((BinaryOp::Lt,  token.span.clone(),  1,  2)),
            TokenKind::Ge       => Some((BinaryOp::Ge,  token.span.clone(),  1,  2)),
            TokenKind::Le       => Some((BinaryOp::Le,  token.span.clone(),  1,  2)),
            TokenKind::Plus     => Some((BinaryOp::Add, token.span.clone(), 10, 11)),
            TokenKind::Minus    => Some((BinaryOp::Sub, token.span.clone(), 10, 11)),
            TokenKind::Asterisk => Some((BinaryOp::Mul, token.span.clone(), 20, 21)),
            TokenKind::Slash    => Some((BinaryOp::Div, token.span.clone(), 20, 21)),
            TokenKind::Caret    => Some((BinaryOp::Pow, token.span.clone(), 31, 30)),
            TokenKind::Pipe     => Some((BinaryOp::Or,  token.span.clone(), 10, 11)),
            TokenKind::Ampersand=> Some((BinaryOp::And, token.span.clone(), 20, 21)),
            _ => None,
        }
    }


    // { already consumed
    fn parse_block_expr(&mut self) -> Option<Expr> {
        let mut statements = Vec::new();

        while self.peek().kind == TokenKind::Let {
            self.next();

            statements.push(match self.parse_let_stmt() {
                Some(stmt) => Statement::Let(stmt),
                None => Statement::Error,
            });
        }

        let expr = match self.parse_expr(0) {
            Some(expr) => expr,
            None => Expr::Error,
        };

        self.expect(TokenKind::RBrace, &SyncRule::Item)?;

        Some(Expr::Block(BlockExpr {
            statements,
            expr: Box::new(expr),
            expr_type: Type::Unknown,
        }))
    }

    // Cases token already consumed
    fn parse_cases(&mut self, span: Span) -> Option<Expr> {
        let scrutinee = self.parse_expr(0)?;

        self.expect(TokenKind::LBrace, &SyncRule::Expr {depth: 0})?;

        let mut arms = Vec::new();

        while self.peek().kind != TokenKind::RBrace {
            arms.push(self.parse_cases_arm()?);

            if self.peek().kind == TokenKind::Comma {
                self.next();
            } else {
                break;
            }
        }

        self.expect(TokenKind::RBrace, &SyncRule::Expr {depth: 0})?;

        Some(Expr::Cases(CasesExpr {
            scrutinee: Box::new(scrutinee),
            arms,
            expr_type: Type::Unknown,
            span,
        }))
    }

    fn parse_cases_arm(&mut self) -> Option<CasesArm> {
        let arm_span = self.peek().span.clone();

        let pattern = self.parse_pattern()?;

        self.expect(TokenKind::Colon, &SyncRule::Expr {depth: 1})?;

        let expr = match self.parse_expr(0) {
            Some(expr) => expr,
            None => Expr::Error,
        };

        Some(CasesArm {
            pattern,
            expr,
            arm_span,
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

            TokenKind::Ident(name) => Some(SimplePattern::Ident(Ident::Str { val: name, span: token.span })),

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
    fn parse_sample(&mut self, span: Span) -> Option<Expr> {
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

        Some(Expr::Sample(SampleExpr {
            arms,
            expr_type: Type::Unknown,
            span,
        }))
    }

    fn parse_sample_arm(&mut self) -> Option<SampleArm> {
        let arm_span = self.peek().span.clone();

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

        self.expect(TokenKind::Colon, &SyncRule::Expr {depth: 1})?;

        let expr = match self.parse_expr(0) {
            Some(expr) => expr,
            None => Expr::Error,
        };

        Some(SampleArm {
            prob,
            expr,
            arm_span,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer::token::TokenKind::*;
    use crate::compiler::diagnostics::{Diagnostics, Span};
    use crate::compiler::ast;

    fn build_token_vec(tokens: Vec<TokenKind>) -> Vec<Token> {
        tokens
            .into_iter()
            .map(|x| Token {kind: x, span: Span{line: 0, col: 0}})
            .collect()
    }

    fn build_ident_str(name: &str) -> ast::Ident {
        ast::Ident::Str {
            val: name.to_string(),
            span: Span{line: 0, col: 0},
        }
    }

    fn build_s_expr(expr: &Expr) -> String {
        match expr {
            Expr::Literal(Literal::Int(n)) => n.to_string(),
            Expr::Literal(Literal::Bool(b)) => b.to_string(),
            Expr::Literal(Literal::Real(x)) => x.to_string(),

            Expr::Ident(ident) => match ident {
                ast::Ident::Str {val, ..} => val.clone(),
                ast::Ident::Symbol(id) => format!("sym:{}", id), // should be unreachable
            }

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

            // (block (let ident a) b)
            Expr::Block(block) => {
                let statements = block
                    .statements
                    .iter()
                    .map(build_statement)
                    .collect::<Vec<_>>()
                    .join(" ");

                format!("(block {} {})", statements, build_s_expr(&block.expr))
            }

            // (cases x (arm 0 1) (arm _ 2))
            Expr::Cases(cases_expr) => {
                let arms = cases_expr 
                    .arms
                    .iter()
                    .map(build_cases_arm)
                    .collect::<Vec<_>>()
                    .join(" ");

                format!("(cases {} {})", build_s_expr(&cases_expr.scrutinee), arms)
            }

            // (sample (arm 0.5 1) (arm _ 0))
            Expr::Sample(sample_expr) => {
                let arms = sample_expr
                    .arms
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

    fn build_statement(stmt: &Statement) -> String {
        match stmt {
            Statement::Let(let_stmt) => {
                let name = match &let_stmt.name {
                    ast::Ident::Str {val, ..} => val.clone(),
                    ast::Ident::Symbol(id) => format!("sym:{}", id), // should be unreachable
                };

                format!("(let {} {})", name, build_s_expr(&let_stmt.expr))
            }
            Statement::Error => {
                format!("(error)")
            }
        }
    }

    fn build_cases_arm(arm: &CasesArm) -> String {
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

            SimplePattern::Ident(ident) => match ident {
                ast::Ident::Str {val, ..} => val.clone(),
                ast::Ident::Symbol(id) => format!("sym:{}", id), // should be unreachable
            }

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

            SimplePattern::Error => {
                format!("(error)")
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
            BinaryOp::Or  => "|",
            BinaryOp::And => "&",
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
                op_span: Span {line: 0, col: 0},
                expr_type: Type::Unknown,
            })),
            op: BinaryOp::Add,
            right: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Literal(Literal::Int(2))),
                op: BinaryOp::Mul,
                right: Box::new(Expr::Ident(build_ident_str("a"))),
                op_span: Span {line: 0, col: 0},
                expr_type: Type::Unknown
            })),
            op_span: Span {line: 0, col: 0},
            expr_type: Type::Unknown,
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

        assert_eq!(result, Expr::Ident(build_ident_str("hey")));
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
    
        // (9 + 10) | 5
        let kinds: Vec<TokenKind> = vec![LParen, IntLiteral(9), Plus, 
        IntLiteral(10), RParen, Pipe, IntLiteral(5), Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_expr(0).unwrap();

        let result_str: String = build_s_expr(&result);

        assert_eq!(result_str, "(| (+ 9 10) 5)".to_string()); 
    
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
    fn block_expr() {
        // {
        //     let n = 1;
            
        //     n + 1
        // } + 2
        let kinds: Vec<TokenKind> = vec![LBrace, 
            Let, Ident("n".to_string()), Equals, IntLiteral(1), Semicolon,
            Ident("n".to_string()), Plus, IntLiteral(1),
            RBrace, Plus, IntLiteral(2), Eof];
        let tokens = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let result = Parser::new(tokens, &mut diagnostics).parse_expr(0).unwrap();

        let result_str = build_s_expr(&result);

        assert_eq!(result_str, "(+ (block (let n 1) (+ n 1)) 2)".to_string());
    }

    #[test]
    fn match_expr() {
        // cases a {
        //     1 : 1+a,
        //     _ : a,
        // } - 2
        let kinds: Vec<TokenKind> = vec![Cases, Ident("a".to_string()), LBrace, 
            IntLiteral(1), Colon, IntLiteral(1), Plus, Ident("a".to_string()), Comma,
            Underscore, Colon, Ident("a".to_string()), Comma,
            RBrace, Minus, IntLiteral(2), Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result= parser.parse_expr(0).unwrap();

        let result_str: String = build_s_expr(&result);

        assert_eq!(result_str, "(- (cases a (arm 1 (+ 1 a)) (arm _ a)) 2)".to_string());

        // cases (a, b) {
        //     (1, 0) | (0, 1) | (1, 1) : 1,
        //     _ : 0,
        // }
        let kinds: Vec<TokenKind> = vec![Cases, LParen, Ident("a".to_string()), Comma,  Ident("b".to_string()), RParen, LBrace,
                LParen, IntLiteral(1), Comma, IntLiteral(0), RParen, Pipe, 
                LParen, IntLiteral(0), Comma, IntLiteral(1), RParen, Pipe,
                LParen, IntLiteral(1), Comma, IntLiteral(1), RParen,
                Colon, IntLiteral(1), Comma,
            Underscore, Colon, IntLiteral(0),
            RBrace, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result= parser.parse_expr(0).unwrap();

        let result_str: String = build_s_expr(&result);

        assert_eq!(result_str, "(cases (tuple a b) (arm (or (tuple 1 0) (tuple 0 1) (tuple 1 1)) 1) (arm _ 0))".to_string());
    }

    #[test]
    fn sample_expr() {
        // sample {
        //     0.5 : 1,
        //     _ : 0,
        // }
        let kinds: Vec<TokenKind> = vec![Sample, LBrace,
            RealLiteral(0.5), Colon, IntLiteral(1), Comma,
            Underscore, Colon, IntLiteral(0), Comma,
            RBrace, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result= parser.parse_expr(0).unwrap();

        let result_str: String = build_s_expr(&result);

        assert_eq!(result_str, "(sample (arm 0.5 1) (arm _ 0))".to_string());

        // cases coin {
        //     H : sample {
        //         0.1 : H,
        //         _ : T
        //     },
        //     T : sample {
        //         0.8 : H,
        //         _ : T
        //     },
        // }
        let kinds: Vec<TokenKind> = vec![Cases, Ident("coin".to_string()), LBrace,
            Ident("H".to_string()), Colon, Sample, LBrace,
                RealLiteral(0.1), Colon, Ident("H".to_string()), Comma,
                Underscore, Colon, Ident("T".to_string()),
            RBrace, Comma,
            Ident("T".to_string()), Colon, Sample, LBrace,
                RealLiteral(0.8), Colon, Ident("H".to_string()), Comma,
                Underscore, Colon, Ident("T".to_string()),
            RBrace, Comma,
            RBrace, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result= parser.parse_expr(0).unwrap();

        let result_str: String = build_s_expr(&result);

        assert_eq!(result_str, "(cases coin (arm H (sample (arm 0.1 H) (arm _ T))) (arm T (sample (arm 0.8 H) (arm _ T))))".to_string());

        // sample {}
        let kinds: Vec<TokenKind> = vec![Sample, LBrace, RBrace, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_expr(0);

        assert_eq!(result, Some(Expr::Sample(SampleExpr {arms: vec![], expr_type: Type::Unknown, span: Span {line: 0, col: 0}})));

        // sample {
        //     a : sample {
        //        a : b        // no fat arrow
        //     },
        //     _ : c
        // }
        let kinds: Vec<TokenKind> = vec![Sample, LBrace,
            Ident("a".to_string()), Colon, Sample, LBrace, 
                Ident("a".to_string()), Colon, Ident("b".to_string()),
            RBrace, Comma,
            Underscore, Colon, Ident("c".to_string()), Comma,
            RBrace, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_expr(0);

        diagnostics.debug_print();

        assert_eq!(result, Some(Expr::Sample(SampleExpr {
            arms: vec![
                SampleArm {
                    prob: Prob::Expr(Expr::Ident(build_ident_str("a"))),
                    expr: Expr::Sample(SampleExpr {
                        arms: vec![SampleArm {
                            prob: Prob::Expr(Expr::Ident(build_ident_str("a"))),
                            expr: Expr::Ident(build_ident_str("b")),
                            arm_span: Span {line: 0, col: 0},
                        }],
                        expr_type: Type::Unknown,
                        span: Span {line: 0, col: 0},
                    }),
                    arm_span: Span {line: 0, col: 0},
                },
                SampleArm {
                    prob: Prob::Default,
                    expr: Expr::Ident(build_ident_str("c")),
                    arm_span: Span {line: 0, col: 0},
                },
            ],
            expr_type: Type::Unknown,
            span: Span {line: 0, col: 0},
        })));
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

        diagnostics.debug_print();

        assert_eq!(result, Some(Expr::Tuple(vec![
            Expr::Literal(Literal::Int(1)),
            Expr::Error,
            Expr::Literal(Literal::Int(3)),
        ])));
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn bad_block_expr_1() {
        // {
        //     let m = 1;
        //     let n = 1 // no semicolon
        //     n + 1
        // }
        let kinds: Vec<TokenKind> = vec![LBrace, 
            Let, Ident("m".to_string()), Equals, IntLiteral(1), Semicolon,
            Let, Ident("n".to_string()), Equals, IntLiteral(1),
            Ident("n".to_string()), Plus, IntLiteral(1),
            RBrace, Eof];
        let tokens = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let result = Parser::new(tokens, &mut diagnostics).parse_expr(0);

        diagnostics.debug_print();

        // Synchronization will eat the expression coming after, 
        //  then the block expression parser will expect an expression and
        //  emit another error
        assert_eq!(result, Some(Expr::Block(BlockExpr {
            statements: vec![
                Statement::Let(LetStatement {
                    name: ast::Ident::Str{
                        val: "m".to_string(),
                        span: Span{line: 0, col: 0},
                    },
                    expr: Expr::Literal(Literal::Int(1)),
                }),
                Statement::Error,
            ],
            expr: Box::new(Expr::Error),
            expr_type: Type::Unknown,
        })));
        assert_eq!(diagnostics.num_errors(), 2);
    }

    #[test]
    fn bad_block_expr_2() {
        // {
        //     let n = 1;
            
        //     // no expr
        // }
        let kinds: Vec<TokenKind> = vec![LBrace, 
            Let, Ident("n".to_string()), Equals, IntLiteral(1), Semicolon,
            RBrace, Eof];
        let tokens = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let result = Parser::new(tokens, &mut diagnostics).parse_expr(0);

        diagnostics.debug_print();

        assert_eq!(result, Some(Expr::Block(BlockExpr {
            statements: vec![
                Statement::Let(LetStatement {
                    name: ast::Ident::Str{
                        val: "n".to_string(),
                        span: Span{line: 0, col: 0},
                    },
                    expr: Expr::Literal(Literal::Int(1)),
                })
            ],
            expr: Box::new(Expr::Error),
            expr_type: Type::Unknown,
        })));
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn bad_block_expr_3() {
        // {
        //     let n = 1;
            
        //     n + 1
        //  // no end brace
        let kinds: Vec<TokenKind> = vec![LBrace, 
            Let, Ident("n".to_string()), Equals, IntLiteral(1), Semicolon,
            Ident("n".to_string()), Plus, IntLiteral(1),
            Eof];
        let tokens = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let result = Parser::new(tokens, &mut diagnostics).parse_expr(0);

        diagnostics.debug_print();

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn bad_match_1() {
        //     cases { // missing expression after cases
        //         a : 1,
        //         _ : 0,
        //     }
        // }
        let kinds: Vec<TokenKind> = vec![Cases, LBrace,
            Ident("a".to_string()), Colon, IntLiteral(1), Comma, 
            Underscore, Colon, IntLiteral(0), Comma,
            RBrace, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_expr(0);

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn bad_match_2() {
        //     cases a { 
        //         @ : 1, // unknown token
        //         _ : 0,
        //     }
        // }
        let kinds: Vec<TokenKind> = vec![Cases, Ident("a".to_string()), LBrace,
            ErrorToken, Colon, IntLiteral(1), Comma, 
            Underscore, Colon, IntLiteral(0), Comma,
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
        //     cases a { 
        //         : 1, // missing expr
        //         _ : 0,
        //     }
        // }
        let kinds: Vec<TokenKind> = vec![Cases, Ident("a".to_string()), LBrace,
            Colon, IntLiteral(1), Comma, 
            Underscore, Colon, IntLiteral(0), Comma,
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
        //     cases a { 
        //         b : , // missing expr
        //         _ : 0,
        //     }
        // }
        let kinds: Vec<TokenKind> = vec![Cases, Ident("a".to_string()), LBrace,
            Ident("b".to_string()), Colon, Comma, 
            Underscore, Colon, IntLiteral(0), Comma,
            RBrace, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_expr(0);

        diagnostics.debug_print();

        assert_eq!(result, Some(Expr::Cases(CasesExpr {
            scrutinee: Box::new(Expr::Ident(build_ident_str("a"))),
            arms: vec![
                CasesArm {
                    pattern: vec![SimplePattern::Ident(build_ident_str("b"))],
                    expr: Expr::Error,
                    arm_span: Span {line: 0, col: 0},
                },
                CasesArm {
                    pattern: vec![SimplePattern::Default],
                    expr: Expr::Literal(Literal::Int(0)),
                    arm_span: Span {line: 0, col: 0},
                }
            ],
            expr_type: Type::Unknown,
            span: Span {line: 0, col: 0},
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
        //     b : 1   // no comma
        //     _ : c
        // }
        let kinds: Vec<TokenKind> = vec![Sample, LBrace,
            Ident("b".to_string()), Colon, IntLiteral(1),
            Underscore, Colon, Ident("c".to_string()), 
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
        //     a : sample {, // No closing brace causes last } to be mistaken for the second samples closing.
        //     _ : c
        // }
        let kinds: Vec<TokenKind> = vec![Sample, LBrace,
            Ident("a".to_string()), Colon, Sample, LBrace, Comma,
            Underscore, Colon, Ident("c".to_string()), Comma,
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
        //     a : sample {
        //        a b        // no fat arrow
        //     },
        //     _ : c
        // }
        let kinds: Vec<TokenKind> = vec![Sample, LBrace,
            Ident("a".to_string()), Colon, Sample, LBrace, 
                Ident("a".to_string()), Ident("b".to_string()),
            RBrace, Comma,
            Underscore, Colon, Ident("c".to_string()), Comma,
            RBrace, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_expr(0);

        assert_eq!(result, Some(Expr::Sample(SampleExpr {
            arms: vec![
                SampleArm {
                    prob: Prob::Expr(Expr::Ident(build_ident_str("a"))),
                    expr: Expr::Error,
                    arm_span: Span {line: 0, col: 0},
                },
                SampleArm {
                    prob: Prob::Default,
                    expr: Expr::Ident(build_ident_str("c")),
                    arm_span: Span {line: 0, col: 0},
                },
            ],
            expr_type: Type::Unknown,
            span: Span {line: 0, col: 0},
        })));
        assert_eq!(diagnostics.num_errors(), 1);
    }
}