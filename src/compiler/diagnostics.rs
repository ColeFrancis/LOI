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

//! # diagnostics
//!
//! Defines error types from the lexer, parser, and semantic analysis
//!
//! ## Invariants
//!
//! - All compiler errors will be defined in the CompilerError enum
//!
//! Author: Cole Francis

use super::lexer::token::TokenKind;

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

    pub fn debug_print(&self) {
        println!("{} error(s):", self.errors.len());

        for (i, error) in self.errors.iter().enumerate() {
            println!("{}: {:#?}", i + 1, error);
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum CompilerError {
    ////////////////////
    // Lexer
    ////////////////////

    UnknownToken {
        lexeme: String,
        span: Span,
    },

    InvalidNum {
        lexeme: String,
        span: Span,
    },

    ////////////////////
    // Parser
    ////////////////////

    UnexpectedToken {
        expected: Vec<Expected>,
        found: TokenKind,
        span: Span,
    },

    ////////////////////
    // Semantic Analysis
    ////////////////////

    DuplicateDefinition {
        name: String,
        span: Span,
    }
}

#[derive(Debug, PartialEq)]
pub enum Expected {
    Token(TokenKind),
    Expr,
    Pattern,
    Ident,
    BoolLiteral,
    IntLiteral,
    RealLiteral,
}

#[derive(PartialEq, Debug, Clone)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer::Lexer;
    use crate::compiler::parser::Parser;

    #[test]
    fn no_errors() {
        let mut diagnostics = Diagnostics::new();
        let tokens = Lexer::new("
            ent_t COIN = {H, T};
        
            let a = 1;

            rel_t ONE : () -> Real = 1;

            net EMPTY {}
        ", &mut diagnostics).tokenize();

        Parser::new(tokens, &mut diagnostics).parse();

        assert!(!diagnostics.has_errors());
    }

    #[test]
    fn lexer_1() {
        let mut diagnostics = Diagnostics::new();
        Lexer::new(
        "@ 9a
        ", &mut diagnostics).tokenize();

        assert_eq!(diagnostics.errors, vec![
            CompilerError::UnknownToken{
                lexeme: "@".to_string(),
                span: Span {
                    line: 1,
                    col: 1
                }
            },
            CompilerError::InvalidNum{
                lexeme: "9a".to_string(),
                span: Span {
                    line: 1,
                    col: 3
                }
            },
        ]);
    }

    #[test]
    fn rel() {
        let mut diagnostics = Diagnostics::new();
        let tokens = Lexer::new(
        "rel_t A () -> Real a;
        ", &mut diagnostics).tokenize();

        Parser::new(tokens, &mut diagnostics).parse();

        assert_eq!(diagnostics.errors, vec![
            CompilerError::UnexpectedToken {
                expected: vec![Expected::Token(TokenKind::Colon)],
                found: TokenKind::LParen,
                span: Span {
                    line: 1,
                    col: 9
                }
            }
        ]);
    }

    #[test]
    fn expr() {
        let mut diagnostics = Diagnostics::new();
        let tokens = Lexer::new(
"let n = match a {
    let => 1,
};", &mut diagnostics).tokenize();

        Parser::new(tokens, &mut diagnostics).parse();

        assert_eq!(diagnostics.errors, vec![
            CompilerError::UnexpectedToken {
                expected: vec![Expected::Pattern],
                found: TokenKind::Let,
                span: Span {
                    line: 2,
                    col: 5,
                }
            }
        ]);
    }

    #[test]
    fn multiple_errors_1() {
        let mut diagnostics = Diagnostics::new();
        let tokens = Lexer::new(
"let n = 1;
n = 2;
let n = 3;
let 9n = 4;
let n = 5;
let n = 6
let n = 7;
let n = @;", &mut diagnostics).tokenize();

        Parser::new(tokens, &mut diagnostics).parse();

        assert_eq!(diagnostics.errors, vec![
            CompilerError::InvalidNum {
                lexeme: "9n".to_string(),
                span: Span {
                    line: 4,
                    col: 5,
                }
            },
            CompilerError::UnknownToken {
                lexeme: "@".to_string(),
                span: Span {
                    line: 8,
                    col: 9,
                }
            },
            CompilerError::UnexpectedToken {
                expected: vec![
                    Expected::Token(TokenKind::Let),
                    Expected::Token(TokenKind::Ent_t),
                    Expected::Token(TokenKind::Rel_t),
                    Expected::Token(TokenKind::NetToken),
                ],
                found: TokenKind::Ident("n".to_string()),
                span: Span {
                    line: 2,
                    col: 1,
                }
            },
            CompilerError::UnexpectedToken {
                expected: vec![Expected::Ident],
                found: TokenKind::ErrorToken,
                span: Span {
                    line: 4,
                    col: 5,
                }
            },
            CompilerError::UnexpectedToken {
                expected: vec![Expected::Token(TokenKind::Semicolon)],
                found: TokenKind::Let,
                span: Span {
                    line: 7,
                    col: 1,
                }
            },
            CompilerError::UnexpectedToken {
                expected: vec![Expected::Expr],
                found: TokenKind::ErrorToken,
                span: Span {
                    line: 8,
                    col: 9,
                }
            },
        ]);
    }
}
