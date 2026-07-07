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
//! Last Updated: 07/06/2026

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

    pub fn errors(&self) -> &[CompilerError] {
        &self.errors
    }
}

#[derive(Debug)]
pub enum CompilerError {
    UnknownToken(String),

    InvalidNum(String),
}