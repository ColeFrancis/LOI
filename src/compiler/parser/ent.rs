//! # core
//!
//! Handles entity parsing 
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
    // Ent_t token already consumed
    pub(super) fn parse_ent_t(&mut self) -> Option<EntType> {
        let name = self.expect_ident()?;

        self.expect(TokenKind::Equals)?;

        let expr = self.parse_ent_expr()?;

        self.expect(TokenKind::Semicolon)?;

        Some(EntType {
            name,
            expr
        })
    }

    fn parse_ent_expr(&mut self) -> Option<EntExpr> {
        match &self.peek().kind {
            TokenKind::LBrace => self.parse_set_ent(),
            _ => self.parse_type().map(EntExpr::Type),
        }
    }

    fn parse_set_ent(&mut self) -> Option<EntExpr> {
        self.expect(TokenKind::LBrace)?;

        let mut members = vec![self.expect_ident()?];

        while self.peek().kind == TokenKind::Comma {
            self.next();
            members.push(self.expect_ident()?);
        }

        self.expect(TokenKind::RBrace)?;

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
        assert!(diagnostics.has_errors());
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
    fn bad_mod_ent() {
        // ent_t z4 = mod(4);
        let kinds: Vec<TokenKind> = vec![Ident("z4".to_string()), Equals, 
            Ident("mod".to_string()), LParen, IntLiteral(4), RParen, Semicolon, Eof];

        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_ent_t();

        assert_eq!(result, None);
    }
}