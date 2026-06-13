//! # lexer
//!
//! Lexer takes a string slice (the entire code) and becomes an iterator over tokens 
//!
//! ## Invariants
//!
//! - operations must be binary (for now)
//! - Lexer returns none upon EOF
//! - All errors in parsing potential tokens result in an Invalid token
//!
//! Author: Cole Francis
//!
//! Last Updated: 06/13/2026

use super::token::Token;

pub struct Lexer<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            input: source.as_bytes(),
            pos: 0,
        }
    }

    fn _next(&mut self) -> Option<u8> {
        let c = self.input.get(self.pos).copied();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn _peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn next_token(&mut self) -> Option<Token> {
        while let Some(c) = self._next() {
            match c {
                // Whitespace
                b' ' | b'\n' | b'\r' => continue,

                // Comments
                b'/' => {
                    if let Some(token) = self._handle_slash() {
                        return Some(token);
                    }
                    continue;
                }

                // Keywords and Identifiers
                b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                    return Some(self._handle_letter_underscore(c));
                }

                // Literals
                b'0'..=b'9' => {
                    return Some(self._handle_number(c));
                }

                // Punctuation
                b':' => return Some(Token::Colon),
                b';' => return Some(Token::Semicolon),
                b',' => return Some(Token::Comma),
                b'.' => return Some(Token::Period),

                b'(' => return Some(Token::LParen),
                b')' => return Some(Token::RParen),
                b'{' => return Some(Token::LBrace),
                b'}' => return Some(Token::RBrace),

                // Operators
                b'-' => {
                    if self._peek() == Some(b'>') {
                        self._next();
                        return Some(Token::Arrow);
                    } else {
                        return Some(Token::Minus);
                    }
                }
                b'=' => {
                    if self._peek() == Some(b'>') {
                        self._next();
                        return Some(Token::FatArrow);
                    } else {
                        return Some(Token::Equals);
                    }
                }
                b'+' => return Some(Token::Plus),
                b'*' => return Some(Token::Asterisk),

                // Unknown
                _ => return Some(Token::Unknown(c as char)),
            }
        }

        None
    }

    // Comments, slash token
    fn _handle_slash(&mut self) -> Option<Token> {
        match self._peek() {
            // Single line comment
            Some(b'/') => {
                self._next();

                while let Some(c) = self._next() {
                    if c == b'\n' {
                        break;
                    }
                }

                return None;
            }
            // Multi-line comment
            Some(b'*') => {
                self._next();

                while let Some(c) = self._next() {
                    if c == b'*' && self._peek() == Some(b'/') {
                        self._next();
                        return None;
                    }
                }

                return None;
            }
            // Slash token
            _ => {
                return Some(Token::Slash);
            }
        }
    }

    // Keywords, Identifiers
    // TODO: correctly handle i (complex number) by itself
    fn _handle_letter_underscore(&mut self, first: u8) -> Token  {
        let mut buf = String::new();
        buf.push(first as char);

        while let Some(c) = self._peek() {
            match c {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' => {
                    buf.push(c as char);
                    self._next(); // consume
                }
                _ => break,
            }
        }

        match buf.as_str() {
            "ent"    => Token::Ent,
            "rel"    => Token::Rel,
            "net"    => Token::Net,
            "match"  => Token::Match,
            "sample" => Token::Sample,
            "Real"   => Token::Real,
            "Int"    => Token::Int,
            "Cmp"    => Token::Cmp,
            "i"      => Token::I,
            _ => Token::Identifier(buf),
        }
    }

    // Literals
    fn _handle_number(&mut self, first: u8) -> Token {
        let mut buf = String::new();
        buf.push(first as char);

        let mut is_valid = true;
        let mut is_float = false;

        while let Some(c) = self._peek() {
            match c {
                b'0'..=b'9' => {
                    buf.push(c as char);
                    self._next(); // consume
                }

                b'.' if !is_float => {
                    is_float = true;
                    buf.push('.');
                    self._next();
                }

                b'.' if is_float => {
                    is_valid = false;
                    buf.push('.');
                    self._next();
                }

                // Underscores are skipped in numbers
                b'_' => {
                    self._next(); // consume
                }

                // i is a keyword for imaginary number
                //  Therefore, 9i is valid and is broken into two tokens
                b'i' => break,

                b'a'..=b'z' | b'A'..=b'Z' => {
                    is_valid = false;
                    buf.push(c as char);
                    self._next();
                }

                _ => break,
            }
        }

        if !is_valid {
            Token::InvalidNum(buf)
        } else if is_float {
            match buf.parse::<f64>() {
                Ok(n) => Token::RealLiteral(n),
                Err(_) => Token::InvalidNum(buf),
            }
        } else {
            match buf.parse::<i64>() {
                Ok(n) => Token::IntLiteral(n),
                Err(_) => Token::InvalidNum(buf),
            }
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_comments() {
        let mut lexer = Lexer::new("// asdklf;jsk \n   ");

        assert_eq!(lexer.next(), None);

        let mut lexer = Lexer::new("   /* jalsdjf\nasjflds*/");

        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn test_not_alphanumeric() {
        let lexer = Lexer::new("( /*asdjf*/ ) =>\n;");

        let tokens: Vec<Token> = lexer.collect();

        assert_eq!(tokens, vec![Token::LParen,Token::RParen, Token::FatArrow,Token::Semicolon]);
    }

    #[test]
    fn test_ent() {
        let lexer = Lexer::new("ent COIN = {H,T}; // This is an entity\n");

        let tokens: Vec<Token> = lexer.collect();

        assert_eq!(tokens, vec![Token::Ent,Token::Identifier("COIN".to_string())
            ,Token::Equals,Token::LBrace,Token::Identifier("H".to_string()),Token::Comma
            ,Token::Identifier("T".to_string()),Token::RBrace,Token::Semicolon]);
    } 

    #[test]
    fn test_unknown() {
        let lexer = Lexer::new("@");

        let tokens: Vec<Token> = lexer.collect();

        assert_eq!(tokens, vec![Token::Unknown('@')]);
    }

    #[test]
    fn test_num() {
        let lexer = Lexer::new("94f 9.9.9 10_000_000_000_000_000_000 99i 99 9.8");

        let tokens: Vec<Token> = lexer.collect();

        assert_eq!(tokens, vec![Token::InvalidNum("94f".to_string()),Token::InvalidNum("9.9.9".to_string())
            ,Token::InvalidNum("10000000000000000000".to_string()), Token::IntLiteral(99),Token::I
            ,Token::IntLiteral(99), Token::RealLiteral(9.8)]);
    }

    #[test]
    fn test_identifiers() {
        let lexer = Lexer::new("id i ai_ _ai");

        let tokens: Vec<Token> = lexer.collect();

        assert_eq!(tokens, vec![Token::Identifier("id".to_string()),Token::I
            ,Token::Identifier("ai_".to_string()),Token::Identifier("_ai".to_string())]);
    }
}