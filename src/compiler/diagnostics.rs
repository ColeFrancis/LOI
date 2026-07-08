//! # error
//!
//! Defines error types from the lexer, parser, and semantic analysis
//!
//! ## Invariants
//!
//! - All compiler errors will be found in here
//!
//! Author: Cole Francis
//!
//! Last Updated: 07/08/2026

use super::token::{Span, TokenKind};

pub struct Diagnostics {
    errors: Vec<CompilerError>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
        }
    }

    pub fn error(&mut self, error: CompilerError) {
        self.errors.push(error);
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn num_errors(&self) -> usize {
        self.errors.len()
    }

    pub fn errors(&self) -> &[CompilerError] {
        &self.errors
    }
}

#[derive(Debug)]
pub enum CompilerError {
    // Lexer
    UnknownToken {
        lexeme: String,
        span: Span,
    },

    InvalidNum {
        lexeme: String,
        span: Span,
    },

    // Parser
    UnexpectedToken {
        expected: Vec<Expected>,
        found: TokenKind,
        span: Span,
    },
}

#[derive(Debug)]
pub enum Expected {
    Token(TokenKind),
    Expr,
    Type,
    Ident,
}
