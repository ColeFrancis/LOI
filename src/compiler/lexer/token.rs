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
    Ent_t,    // ent_t
    Rel_t,    // rel_t
    NetToken,    // net
    Cases,  // cases
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
    Ampersand,  // &
    Underscore, // _
    
    Equals,   // =
    Arrow,    // ->
    Connect,  // :=
    
    ErrorToken,

    Eof,
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::Ent_t    => write!(f, "\"ent_t\""),
            TokenKind::Rel_t    => write!(f, "\"rel_t\""),
            TokenKind::NetToken => write!(f, "\"net\""),
            TokenKind::Cases    => write!(f, "\"cases\""),
            TokenKind::Sample   => write!(f, "\"sample\""),

            TokenKind::Input  => write!(f, "\"input\""),
            TokenKind::Output => write!(f, "\"output\""),
            TokenKind::Init   => write!(f, "\"init\""),
            TokenKind::Let    => write!(f, "\"let\""),

            TokenKind::Bool    => write!(f, "\"Bool\""),
            TokenKind::Impulse => write!(f, "\"Impulse\""),
            TokenKind::Int     => write!(f, "\"Int\""),
            TokenKind::Real    => write!(f, "\"Real\""),
            TokenKind::Mod     => write!(f, "\"Mod\""),

            TokenKind::Ident(_)       => write!(f, "Identifier"),
            TokenKind::BoolLiteral(_) => write!(f, "Bool Literal"),
            TokenKind::IntLiteral(_)  => write!(f, "Int Literal"),
            TokenKind::RealLiteral(_) => write!(f, "Real Literal"),

            TokenKind::Colon     => write!(f, "\":\""),
            TokenKind::Semicolon => write!(f, "\";\""),
            TokenKind::Comma     => write!(f, "\",\""),
            TokenKind::Period    => write!(f, "\".\""),

            TokenKind::LParen => write!(f, "\"(\""),
            TokenKind::RParen => write!(f, "\")\""),
            TokenKind::LBrace => write!(f, "\"{{\""),
            TokenKind::RBrace => write!(f, "\"}}\""),

            TokenKind::Gt => write!(f, "\">\""),
            TokenKind::Lt => write!(f, "\"<\""),
            TokenKind::Ge => write!(f, "\">=\""),
            TokenKind::Le => write!(f, "\"<=\""),

            TokenKind::Plus      => write!(f, "\"+\""),
            TokenKind::Minus     => write!(f, "\"-\""),
            TokenKind::Asterisk  => write!(f, "\"*\""),
            TokenKind::Slash     => write!(f, "\"/\""),
            TokenKind::Caret     => write!(f, "\"^\""),
            TokenKind::BitNot    => write!(f, "\"~\""),
            TokenKind::Pipe      => write!(f, "\"|\""),
            TokenKind::Ampersand => write!(f, "\"&\""),

            TokenKind::Underscore => write!(f, "\"_\""),
            TokenKind::Equals     => write!(f, "\"=\""),
            TokenKind::Arrow      => write!(f, "\"->\""),
            TokenKind::Connect    => write!(f, "\":=\""),

            TokenKind::ErrorToken => write!(f, "Error Token"),
            TokenKind::Eof => write!(f, "End of File"),
        }
    }
}