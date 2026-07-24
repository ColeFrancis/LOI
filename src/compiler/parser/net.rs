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
//! Handles handles network 
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
    lexer::token::TokenKind,
    diagnostics::{CompilerError, Expected},
};

impl<'a> Parser<'a> {
    // Net token already consumed
    pub(super) fn parse_net(&mut self) -> Option<Net> {
        let name = self.expect_ident(&SyncRule::Item)?;

        self.expect(TokenKind::LBrace, &SyncRule::Item)?;

        let mut items = Vec::new();

        while self.peek().kind != TokenKind::RBrace {
            items.push(match self.parse_net_item() {
                Some(item) => item,
                None => NetItem::Error,
            });
        }

        self.expect(TokenKind::RBrace, &SyncRule::Item)?;

        Some(Net {
            name,
            items,
        })
    }

    fn parse_net_item(&mut self) -> Option<NetItem> {
        let token = &self.peek();

        match &token.kind {
            TokenKind::Input => {
                self.next();

                let item = NetItem::Input(self.parse_param(&SyncRule::NetItem {depth: 0})?);

                self.expect(TokenKind::Semicolon, &SyncRule::NetItem {depth: 0})?;

                Some(item)
            },

            TokenKind::Output => {
                self.next();

                let item = NetItem::Output(self.parse_param(&SyncRule::NetItem {depth: 0})?);

                self.expect(TokenKind::Semicolon, &SyncRule::NetItem {depth: 0})?;

                Some(item)
            },

            TokenKind::Init => {
                self.next();
                Some(NetItem::Init(self.parse_init_ent()?))
            },

            TokenKind::Ident(_) => match self.peek_n(1).kind {
                TokenKind::Connect => Some(NetItem::RelInst(self.parse_rel_inst()?)),
                _ => Some(NetItem::NetInst(self.parse_net_inst()?)),
            },

            other => {
                self.diagnostics.error(CompilerError::UnexpectedToken {
                    expected: vec![
                        Expected::Token(TokenKind::Input),
                        Expected::Token(TokenKind::Output),
                        Expected::Token(TokenKind::Init),
                        Expected::Ident,
                    ],
                    found: other.clone(),
                    span: token.span.clone(),
                });

                self.sync(&SyncRule::NetItem {depth: 0});

                None
            } 
        }
    }

    fn parse_init_ent(&mut self) -> Option<EntInit> {
        let param = self.parse_param(&SyncRule::NetItem {depth: 0})?;

        self.expect(TokenKind::Equals, &SyncRule::NetItem {depth: 0})?;

        let val = match self.parse_expr(0) {
            Some(expr) => expr,
            None => Expr::Error,
        };

        self.expect(TokenKind::Semicolon, &SyncRule::NetItem {depth: 0})?;

        Some(EntInit {
            param,
            val,
        })
    }

    fn parse_rel_inst(&mut self) -> Option<RelInst> {
        let asignee = self.expect_ident(&SyncRule::NetItem {depth: 0})?;

        self.expect(TokenKind::Connect, &SyncRule::NetItem {depth: 0})?;

        let rel = self.expect_ident(&SyncRule::NetItem {depth: 0})?;

        self.expect(TokenKind::LParen, &SyncRule::NetItem {depth: 0})?;

        let mut args = Vec::new();

        while self.peek().kind != TokenKind::RParen {
            args.push(self.expect_ident(&SyncRule::NetItem {depth: 0})?);

            if self.peek().kind == TokenKind::Comma {
                self.next();
            } else {
                break;
            }
        }

        self.expect(TokenKind::RParen, &SyncRule::NetItem {depth: 0})?;

        self.expect(TokenKind::Semicolon, &SyncRule::NetItem {depth: 0})?;

        Some(RelInst {
            asignee,
            rel,
            args,
        })
    }

    fn parse_net_inst(&mut self) -> Option<NetInst> {
        let net = self.expect_ident(&SyncRule::NetItem {depth: 0})?;

        self.expect(TokenKind::LBrace, &SyncRule::NetItem {depth: 0})?;

        let mut connections = Vec::new();

        while self.peek().kind != TokenKind::RBrace {
            connections.push(self.parse_connection()?);

            if self.peek().kind == TokenKind::Comma {
                self.next();
            } else {
                break;
            }
        }

        self.expect(TokenKind::RBrace, &SyncRule::NetItem {depth: 0});

        self.expect(TokenKind::Semicolon, &SyncRule::NetItem {depth: 0})?;

        Some(NetInst {
            net,
            connections,
        })
    }

    fn parse_connection(&mut self) -> Option<Connection> {
        let port = self.expect_ident(&SyncRule::NetItem {depth: 1})?;

        self.expect(TokenKind::Connect, &SyncRule::NetItem {depth: 1})?;

        let net = self.expect_ident(&SyncRule::NetItem {depth: 1})?;

        Some(Connection {
            port,
            net,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer::token::{Token, TokenKind::*};
    use crate::compiler::diagnostics::{Diagnostics, Span};
    use crate::compiler::parser::ast;
    
    fn build_token_vec(tokens: Vec<TokenKind>) -> Vec<Token> {
        tokens
            .into_iter()
            .map(|x| Token {kind: x, span: Span{line: 0, col: 0}})
            .collect()
    }

    fn build_ident_str(name: &str) -> ast::Ident {
        ast::Ident::Str {
            val: name.to_string(),
            span: Span{line: 0, col: 0},
        }
    }

    #[test]
    fn net_add() {
        // net ADD {
        //     input a: Bool;
        //     input b: Bool;

        //     output sum: Bool;
        //     output cout: Bool;

        //     input cin: Bool = false;

        //     HALF_ADD {
        //         a := a,
        //         b := b,
        //         sum := h1_sum,
        //         cout := h1_carry,
        //     };

        //     HALF_ADD {
        //         a := h1_sum,
        //         b := cin,
        //         sum := sum,
        //         cout := h2_carry,
        //     };

        //     cout := OR(h1_carry, h2_carry);

        // }
        let kinds: Vec<TokenKind> = vec![
            Ident("ADD".to_string()), LBrace,
                Input, Ident("a".to_string()), Colon, Bool, Semicolon,
                Input, Ident("b".to_string()), Colon, Bool, Semicolon,
                Output, Ident("sum".to_string()), Colon, Bool, Semicolon,
                Output, Ident("cout".to_string()), Colon, Bool, Semicolon,
                Init, Ident("cin".to_string()), Colon, Bool, Equals, BoolLiteral(false), Semicolon,
                
                Ident("HALF_ADD".to_string()), LBrace,
                    Ident("a".to_string()), Connect, Ident("a".to_string()), Comma,
                    Ident("b".to_string()), Connect, Ident("b".to_string()), Comma,
                    Ident("sum".to_string()), Connect, Ident("h1_sum".to_string()), Comma,
                    Ident("cout".to_string()), Connect, Ident("h1_carry".to_string()), Comma,
                RBrace, Semicolon,

                Ident("HALF_ADD".to_string()), LBrace,
                    Ident("a".to_string()), Connect, Ident("h1_sum".to_string()), Comma,
                    Ident("b".to_string()), Connect, Ident("cin".to_string()), Comma,
                    Ident("sum".to_string()), Connect, Ident("sum".to_string()), Comma,
                    Ident("cout".to_string()), Connect, Ident("h2_carry".to_string()), Comma,
                RBrace, Semicolon,

                Ident("cout".to_string()), Connect, Ident("OR".to_string()), LParen,
                    Ident("h1_carry".to_string()), Comma, Ident("h2_carry".to_string()),
                RParen, Semicolon,
            RBrace, Eof
            ];

        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_net();

        assert_eq!(result, Some(Net {
            name: build_ident_str("ADD"),
            items: vec![
                NetItem::Input(Param {
                    name: build_ident_str("a"),
                    param_type: Type::Bool,
                }),
                NetItem::Input(Param {
                    name: build_ident_str("b"),
                    param_type: Type::Bool,
                }),
                NetItem::Output(Param {
                    name: build_ident_str("sum"),
                    param_type: Type::Bool,
                }),
                NetItem::Output(Param {
                    name: build_ident_str("cout"),
                    param_type: Type::Bool,
                }),
                NetItem::Init(EntInit {
                    param: Param {
                        name: build_ident_str("cin"),
                        param_type: Type::Bool,
                    },
                    val: Expr::Literal(Literal::Bool(false)),
                }),
                NetItem::NetInst(NetInst {
                    net: build_ident_str("HALF_ADD"),
                    connections: vec![
                        Connection {
                            port: build_ident_str("a"),
                            net: build_ident_str("a"),
                        },
                        Connection {
                            port: build_ident_str("b"),
                            net: build_ident_str("b"),
                        },
                        Connection {
                            port: build_ident_str("sum"),
                            net: build_ident_str("h1_sum"),
                        },
                        Connection {
                            port: build_ident_str("cout"),
                            net: build_ident_str("h1_carry"),
                        },
                    ],
                }),
                NetItem::NetInst(NetInst {
                    net: build_ident_str("HALF_ADD"),
                    connections: vec![
                        Connection {
                            port: build_ident_str("a"),
                            net: build_ident_str("h1_sum"),
                        },
                        Connection {
                            port: build_ident_str("b"),
                            net: build_ident_str("cin"),
                        },
                        Connection {
                            port: build_ident_str("sum"),
                            net: build_ident_str("sum"),
                        },
                        Connection {
                            port: build_ident_str("cout"),
                            net: build_ident_str("h2_carry"),
                        },
                    ],
                }),
                NetItem::RelInst(RelInst {
                    asignee: build_ident_str("cout"),
                    rel: build_ident_str("OR"),
                    args: vec![
                        build_ident_str("h1_carry"),
                        build_ident_str("h2_carry"),
                    ],
                })
            ],
        }));
    }

    #[test]
    fn net_empty() {
        // net EMPTY {}
        let kinds: Vec<TokenKind> = vec![
            Ident("EMPTY".to_string()), LBrace, RBrace, Eof];

        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_net();

        assert_eq!(result, Some(Net {
            name: build_ident_str("EMPTY"),
            items: vec![],
        }));
    }

    #[test]
    fn net_bad_1() {
        // net ADD {
        //     input a: Bool;
        //     input b Bool; // missing colon

        //     output sum: Bool;
        // }
        let kinds: Vec<TokenKind> = vec![
            Ident("ADD".to_string()), LBrace,
                Input, Ident("a".to_string()), Colon, Bool, Semicolon,
                Input, Ident("b".to_string()), Bool, Semicolon,
                Output, Ident("sum".to_string()), Colon, Bool, Semicolon,
            RBrace, Eof
            ];

        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_net();

        assert_eq!(result, Some(Net {
            name: build_ident_str("ADD"),
            items: vec![
                NetItem::Input(Param {
                    name: build_ident_str("a"),
                    param_type: Type::Bool,
                }),
                NetItem::Error,
                NetItem::Output(Param {
                    name: build_ident_str("sum"),
                    param_type: Type::Bool,
                }),
            ],
        }));
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn net_bad_2() {
        // net ADD {
        //     input a: ; // missing bool
        //     input b : Bool; 

        //     sum: Bool; // missing output
        // }
        let kinds: Vec<TokenKind> = vec![
            Ident("ADD".to_string()), LBrace,
                Input, Ident("a".to_string()), Colon, Semicolon,
                Input, Ident("b".to_string()), Colon, Bool, Semicolon,
                Ident("sum".to_string()), Colon, Bool, Semicolon,
            RBrace, Eof
            ];

        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_net();

        assert_eq!(result, Some(Net {
            name: build_ident_str("ADD"),
            items: vec![
                NetItem::Error,
                NetItem::Input(Param {
                    name: build_ident_str("b"),
                    param_type: Type::Bool,
                }),
                NetItem::Error,
            ],
        }));
        assert_eq!(diagnostics.num_errors(), 2);
    }

    #[test]
    fn net_bad_3() {
        // net ADD {
        //     input a: Bool;
        //     input b: Bool // missing semicolon

        //     output sum: Bool;
        // }
        let kinds: Vec<TokenKind> = vec![
            Ident("ADD".to_string()), LBrace,
                Input, Ident("a".to_string()), Colon, Bool, Semicolon,
                Input, Ident("b".to_string()), Colon, Bool,
                Output, Ident("sum".to_string()), Colon, Bool, Semicolon,
            RBrace, Eof
            ];

        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_net();

        assert_eq!(result, Some(Net {
            name: build_ident_str("ADD"),
            items: vec![
                NetItem::Input(Param {
                    name: build_ident_str("a"),
                    param_type: Type::Bool,
                }),
                NetItem::Error,
                NetItem::Output(Param {
                    name: build_ident_str("sum"),
                    param_type: Type::Bool,
                }),
            ],
        }));
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn net_bad_4() {
        // net A {
        //     B {
        //         c  d,   // missing connect
        //         e := f,
        //     };
        // }
        let kinds: Vec<TokenKind> = vec![
            Ident("A".to_string()), LBrace,
                Ident("B".to_string()), LBrace,
                    Ident("c".to_string()), Ident("d".to_string()), Comma,
                    Ident("e".to_string()), Connect, Ident("f".to_string()), Comma,
                RBrace, Semicolon,
            RBrace, Eof
            ];

        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_net();

        assert_eq!(result, Some(Net {
            name: build_ident_str("A"),
            items: vec![
                NetItem::Error,
            ],
        }));
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn net_bad_5() {
        // net A {
        //     B {
        //         c :=,   // missing ident
        //         e := f,
        //     };
        // }
        let kinds: Vec<TokenKind> = vec![
            Ident("A".to_string()), LBrace,
                Ident("B".to_string()), LBrace,
                    Ident("c".to_string()), Connect, Comma,
                    Ident("e".to_string()), Connect, Ident("f".to_string()), Comma,
                RBrace, Semicolon,
            RBrace, Eof
            ];

        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_net();

        assert_eq!(result, Some(Net {
            name: build_ident_str("A"),
            items: vec![
                NetItem::Error,
            ],
        }));
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn net_bad_6() {
        // net A {
        //     B {
        //         c := d,   
        //         e := f,
        //     } // Missing semicolon
        // }
        let kinds: Vec<TokenKind> = vec![
            Ident("A".to_string()), LBrace,
                Ident("B".to_string()), LBrace,
                    Ident("c".to_string()), Connect, Ident("d".to_string()), Comma,
                    Ident("e".to_string()), Connect, Ident("f".to_string()), Comma,
                RBrace,
            RBrace, Eof
            ];

        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_net();

        assert_eq!(result, Some(Net {
            name: build_ident_str("A"),
            items: vec![
                NetItem::Error,
            ],
        }));
        assert_eq!(diagnostics.num_errors(), 1);
    }

    #[test]
    fn net_bad_7() {
        // net ADD {
        //     cout  OR(h1_carry, h2_carry); // missing connect
        // }
        let kinds: Vec<TokenKind> = vec![
            Ident("ADD".to_string()), LBrace,
                Ident("cout".to_string()), Ident("OR".to_string()), LParen,
                    Ident("h1_carry".to_string()), Comma, Ident("h2_carry".to_string()),
                RParen, Semicolon,
            RBrace, Eof
            ];

        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_net();

        assert_eq!(result, Some(Net {
            name: build_ident_str("ADD"),
            items: vec![
                NetItem::Error
            ],
        }));
        assert_eq!(diagnostics.num_errors(), 1);
    }
}