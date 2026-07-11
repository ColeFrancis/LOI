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
//!
//! Last Updated: 07/03/2026

use super::diagnostics::Span;

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