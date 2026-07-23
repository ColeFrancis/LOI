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
//! Lexer takes a string slice (the entire code) and becomes an iterator over tokens 
//!
//! ## Invariants
//!
//! - operations must be binary (for now)
//! - Lexer returns none upon Eof
//! - All errors in parsing potential tokens result in an Invalid token
//! - All tokens must be parsable
//!
//! Author: Cole Francis

use super::Lexer;
use super::token::{Token, TokenKind};
use crate::compiler::diagnostics::{Diagnostics, CompilerError, Span};

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str, diagnostics: &'a mut Diagnostics) -> Self {
        Self {
            input: source.as_bytes(),
            pos: 0,
            curr_line: 1,
            curr_col: 0,
            done: false,
            diagnostics,
        }
    }

    fn next(&mut self) -> Option<u8> {
        let c = self.input.get(self.pos).copied();
        if c.is_some() {
            self.pos += 1;
            self.curr_col += 1;
        }
        c
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    pub fn tokenize(mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        while let Some(token) = self.next_token() {
            tokens.push(token);
        }

        tokens
    }

    fn next_token(&mut self) -> Option<Token> {
        while let Some(c) = self.next() {
            let line = self.curr_line;
            let col = self.curr_col; 
            match c {
                // Whitespace
                b' ' | b'\t' | b'\r' => continue,

                b'\n' => {
                    self.curr_line += 1;
                    self.curr_col = 0;
                    continue
                }

                // Comments
                b'/' => {
                    if let Some(token) = self.handle_slash() {
                        return Some(Token::new(token, line, col));
                    }
                    continue;
                }

                // Keywords and Identifiers
                b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                    let kind = self.handle_letter_underscore(c);

                    return Some(Token::new(kind, line, col));
                }

                // Literals
                b'0'..=b'9' => {
                    let kind = self.handle_number(c, line, col);

                    return Some(Token::new(kind, line, col));
                }

                // Punctuation
                b':' => {
                    if self.peek() == Some(b'=') {
                        self.next();
                        return Some(Token::new(TokenKind::Connect, line, col));
                    } else {
                        return Some(Token::new(TokenKind::Colon, line, col));
                    }
                }
                b';' => return Some(Token::new(TokenKind::Semicolon, line, col)),
                b',' => return Some(Token::new(TokenKind::Comma, line, col)),
                b'.' => return Some(Token::new(TokenKind::Period, line, col)),

                b'(' => return Some(Token::new(TokenKind::LParen, line, col)),
                b')' => return Some(Token::new(TokenKind::RParen, line, col)),
                b'{' => return Some(Token::new(TokenKind::LBrace, line, col)),
                b'}' => return Some(Token::new(TokenKind::RBrace, line, col)),

                // Operators
                b'>' => {
                    if self.peek() == Some(b'=') {
                        self.next();
                        return Some(Token::new(TokenKind::Ge, line, col));
                    } else {
                        return Some(Token::new(TokenKind::Gt, line, col));
                    }
                }
                b'<' => {
                    if self.peek() == Some(b'=') {
                        self.next();
                        return Some(Token::new(TokenKind::Le, line, col));
                    } else {
                        return Some(Token::new(TokenKind::Gt, line, col));
                    }
                }
                b'-' => {
                    if self.peek() == Some(b'>') {
                        self.next();
                        return Some(Token::new(TokenKind::Arrow, line, col));
                    } else {
                        return Some(Token::new(TokenKind::Minus, line, col));
                    }
                }
                b'=' => {
                    if self.peek() == Some(b'>') {
                        self.next();
                        return Some(Token::new(TokenKind::FatArrow, line, col));
                    } else {
                        return Some(Token::new(TokenKind::Equals, line, col));
                    }
                }
                b'+' => return Some(Token::new(TokenKind::Plus, line, col)),
                b'*' => return Some(Token::new(TokenKind::Asterisk, line, col)),
                b'^' => return Some(Token::new(TokenKind::Caret, line, col)),
                b'~' => return Some(Token::new(TokenKind::BitNot, line, col)),
                b'|' => return Some(Token::new(TokenKind::Pipe, line, col)),

                // Unknown
                _ => {
                    self.diagnostics.error(CompilerError::UnknownToken {
                        lexeme: (c as char).to_string(),
                        span: Span {line, col},
                    });
                    return Some(Token::new(TokenKind::ErrorToken, line, col))
                }
            }
        }

        if self.done {
            None
        } else {
            self.done = true;
            Some(Token::new(TokenKind::Eof, self.curr_line, self.curr_col))
        }
    }

    // Comments, slash token
    fn handle_slash(&mut self) -> Option<TokenKind> {
        match self.peek() {
            // Single line comment
            Some(b'/') => {
                self.next();

                while let Some(c) = self.next() {
                    if c == b'\n' {
                        break;
                    }
                }

                return None;
            }
            // Multi-line comment
            Some(b'*') => {
                self.next();

                while let Some(c) = self.next() {
                    if c == b'*' && self.peek() == Some(b'/') {
                        self.next();
                        return None;
                    }
                }

                return None;
            }
            // Slash token
            _ => {
                return Some(TokenKind::Slash);
            }
        }
    }

    // Keywords, Identifiers
    fn handle_letter_underscore(&mut self, first: u8) -> TokenKind  {
        let mut buf = String::new();
        buf.push(first as char);

        while let Some(c) = self.peek() {
            match c {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' => {
                    buf.push(c as char);
                    self.next();
                }
                _ => break,
            }
        }

        match buf.as_str() {
            "ent_t"   => TokenKind::Ent_t,
            "rel_t"   => TokenKind::Rel_t,
            "net"     => TokenKind::NetToken,
            "match"   => TokenKind::Match,
            "sample"  => TokenKind::Sample,
            "input"   => TokenKind::Input,
            "output"  => TokenKind::Output,
            "init"    => TokenKind::Init,
            "let"     => TokenKind::Let,
            "Bool"    => TokenKind::Bool,
            "Impulse" => TokenKind::Impulse,
            "Real"    => TokenKind::Real,
            "Int"     => TokenKind::Int,
            "Mod"     => TokenKind::Mod,
            "true"    => TokenKind::BoolLiteral(true),
            "false"   => TokenKind::BoolLiteral(false),
            "_"       => TokenKind::Underscore,
            _         => TokenKind::Ident(buf),
        }
    }

    // Literals
    fn handle_number(&mut self, first: u8, line: usize, col: usize) -> TokenKind {
        let mut buf = String::new();
        buf.push(first as char);

        let mut is_valid = true;
        let mut is_float = false;

        while let Some(c) = self.peek() {
            match c {
                b'0'..=b'9' => {
                    buf.push(c as char);
                    self.next();
                }

                b'.' if !is_float => {
                    is_float = true;
                    buf.push('.');
                    self.next();
                }

                b'.' if is_float => {
                    is_valid = false;
                    buf.push('.');
                    self.next();
                }

                // Underscores are skipped in numbers
                b'_' => {
                    self.next();
                }

                b'a'..=b'z' | b'A'..=b'Z' => {
                    is_valid = false;
                    buf.push(c as char);
                    self.next();
                }

                _ => break,
            }
        }

        if !is_valid {
            self.diagnostics.error(CompilerError::InvalidNum {
                lexeme: buf.clone(),
                span: Span {line, col},
            });
            return TokenKind::ErrorToken;
        } else if is_float {
            match buf.parse::<f64>() {
                Ok(n) => TokenKind::RealLiteral(n),
                Err(_) => {
                    self.diagnostics.error(CompilerError::InvalidNum {
                        lexeme: buf.clone(),
                        span: Span {line, col},
                    });
                    return TokenKind::ErrorToken;
                }
            }
        } else {
            match buf.parse::<i64>() {
                Ok(n) => TokenKind::IntLiteral(n),
                Err(_) => {
                    self.diagnostics.error(CompilerError::InvalidNum {
                        lexeme: buf.clone(),
                        span: Span {line, col},
                    });
                    return TokenKind::ErrorToken;
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::compiler::lexer::token::TokenKind::*;

    fn kinds(tokens: &[Token]) -> Vec<TokenKind> {
        tokens.iter().map(|t| t.kind.clone()).collect()
    }

    #[test]
    fn test_comments() {
        let mut diagnostics = Diagnostics::new();
        let mut lexer = Lexer::new("// asdklf;jsk \n   ", &mut diagnostics);

        let tokens: Vec<Token> = lexer.tokenize();

        assert_eq!(kinds(&tokens), vec![Eof]);

        let mut diagnostics = Diagnostics::new();
        let tokens = Lexer::new("   /* jalsdjf\nasjflds*/", &mut diagnostics).tokenize();

        assert_eq!(kinds(&tokens), vec![Eof]);
    }

    #[test]
    fn test_not_alphanumeric() {
        let mut diagnostics = Diagnostics::new();
        let tokens = Lexer::new("( /*asdjf*/ ) =>\n;", &mut diagnostics).tokenize();

        assert_eq!(kinds(&tokens), vec![LParen, RParen, FatArrow, Semicolon, Eof]);
    }

    #[test]
    fn test_ent() {
        let mut diagnostics = Diagnostics::new();
        let tokens = Lexer::new("ent_t COIN = {H,T}; // This is an entity\n", &mut diagnostics).tokenize();

        assert_eq!(kinds(&tokens), vec![Ent_t, Ident("COIN".to_string())
            , Equals, LBrace, Ident("H".to_string()), Comma
            , Ident("T".to_string()), RBrace, Semicolon, Eof]);
        assert!(!diagnostics.has_errors());
    } 

    #[test]
    fn test_unknown() {
        let mut diagnostics = Diagnostics::new();
        let tokens = Lexer::new("@", &mut diagnostics).tokenize();

        assert_eq!(kinds(&tokens), vec![ErrorToken, Eof]);
        assert!(diagnostics.has_errors());
    }

    #[test]
    fn test_num() {
        let mut diagnostics = Diagnostics::new();
        let tokens = Lexer::new("94f 9.9.9 10_000_000_000_000_000_000 99 9.8 1_000", &mut diagnostics).tokenize();

        assert_eq!(kinds(&tokens), vec![ErrorToken, ErrorToken
            , ErrorToken, IntLiteral(99)
            , RealLiteral(9.8), IntLiteral(1000), Eof]);
        assert!(diagnostics.has_errors());
    }

    #[test]
    fn test_identifiers() {
        let mut diagnostics = Diagnostics::new();
        let tokens = Lexer::new("id ai_ _ai", &mut diagnostics).tokenize();

        assert_eq!(kinds(&tokens), vec![Ident("id".to_string())
            , Ident("ai_".to_string()), Ident("_ai".to_string()), Eof]);
    }

    #[test]
    fn test_rel() {
        let mut diagnostics = Diagnostics::new();
        let tokens = Lexer::new("rel_t A : (a:Real) -> Real = (a / 2);", &mut diagnostics).tokenize();

        assert_eq!(kinds(&tokens), vec![Rel_t, Ident("A".to_string()), Colon, LParen
        , Ident("a".to_string()), Colon, Real, RParen, Arrow, Real
        , Equals, LParen, Ident("a".to_string()), Slash, IntLiteral(2)
        , RParen, Semicolon, Eof]);
    }

    #[test]
    fn test_line_col() {
        let mut diagnostics = Diagnostics::new();
        let tokens = Lexer::new(" a\nbc\td_ 67\n  / /* */;", &mut diagnostics).tokenize();

        assert_eq!(tokens[0].span.line, 1);
        assert_eq!(tokens[0].span.col, 2);
        
        assert_eq!(tokens[1].span.line, 2);
        assert_eq!(tokens[1].span.col, 1);
        
        assert_eq!(tokens[2].span.line, 2);
        assert_eq!(tokens[2].span.col, 4);
        
        assert_eq!(tokens[3].span.line, 2);
        assert_eq!(tokens[3].span.col, 7);

        assert_eq!(tokens[4].span.line, 3);
        assert_eq!(tokens[4].span.col, 3);

        assert_eq!(tokens[5].span.line, 3);
        assert_eq!(tokens[5].span.col, 10);
    }
}