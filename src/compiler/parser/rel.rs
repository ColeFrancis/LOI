//! # core
//!
//! Handles relation parsing 
//!
//! ## Invariants
//!
//! - 
//!
//! Author: Cole Francis
//!
//! Last Updated: 07/10/2026

use super::Parser;
use crate::compiler::token::TokenKind;
use crate::compiler::ast::*;

impl<'a> Parser<'a> {
    // Rel_t token already consumed
    pub(super) fn parse_rel_t(&mut self) -> Option<RelType> {
        let name = self.expect_ident()?;

        self.expect(TokenKind::Colon)?;

        self.expect(TokenKind::LParen)?;

        let mut params = Vec::new();

        while self.peek().kind != TokenKind::RParen {
            params.push(self.parse_param()?);

            if self.peek().kind == TokenKind::Comma {
                self.next();
            } else {
                break;
            }
        }

        self.expect(TokenKind::RParen)?;

        self.expect(TokenKind::Arrow)?;

        let return_type = self.parse_type()?;

        self.expect(TokenKind::Equals)?;

        // parse_block_expr returns Option, parse_expr always returns Expr even if its an Expr:Error
        let body = match self.peek().kind {
            TokenKind::LBrace => self.parse_block_expr().map(RelBody::Block),
            _ => Some(RelBody::Expr(match self.parse_expr(0) {
                Some(expr) => expr,
                None => Expr::Error,
            })),
        }?;

        self.expect(TokenKind::Semicolon)?;

        Some(RelType {
            name,
            params,
            return_type,
            body,
        })
    }

    // { not consumed
    fn parse_block_expr(&mut self) -> Option<BlockExpr> {
        self.expect(TokenKind::LBrace)?;

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

        self.expect(TokenKind::RBrace)?;

        Some(BlockExpr {
            statements,
            expr,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::token::{Token, TokenKind::*};
    use crate::compiler::diagnostics::{Diagnostics, Span};
    
    fn build_token_vec(tokens: Vec<TokenKind>) -> Vec<Token> {
        tokens
            .into_iter()
            .map(|x| Token {kind: x, span: Span{line: 0, col: 0}})
            .collect()
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
            name: "AND".to_string(),
            params: vec![Param {
                name: "a".to_string(),
                param_type: Type::Bool,
            }, Param {
                name: "b".to_string(),
                param_type: Type::Bool,
            }],
            return_type: Type::Bool,
            body: RelBody::Expr(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Ident("a".to_string())),
                op: BinaryOp::Mul,
                right: Box::new(Expr::Ident("b".to_string()))
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
            name: "FLIP".to_string(),
            params: vec![],
            return_type: Type::Bool,
            body: RelBody::Block(BlockExpr {
                statements: vec![Statement::Let(LetStatement {
                    name: "p".to_string(),
                    expr: Expr::Literal(Literal::Real(0.5)),
                })],
                expr: Expr::Sample( vec![
                    SampleArm {
                        prob: Prob::Expr(Expr::Ident("p".to_string())),
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

    // #[test]
    // fn bad_rel_flip() {
    //     // rel_t NUM : () -> Real = {
    //     //     let p = 0.5
    //     //     let q = 0.4;
        
    //     //     p+q
    //     // };
    //     let kinds: Vec<TokenKind> = vec![Ident("FLIP".to_string()), Colon, LParen, RParen, Arrow, Bool, Equals, LBrace,
    //         Let, Ident("p".to_string()), Equals, RealLiteral(0.5), Semicolon,
    //         Sample, LBrace, Ident("p".to_string()), FatArrow, BoolLiteral(true), Comma,
    //         Underscore, FatArrow, BoolLiteral(false), Comma, RBrace,
    //         RBrace, Semicolon, Eof];
    //     let tokens: Vec<Token> = build_token_vec(kinds);

    //     let mut diagnostics = Diagnostics::new();
    //     let mut parser = Parser::new(tokens, &mut diagnostics);

    //     let result = parser.parse_rel_t();

    //     assert_eq!(result, Some(RelType {
    //         name: "NUM".to_string(),
    //         params: vec![],
    //         return_type: Type::Real,
    //         body: RelBody::Block(BlockExpr {
    //             statements: vec![
    //                 Statement::Error,
    //                 Statement::Let(LetStatement {
    //                     name: "q".to_string(),
    //                     expr: Expr::Literal(Literal::Real(0.4)),
    //                 })
    //             ],
    //             expr: Expr::Error,
    //         }),
    //     }));
    // }
}