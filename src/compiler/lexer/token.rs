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

//! # token
//!
//! Contains an enum of all tokens
//!
//! ## Invariants
//!
//! - Keywords are their own variants
//! - Tokens must obey and impelment grammar
//!
//! Author: Cole Francis

use crate::compiler::diagnostics::Span;

#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize, col: usize) -> Self {
        Self {
            kind: kind,
            span: Span {
                line,
                col,
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
#[allow(non_camel_case_types)]
pub enum TokenKind {
    // Keywords
    Ent_t,    // net_t
    Rel_t,    // rel_t
    NetToken,    // net
    Match,  // match
    Sample, // sample

    Input,  // input 
    Output, // output 
    Init,   // init 
    Let,    // let

    Bool,    // Bool
    Impulse, // Impulse
    Int,     // Int
    Real,    // Real
    Mod,     // Mod

    Ident(String),
    BoolLiteral(bool),
    IntLiteral(i64),
    RealLiteral(f64),

    // Punctuation
    Colon,     // :
    Semicolon, // ;
    Comma,     // ,
    Period,    // .

    LParen, // (
    RParen, // )
    LBrace, // {
    RBrace, // }

    Gt, // >
    Lt, // <
    Ge, // >=
    Le, // <=

    Plus,     // +
    Minus,    // -
    Asterisk, // *
    Slash,    // /
    Caret,    // ^
    
    BitNot,   // ~

    Pipe,       // |
    Underscore, // _
    
    Equals,   // =
    Arrow,    // ->
    FatArrow, // =>
    Connect,  // :=
    
    ErrorToken,

    Eof,
}