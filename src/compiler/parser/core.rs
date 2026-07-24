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
//! Handles the core of the parser
//!
//! ## Invariants
//!
//! - 
//!
//! Author: Cole Francis

use super::Parser;
use super::sync::SyncRule;
use super::ast::*;
use crate::compiler::{
    lexer::token::{Token, TokenKind},
    diagnostics::{Diagnostics, CompilerError, Expected},
};


impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token>, diagnostics: &'a mut Diagnostics) -> Self {
        Self {
            tokens,
            current: 0,
            diagnostics,
        }
    }

    pub fn parse(mut self) -> Program {
        let mut items = Vec::new();

        while self.peek().kind != TokenKind::Eof {
            let token = self.next();

            let item = match token.kind {
                TokenKind::Let => match self.parse_let_stmt() {
                    Some(stmt) => Item::Let(stmt),
                    None => Item::Error,
                },

                TokenKind::Ent_t => match self.parse_ent_t() {
                    Some(ent) => Item::Ent(ent),
                    None => Item::Error,
                },

                TokenKind::Rel_t => match self.parse_rel_t() {
                    Some(rel) => Item::Rel(rel),
                    None => Item::Error,
                },

                TokenKind::NetToken => match self.parse_net() {
                    Some(net) => Item::Net(net),
                    None => Item::Error,
                }

                other => {
                    self.diagnostics.error(CompilerError::UnexpectedToken {
                        expected: vec![
                            Expected::Token(TokenKind::Let), 
                            Expected::Token(TokenKind::Ent_t), 
                            Expected::Token(TokenKind::Rel_t), 
                            Expected::Token(TokenKind::NetToken)
                        ],
                        found: other,
                        span: token.span,
                    });

                    self.sync(&SyncRule::Item);

                    Item::Error
                },
            };

            items.push(item);
        }

        Program { items }
    }

    pub(super) fn peek (&self) -> &Token {
        &self.tokens[self.current]
    }

    pub(super) fn peek_n (&self, offset: usize) -> &Token {
        &self.tokens[self.current + offset]
    }

    pub(super) fn next(&mut self) -> Token {
        let token = self.tokens[self.current].clone();
        self.current += 1;
        token
    }

    pub(super) fn expect(&mut self, expected: TokenKind, rule: &SyncRule)-> Option<()> {
        let token = self.peek();

        if token.kind == expected {
            self.next();
            Some(())
        } else if token.kind == TokenKind::Eof {
            self.diagnostics.error(CompilerError::UnexpectedToken {
                expected: vec![Expected::Token(expected)],
                found: TokenKind::Eof,
                span: token.span.clone(),
            });
            None
        } else {
            self.diagnostics.error(CompilerError::UnexpectedToken {
                expected: vec![Expected::Token(expected)],
                found: token.kind.clone(),
                span: token.span.clone(),
            });
            self.sync(rule);
            None
        }
    }

    pub(super) fn expect_ident(&mut self, rule: &SyncRule) -> Option<String> {
        let token = self.peek();

        match token.kind.clone() {
            TokenKind::Ident(name) => {
                self.next();
                Some(name)
            },
            TokenKind::Eof => {
                self.diagnostics.error(CompilerError::UnexpectedToken {
                    expected: vec![Expected::Ident],
                    found: TokenKind::Eof,
                    span: token.span.clone(),
                });
                None
            }
            other => {
                self.diagnostics.error(CompilerError::UnexpectedToken {
                    expected: vec![Expected::Ident],
                    found: other,
                    span: token.span.clone(),
                });
                self.sync(rule);
                None
            }
        }
    }
    
    pub(super) fn parse_type(&mut self, rule: &SyncRule) -> Option<Type> {
        let token = self.next();
        
        match token.kind {
            TokenKind::Bool        => Some(Type::Bool),
            TokenKind::Impulse     => Some(Type::Impulse),
            TokenKind::Int         => Some(Type::Int),
            TokenKind::Real        => Some(Type::Real),
            TokenKind::Ident(name) => Some(Type::CustomType(name)),
            
            other => {
                self.diagnostics.error(CompilerError::UnexpectedToken {
                    expected: vec![
                        Expected::Token(TokenKind::Bool), 
                        Expected::Token(TokenKind::Impulse), 
                        Expected::Token(TokenKind::Int), 
                        Expected::Token(TokenKind::Real),
                        Expected::Ident,
                    ],
                    found: other,
                    span: token.span,
                });
                self.sync(rule);
                    
                None
            }
        }
    }

    pub(super) fn parse_param(&mut self, rule: &SyncRule) -> Option<Param> {
        let name = self.expect_ident(rule)?;

        self.expect(TokenKind::Colon, rule)?;

        let param_type = self.parse_type(rule)?;

        Some(Param {
            name,
            param_type,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer::{Lexer, token::TokenKind::*};
    use crate::compiler::diagnostics::Span;
    
    fn build_token_vec(tokens: Vec<TokenKind>) -> Vec<Token> {
        tokens
            .into_iter()
            .map(|x| Token {kind: x, span: Span{line: 0, col: 0}})
            .collect()
    }

    #[test]
    fn test_parse() {
        // ent_t COIN = {H, T};
        
        // let a = 1;

        // rel_t ONE : () -> Real = 1;

        // net EMPTY {}
        let kinds: Vec<TokenKind> = vec![Ent_t, Ident("COIN".to_string()), Equals, LBrace, Ident("H".to_string()), Comma, Ident("T".to_string()), RBrace, Semicolon,
            Let, Ident("a".to_string()), Equals, IntLiteral(1), Semicolon,
            Rel_t, Ident("ONE".to_string()), Colon, LParen, RParen, Arrow, Real, Equals, IntLiteral(1), Semicolon,
            NetToken, Ident("EMPTY".to_string()), LBrace, RBrace, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let result = Parser::new(tokens, &mut diagnostics).parse();

        assert_eq!(result, Program { 
            items: vec![
                Item::Ent(EntType {
                    name: "COIN".to_string(),
                    expr: EntExpr::SetEnt(vec!["H".to_string(), "T".to_string()]),
                }),
                Item::Let(LetStatement {
                    name: "a".to_string(),
                    expr: Expr::Literal(Literal::Int(1)),
                }),
                Item::Rel(RelType {
                    name: "ONE".to_string(),
                    params: vec![],
                    return_type: Type::Real,
                    body: RelBody::Expr(Expr::Literal(Literal::Int(1))),
                }),
                Item::Net(Net {
                    name: "EMPTY".to_string(),
                    items: vec![],
                }),
            ],
        });
        assert!(!diagnostics.has_errors());
    }

    #[test]
    fn integrate_lexer_parser() {
        let mut diagnostics = Diagnostics::new();
        let tokens = Lexer::new("
            ent_t COIN = {H, T};
        
            let a = 1;

            rel_t ONE : () -> Real = 1;

            net EMPTY {}
        ", &mut diagnostics).tokenize();

        let result = Parser::new(tokens, &mut diagnostics).parse();

        assert_eq!(result, Program { 
            items: vec![
                Item::Ent(EntType {
                    name: "COIN".to_string(),
                    expr: EntExpr::SetEnt(vec!["H".to_string(), "T".to_string()]),
                }),
                Item::Let(LetStatement {
                    name: "a".to_string(),
                    expr: Expr::Literal(Literal::Int(1)),
                }),
                Item::Rel(RelType {
                    name: "ONE".to_string(),
                    params: vec![],
                    return_type: Type::Real,
                    body: RelBody::Expr(Expr::Literal(Literal::Int(1))),
                }),
                Item::Net(Net {
                    name: "EMPTY".to_string(),
                    items: vec![],
                }),
            ],
        });
        assert!(!diagnostics.has_errors());
    }

    #[test]
    fn multiple_errors_1() {
        let mut diagnostics = Diagnostics::new();
        let tokens = Lexer::new("
            let n = 1;
            n = 2;
            let n = 3;
            let 9n = 4;
            let n = 5;
            let n = 6
            let n = 7;
            let n = @;
        ", &mut diagnostics).tokenize();

        let result = Parser::new(tokens, &mut diagnostics).parse();

        assert_eq!(result, Program {
            items: vec![
                Item::Let(LetStatement {
                    name: "n".to_string(),
                    expr: Expr::Literal(Literal::Int(1)),
                }),
                Item::Error,
                Item::Let(LetStatement {
                    name: "n".to_string(),
                    expr: Expr::Literal(Literal::Int(3)),
                }),
                Item::Error,
                Item::Let(LetStatement {
                    name: "n".to_string(),
                    expr: Expr::Literal(Literal::Int(5)),
                }),
                Item::Error,
                Item::Let(LetStatement {
                    name: "n".to_string(),
                    expr: Expr::Literal(Literal::Int(7)),
                }),
                Item::Let(LetStatement {
                    name: "n".to_string(),
                    expr: Expr::Error,
                }),
            ]
        });
        assert_eq!(diagnostics.num_errors(), 6);
    }

    #[test]
    fn multiple_errors_2() {
        let mut diagnostics = Diagnostics::new();
        let tokens = Lexer::new("
            rel_t A () -> Real = a;

            net {
            
            }
            net A {
                input A: Bool
            }
        ", &mut diagnostics).tokenize();

        let result = Parser::new(tokens, &mut diagnostics).parse();

        assert_eq!(result, Program {
            items: vec![
                Item::Error,
                Item::Error,
                Item::Net(Net {
                    name: "A".to_string(),
                    items: vec![NetItem::Error],
                }),
            ]
        });
        assert_eq!(diagnostics.num_errors(), 3);
    }
}
