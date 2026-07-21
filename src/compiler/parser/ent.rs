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
//! Handles entity parsing 
//!
//! ## Invariants
//!
//! - 
//!
//! Author: Cole Francis

use super::Parser;
use super::sync::SyncRule;
use crate::compiler::token::TokenKind;
use crate::compiler::ast::*;

impl<'a> Parser<'a> {
    // Ent_t token already consumed
    pub(super) fn parse_ent_t(&mut self) -> Option<EntType> {
        let name = self.expect_ident(&SyncRule::Item)?;

        self.expect(TokenKind::Equals, &SyncRule::Item)?;

        let expr = self.parse_ent_expr()?;

        self.expect(TokenKind::Semicolon, &SyncRule::Item)?;

        Some(EntType {
            name,
            expr
        })
    }

    fn parse_ent_expr(&mut self) -> Option<EntExpr> {
        match &self.peek().kind {
            TokenKind::LBrace => self.parse_set_ent(),
            _ => self.parse_type(&SyncRule::Item).map(EntExpr::Type),
        }
    }

    fn parse_set_ent(&mut self) -> Option<EntExpr> {
        self.expect(TokenKind::LBrace, &SyncRule::Item)?;

        let mut members = vec![self.expect_ident(&SyncRule::Item)?];

        while self.peek().kind == TokenKind::Comma {
            self.next();
            members.push(self.expect_ident(&SyncRule::Item)?);
        }

        self.expect(TokenKind::RBrace, &SyncRule::Item)?;

        Some(EntExpr::SetEnt(members))
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
    fn set_ent() {
        // ent_t coin = {H, T};
        let kinds: Vec<TokenKind> = vec![Ident("coin".to_string()), Equals, 
            LBrace, Ident("H".to_string()), Comma, Ident("T".to_string()),
            RBrace, Semicolon, Eof];

        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_ent_t();

        assert_eq!(result, Some(EntType {
            name: "coin".to_string(),
            expr: EntExpr::SetEnt(vec!["H".to_string(), "T".to_string()]),
        }));
    }

    #[test]
    fn bad_set_ent() {
        // ent_t coin = {H, T;
        let kinds: Vec<TokenKind> = vec![Ident("coin".to_string()), Equals, 
            LBrace, Ident("H".to_string()), Comma, Ident("T".to_string()),
            Semicolon, Eof];

        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_ent_t();

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn mod_ent() {
        // ent_t z4 = Mod(4); 
        let kinds: Vec<TokenKind> = vec![Ident("z4".to_string()), Equals, 
            Mod, LParen, IntLiteral(4), RParen, Semicolon, Eof];

        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_ent_t();

        assert_eq!(result, Some(EntType {
            name: "z4".to_string(),
            expr: EntExpr::Type(Type::Mod(4)),
        }));
    }

    #[test]
    fn bad_mod_ent_1() {
        // ent_t z4 = mod(4);    (should be Mod not mod)
        let kinds: Vec<TokenKind> = vec![Ident("z4".to_string()), Equals, 
            Ident("mod".to_string()), LParen, IntLiteral(4), RParen, Semicolon, Eof];

        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_ent_t();

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn bad_mod_ent_2() {
        // ent_t z4 = Mod(4;    (missing ")")
        let kinds: Vec<TokenKind> = vec![Ident("z4".to_string()), Equals, 
            Mod, LParen, IntLiteral(4), Semicolon, Eof];

        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_ent_t();

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn bad_mod_ent_3() {
        // ent_t z4 Mod(4;    (missing "=", ")")
        let kinds: Vec<TokenKind> = vec![Ident("z4".to_string()), 
            Mod, LParen, IntLiteral(4), Semicolon, Eof];

        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_ent_t();

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn bad_real_ent() {
        // ent_t r = real;    real, not Real
        let kinds: Vec<TokenKind> = vec![Ident("r".to_string()), 
            Equals, Ident("real".to_string()), Eof];

        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_ent_t();

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1);
    }
}