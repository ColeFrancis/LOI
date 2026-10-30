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

//! # statement
//!
//! Handles statement parsing
//!
//! ## Invariants
//!
//! -
//!
//! Author: Cole Francis

use super::Parser;
use super::sync::SyncRule;
use crate::compiler::{lexer::token::TokenKind, objects::ast::*, error_handling::diagnostics::{Diagnostic, Expected}};

impl<'a> Parser<'a> {
    // Let token already consumed
    pub(super) fn parse_stmt(&mut self) -> Option<Statement> {
        let name = self.expect_ident(&SyncRule::Statement)?;

        let token = self.next();
        let stmt = match token.kind {
            TokenKind::Equals => Statement::Var(self.parse_variable(name)?),
            TokenKind::LParen => Statement::Inline(self.parse_inline(name)?),

            _ => {
                self.diagnostics.error(Diagnostic::UnexpectedToken {
                    expected: vec![Expected::Token(TokenKind::Equals), Expected::Token(TokenKind::LParen)],
                    found: token.kind,
                    span: token.span.clone(),
                });
                return None;
            }
        };

        Some(stmt)
    }

    // "=" already consumed
    fn parse_variable(&mut self, name: Ident) -> Option<Variable> {
        let expr = match self.parse_expr(0) {
            Some(expr) => expr,
            None => Expr::Error,
        };

        self.expect(TokenKind::Semicolon, &SyncRule::Statement)?;

        Some(Variable { name, expr })
    }

    // "("" already consumed
    fn parse_inline(&mut self, name: Ident) -> Option<Inline> {
        let mut args = Vec::new();

        while self.peek().kind != TokenKind::RParen {
            args.push(self.expect_ident(&SyncRule::Statement)?);

            if self.peek().kind == TokenKind::Comma {
                self.next();
            } else {
                break;
            }
        }

        self.expect(TokenKind::RParen, &SyncRule::Statement)?;

        self.expect(TokenKind::Equals, &SyncRule::Statement)?;

        let expr = match self.parse_expr(0) {
            Some(expr) => expr,
            None => Expr::Error,
        };

        self.expect(TokenKind::Semicolon, &SyncRule::Statement)?;

        Some(Inline { name, args, expr})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::error_handling::diagnostics::Diagnostics;
    use crate::compiler::error_handling::span::Span;
    use crate::compiler::lexer::{
        Lexer,
        token::{Token, TokenKind::*},
    };
    use crate::compiler::objects::ast;
    use crate::compiler::objects::types::Type;

    fn build_token_vec(tokens: Vec<TokenKind>) -> Vec<Token> {
        tokens
            .into_iter()
            .map(|x| Token {
                kind: x,
                span: Span { line: 0, col: 0 },
            })
            .collect()
    }

    fn build_ident_str(name: &str) -> ast::Ident {
        ast::Ident::Str {
            val: name.to_string(),
            span: Span { line: 0, col: 0 },
        }
    }

    #[test]
    fn variable() {
        // let n = 1 + 2;
        let kinds: Vec<TokenKind> = vec![
            Ident("n".to_string()),
            Equals,
            IntLiteral(1),
            Plus,
            IntLiteral(2),
            Semicolon,
            Eof,
        ];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_stmt();

        assert_eq!(
            result,
            Some(Statement::Var(Variable {
                name: build_ident_str("n"),
                expr: Expr::Binary(BinaryExpr {
                    left: Box::new(Expr::Literal(Literal::Int(1))),
                    op: BinaryOp::Add,
                    right: Box::new(Expr::Literal(Literal::Int(2))),
                    op_span: Span { line: 0, col: 0 },
                    expr_type: Type::Unknown,
                })
            }))
        );
        assert!(!diagnostics.has_errors());
    }

    #[test]
    fn bad_variable() {
        // let n = 1 + 2
        let kinds: Vec<TokenKind> = vec![
            Ident("n".to_string()),
            Equals,
            IntLiteral(1),
            Plus,
            IntLiteral(2),
            Eof,
        ];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_stmt();

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1); // unexpected token

        // let 9n = 1;
        let mut diagnostics = Diagnostics::new();
        let tokens = Lexer::new("9a = 1;", &mut diagnostics).tokenize();

        let result = Parser::new(tokens, &mut diagnostics).parse_stmt();

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 2); // invalid num, unexpected token
    }

    #[test]
    fn inline() {
        // let add(a, b) = a+b;
        let kinds: Vec<TokenKind> = vec![
            Ident("add".to_string()),
            LParen,
            Ident("a".to_string()),
            Comma,
            Ident("b".to_string()),
            RParen,
            Equals,
            Ident("a".to_string()),
            Plus,
            Ident("b".to_string()),
            Semicolon,
            Eof,
        ];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_stmt();

        assert_eq!(
            result,
            Some(Statement::Inline(Inline {
                name: build_ident_str("add"),
                args: vec![
                    build_ident_str("a"),
                    build_ident_str("b"),
                ],
                expr: Expr::Binary(BinaryExpr {
                    left: Box::new(Expr::Ident(build_ident_str("a"))),
                    op: BinaryOp::Add,
                    right: Box::new(Expr::Ident(build_ident_str("b"))),
                    op_span: Span { line: 0, col: 0 },
                    expr_type: Type::Unknown,
                })
            }))
        );
        assert!(!diagnostics.has_errors());
    }
}
