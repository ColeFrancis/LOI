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
//! Handles relation parsing 
//!
//! ## Invariants
//!
//! - 
//!
//! Author: Cole Francis

use super::Parser;
use super::sync::SyncRule;
use super::ast::*;
use crate::compiler::lexer::token::TokenKind;

impl<'a> Parser<'a> {
    // Rel_t token already consumed
    pub(super) fn parse_rel_t(&mut self) -> Option<RelType> {
        let name = self.expect_ident(&SyncRule::Item)?;

        self.expect(TokenKind::Colon, &SyncRule::Item)?;

        self.expect(TokenKind::LParen, &SyncRule::Item)?;

        let mut params = Vec::new();

        while self.peek().kind != TokenKind::RParen {
            params.push(self.parse_param(&SyncRule::Item)?);

            if self.peek().kind == TokenKind::Comma {
                self.next();
            } else {
                break;
            }
        }

        self.expect(TokenKind::RParen, &SyncRule::Item)?;

        self.expect(TokenKind::Arrow, &SyncRule::Item)?;

        let return_type = self.parse_type(&SyncRule::Item)?;

        self.expect(TokenKind::Equals, &SyncRule::Item)?;

        // parse_block_expr returns Option, parse_expr always returns Expr even if its an Expr:Error
        let body = match self.peek().kind {
            TokenKind::LBrace => self.parse_block_expr().map(RelBody::Block),
            _ => Some(RelBody::Expr(match self.parse_expr(0) {
                Some(expr) => expr,
                None => Expr::Error,
            })),
        }?;

        self.expect(TokenKind::Semicolon, &SyncRule::Item)?;

        Some(RelType {
            name,
            params,
            return_type,
            body,
        })
    }

    // { not consumed
    fn parse_block_expr(&mut self) -> Option<BlockExpr> {
        self.expect(TokenKind::LBrace, &SyncRule::Item)?;

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

        Some(BlockExpr {
            statements,
            expr,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer::token::{Token, TokenKind::*};
    use crate::compiler::diagnostics::{Diagnostics, Span};
    use crate::compiler::parser::ast;
    
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

    #[test]
    fn rel_and() {
        // rel_t AND : (a:Bool, b:Bool) -> Bool = a*b;
        let kinds: Vec<TokenKind> = vec![Ident("AND".to_string()), Colon, LParen, 
            Ident("a".to_string()), Colon, Bool, Comma,
            Ident("b".to_string()), Colon, Bool,
            RParen, Arrow, Bool, Equals, 
            Ident("a".to_string()), Asterisk, Ident("b".to_string()),
            Semicolon, Eof
            ];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_rel_t();

        assert_eq!(result, Some(RelType {
            name: build_ident_str("AND"),
            params: vec![Param {
                name: build_ident_str("a"),
                param_type: Type::Bool,
            }, Param {
                name: build_ident_str("b"),
                param_type: Type::Bool,
            }],
            return_type: Type::Bool,
            body: RelBody::Expr(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Ident(build_ident_str("a"))),
                op: BinaryOp::Mul,
                right: Box::new(Expr::Ident(build_ident_str("b")))
            })),
        }));
    }

    #[test]
    fn rel_flip() {
        // rel_t FLIP : () -> Bool = {
        //     let p = 0.5;

        //     sample {
        //         p => true,
        //         _ => false,
        //     }
        // };
        let kinds: Vec<TokenKind> = vec![Ident("FLIP".to_string()), Colon, LParen, RParen, Arrow, Bool, Equals, LBrace,
            Let, Ident("p".to_string()), Equals, RealLiteral(0.5), Semicolon,
            Sample, LBrace, Ident("p".to_string()), FatArrow, BoolLiteral(true), Comma,
            Underscore, FatArrow, BoolLiteral(false), Comma, RBrace,
            RBrace, Semicolon, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_rel_t();

        assert_eq!(result, Some(RelType {
            name: build_ident_str("FLIP"),
            params: vec![],
            return_type: Type::Bool,
            body: RelBody::Block(BlockExpr {
                statements: vec![Statement::Let(LetStatement {
                    name: build_ident_str("p"),
                    expr: Expr::Literal(Literal::Real(0.5)),
                })],
                expr: Expr::Sample( vec![
                    SampleArm {
                        prob: Prob::Expr(Expr::Ident(build_ident_str("p"))),
                        expr: Expr::Literal(Literal::Bool(true)),
                    },
                    SampleArm {
                        prob: Prob::Default,
                        expr: Expr::Literal(Literal::Bool(false)),
                    },
                ]),
            }),
        }));
    }

    #[test]
    fn bad_rel_1() {
        // rel_t NUM  () -> Real = {  // missing colon
        //     let p = 0.5;
        //     let q = 0.4;
        
        //     p+q
        // };
        let kinds: Vec<TokenKind> = vec![Ident("NUM".to_string()), LParen, RParen, Arrow, Real, Equals, LBrace,
            Let, Ident("p".to_string()), Equals, RealLiteral(0.5), Semicolon,
            Let, Ident("q".to_string()), Equals, RealLiteral(0.4), Semicolon,
            Ident("p".to_string()), Plus, Ident("q".to_string()),// Minus,
            RBrace, Semicolon, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_rel_t();

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn bad_rel_2() {
        // rel_t NUM : () -> Real = {
        //     let p = 0.5     // missing semicolon
        //     let q = 0.4;
        
        //     p+q
        // };
        let kinds: Vec<TokenKind> = vec![Ident("NUM".to_string()), Colon, LParen, RParen, Arrow, Real, Equals, LBrace,
            Let, Ident("p".to_string()), Equals, RealLiteral(0.5),
            Let, Ident("q".to_string()), Equals, RealLiteral(0.4), Semicolon,
            Ident("p".to_string()), Plus, Ident("q".to_string()),
            RBrace, Semicolon, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_rel_t();

        diagnostics.debug_print();

        assert_eq!(result, Some(RelType {
            name: build_ident_str("NUM"),
            params: vec![],
            return_type: Type::Real,
            body: RelBody::Block(BlockExpr {
                statements: vec![
                    Statement::Error,
                    Statement::Let(LetStatement {
                        name: build_ident_str("q"),
                        expr: Expr::Literal(Literal::Real(0.4)),
                    })
                ],
                expr: Expr::Binary(BinaryExpr {
                    left: Box::new(Expr::Ident(build_ident_str("p"))),
                    op: BinaryOp::Add,
                    right: Box::new(Expr::Ident(build_ident_str("q")))
                }),
            }),
        }));
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn bad_rel_3() {
        // rel_t NUM : () -> Real = {
        //     let p = 0.5;
        //     let q = 0.4; 
        // }; // missing expr
        let kinds: Vec<TokenKind> = vec![Ident("NUM".to_string()), Colon, LParen, RParen, Arrow, Real, Equals, LBrace,
            Let, Ident("p".to_string()), Equals, RealLiteral(0.5), Semicolon,
            Let, Ident("q".to_string()), Equals, RealLiteral(0.4), Semicolon,
            RBrace, Semicolon, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_rel_t();

        diagnostics.debug_print();

        assert_eq!(result, Some(RelType {
            name: build_ident_str("NUM"),
            params: vec![],
            return_type: Type::Real,
            body: RelBody::Block(BlockExpr {
                statements: vec![
                    Statement::Let(LetStatement {
                        name: build_ident_str("p"),
                        expr: Expr::Literal(Literal::Real(0.5)),
                    }),
                    Statement::Let(LetStatement {
                        name: build_ident_str("q"),
                        expr: Expr::Literal(Literal::Real(0.4)),
                    }),
                ],
                expr: Expr::Error,
            }),
        }));
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn bad_rel_4() {
        // rel_t NUM : () -> Real = {
        //     let p = 0.5;
        //     let q = 0.4;
        
        //     p+q
        // // missing semicolon
        let kinds: Vec<TokenKind> = vec![Ident("NUM".to_string()), Colon, LParen, RParen, Arrow, Real, Equals, LBrace,
            Let, Ident("p".to_string()), Equals, RealLiteral(0.5), Semicolon,
            Let, Ident("q".to_string()), Equals, RealLiteral(0.4), Semicolon,
            Ident("p".to_string()), Plus, Ident("q".to_string()),// Minus,
            RBrace, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_rel_t();

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn bad_rel_5() {
        // rel_t NUM : () -> Real = {
        //     let p = 0.5;
        //     let q = 0.4;
        
        //     p+q
        // // missing closing brace
        let kinds: Vec<TokenKind> = vec![Ident("NUM".to_string()), Colon, LParen, RParen, Arrow, Real, Equals, LBrace,
            Let, Ident("p".to_string()), Equals, RealLiteral(0.5), Semicolon,
            Let, Ident("q".to_string()), Equals, RealLiteral(0.4), Semicolon,
            Ident("p".to_string()), Plus, Ident("q".to_string()),// Minus,
            Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_rel_t();

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }
}