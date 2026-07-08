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
//! Last Updated: 07/06/2026

use super::Parser;
use crate::compiler::token::TokenKind;
use crate::compiler::ast::*;

impl<'a> Parser<'a> {
    // Ent_t token already consumed
    pub(super) fn parse_ent_t(&mut self) -> EntType {
        let name = self.expect_ident_old();

        self.expect_old(TokenKind::Equals);

        let expr = self.parse_ent_expr();

        self.expect_old(TokenKind::Semicolon);

        EntType {
            name,
            expr
        }
    }

    fn parse_ent_expr(&mut self) -> EntExpr {
        match &self.peek().kind {
            TokenKind::LBrace => self.parse_set_ent(),
            _ => EntExpr::Type(self.parse_type()),
        }
    }

    fn parse_set_ent(&mut self) -> EntExpr {
        self.expect_old(TokenKind::LBrace);

        let mut members = vec![self.expect_ident_old()];

        while self.peek().kind == TokenKind::Comma {
            self.next();
            members.push(self.expect_ident_old());
        }

        self.expect_old(TokenKind::RBrace);

        EntExpr::SetEnt(members)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::token::{Token, TokenKind::*, Span};
    use crate::compiler::diagnostics::Diagnostics;
    
    fn build_token_vec(tokens: Vec<TokenKind>) -> Vec<Token> {
        tokens
            .into_iter()
            .map(|x| Token {kind: x, span: Span{line: 0, col: 0}})
            .collect()
    }

    #[test]
    fn test_set_ent() {
        // ent_t coin = {H, T};
        let kinds: Vec<TokenKind> = vec![Ident("coin".to_string()), Equals, 
            LBrace, Ident("H".to_string()), Comma, Ident("T".to_string()),
            RBrace, Semicolon, Eof];

        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_ent_t();

        assert_eq!(result, EntType {
            name: "coin".to_string(),
            expr: EntExpr::SetEnt(vec!["H".to_string(), "T".to_string()]),
        });
    }

    #[test]
    fn test_mod_ent() {
        // ent_t z4 = Mod(4);
        let kinds: Vec<TokenKind> = vec![Ident("z4".to_string()), Equals, 
            Mod, LParen, IntLiteral(4), RParen, Semicolon, Eof];

        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_ent_t();

        assert_eq!(result, EntType {
            name: "z4".to_string(),
            expr: EntExpr::Type(Type::Mod(4)),
        });
    }
}