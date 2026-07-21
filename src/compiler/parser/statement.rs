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
use super::ast::*;
use crate::compiler::lexer::token::TokenKind;

impl<'a> Parser<'a> {
    // Let token already consumed
    pub(super) fn parse_let_stmt(&mut self) -> Option<LetStatement> {
        let name = self.expect_ident(&SyncRule::Statement)?;

        self.expect(TokenKind::Equals, &SyncRule::Statement)?;

        let expr = match self.parse_expr(0) {
            Some(expr) => expr,
            None => Expr::Error,
        };

        self.expect(TokenKind::Semicolon, &SyncRule::Statement)?;

        Some(LetStatement {
            name,
            expr,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer::{Lexer, token::{Token, TokenKind::*}};
    use crate::compiler::diagnostics::{Diagnostics, Span};

    fn build_token_vec(tokens: Vec<TokenKind>) -> Vec<Token> {
        tokens
            .into_iter()
            .map(|x| Token {kind: x, span: Span{line: 0, col: 0}})
            .collect()
    }

    #[test]
    fn let_statement() {
        // let n = 1 + 2;
        let kinds: Vec<TokenKind> = vec![Ident("n".to_string()), Equals, IntLiteral(1), Plus, IntLiteral(2), Semicolon, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_let_stmt();

        assert_eq!(result, Some(LetStatement {
            name: "n".to_string(),
            expr: Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Literal(Literal::Int(1))),
                op: BinaryOp::Add,
                right: Box::new(Expr::Literal(Literal::Int(2))),
            })
        }));
        assert!(!diagnostics.has_errors());
    }

    #[test] 
    fn bad_let_statement() {
        // let n = 1 + 2
        let kinds: Vec<TokenKind> = vec![Ident("n".to_string()), Equals, IntLiteral(1), Plus, IntLiteral(2), Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_let_stmt();

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1); // unexpected token
        
        // let 9n = 1;
        let mut diagnostics = Diagnostics::new();
        let mut lexer = Lexer::new("9a = 1;", &mut diagnostics);
        let tokens: Vec<Token> = lexer.tokenize();

        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_let_stmt();

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 2); // invalid num, unexpected token
    }
}