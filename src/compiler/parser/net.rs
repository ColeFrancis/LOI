//! # core
//!
//! Handles handles network 
//!
//! ## Invariants
//!
//! - 
//!
//! Author: Cole Francis
//!
//! Last Updated: 07/10/2026

use super::Parser;
use super::sync::SyncRule;
use crate::compiler::{
    token::TokenKind,
    ast::*,
    diagnostics::{CompilerError, Expected},
};

impl<'a> Parser<'a> {
    // Net token already consumed
    pub(super) fn parse_net(&mut self) -> Option<Net> {
        let name = self.expect_ident()?;

        self.expect(TokenKind::LBrace)?;

        let mut items = Vec::new();

        while self.peek().kind != TokenKind::RBrace {
            items.push(self.parse_net_item());
        }

        self.expect(TokenKind::RBrace)?;

        Some(Net {
            name,
            items,
        })
    }

    fn parse_net_item(&mut self) -> NetItem {
        let token = &self.peek();

        match &token.kind {
            TokenKind::Input => {
                self.next();

                // If either parse_param or expect return None, then get NetItem::Error
                match (self.parse_param(), self.expect(TokenKind::Semicolon)) {
                    (Some(param), Some(_)) => NetItem::Input(param),
                    _ => NetItem::Error,
                }
            },

            TokenKind::Output => {
                self.next();

                // If either parse_param or expect return None, then get NetItem::Error
                match (self.parse_param(), self.expect(TokenKind::Semicolon)) {
                    (Some(param), Some(_)) => NetItem::Output(param),
                    _ => NetItem::Error,
                }
            },

            TokenKind::Init => {
                self.next();
                match self.parse_init_ent() {
                    Some(init) => NetItem::Init(init),
                    None => NetItem::Error,
                }
            },

            TokenKind::Ident(_) => match self.peek_n(1).kind {
                TokenKind::Connect => match self.parse_rel_inst() {
                    Some(inst) => NetItem::RelInst(inst),
                    None => NetItem::Error,
                },

                _ => match self.parse_net_inst() {
                    Some(inst) => NetItem::NetInst(inst),
                    None => NetItem::Error,
                }
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

                self.sync(SyncRule::Inst);

                NetItem::Error
            } 
        }
    }

    fn parse_init_ent(&mut self) -> Option<EntInit> {
        let param = self.parse_param()?;

        self.expect(TokenKind::Equals)?;

        let val = match self.parse_expr(0) {
            Some(expr) => expr,
            None => Expr::Error,
        };

        self.expect(TokenKind::Semicolon)?;

        Some(EntInit {
            param,
            val,
        })
    }

    fn parse_rel_inst(&mut self) -> Option<RelInst> {
        let asignee = self.expect_ident()?;

        self.expect(TokenKind::Connect)?;

        let rel = self.expect_ident()?;

        self.expect(TokenKind::LParen)?;

        let mut args = Vec::new();

        while self.peek().kind != TokenKind::RParen {
            args.push(self.expect_ident()?);

            if self.peek().kind == TokenKind::Comma {
                self.next();
            } else {
                break;
            }
        }

        self.expect(TokenKind::RParen)?;

        self.expect(TokenKind::Semicolon)?;

        Some(RelInst {
            asignee,
            rel,
            args,
        })
    }

    fn parse_net_inst(&mut self) -> Option<NetInst> {
        let net = self.expect_ident()?;

        self.expect(TokenKind::LBrace)?;

        let mut connections = Vec::new();

        while self.peek().kind != TokenKind::RBrace {
            connections.push(self.parse_connection()?);

            if self.peek().kind == TokenKind::Comma {
                self.next();
            } else {
                break;
            }
        }

        self.expect(TokenKind::RBrace);

        self.expect(TokenKind::Semicolon)?;

        Some(NetInst {
            net,
            connections,
        })
    }

    fn parse_connection(&mut self) -> Option<Connection> {
        let port = self.expect_ident()?;

        self.expect(TokenKind::Connect)?;

        let net = self.expect_ident()?;

        Some(Connection {
            port,
            net,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::token::{Token, TokenKind::*};
    use crate::compiler::diagnostics::{Diagnostics, Span};
    
    fn build_token_vec(tokens: Vec<TokenKind>) -> Vec<Token> {
        tokens
            .into_iter()
            .map(|x| Token {kind: x, span: Span{line: 0, col: 0}})
            .collect()
    }

    #[test]
    fn test_net_t() {
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
            name: "ADD".to_string(),
            items: vec![
                NetItem::Input(Param {
                    name: "a".to_string(),
                    param_type: Type::Bool,
                }),
                NetItem::Input(Param {
                    name: "b".to_string(),
                    param_type: Type::Bool,
                }),
                NetItem::Output(Param {
                    name: "sum".to_string(),
                    param_type: Type::Bool,
                }),
                NetItem::Output(Param {
                    name: "cout".to_string(),
                    param_type: Type::Bool,
                }),
                NetItem::Init(EntInit {
                    param: Param {
                        name: "cin".to_string(),
                        param_type: Type::Bool,
                    },
                    val: Expr::Literal(Literal::Bool(false)),
                }),
                NetItem::NetInst(NetInst {
                    net: "HALF_ADD".to_string(),
                    connections: vec![
                        Connection {
                            port: "a".to_string(),
                            net: "a".to_string(),
                        },
                        Connection {
                            port: "b".to_string(),
                            net: "b".to_string(),
                        },
                        Connection {
                            port: "sum".to_string(),
                            net: "h1_sum".to_string(),
                        },
                        Connection {
                            port: "cout".to_string(),
                            net: "h1_carry".to_string(),
                        },
                    ],
                }),
                NetItem::NetInst(NetInst {
                    net: "HALF_ADD".to_string(),
                    connections: vec![
                        Connection {
                            port: "a".to_string(),
                            net: "h1_sum".to_string(),
                        },
                        Connection {
                            port: "b".to_string(),
                            net: "cin".to_string(),
                        },
                        Connection {
                            port: "sum".to_string(),
                            net: "sum".to_string(),
                        },
                        Connection {
                            port: "cout".to_string(),
                            net: "h2_carry".to_string(),
                        },
                    ],
                }),
                NetItem::RelInst(RelInst {
                    asignee: "cout".to_string(),
                    rel: "OR".to_string(),
                    args: vec![
                        "h1_carry".to_string(),
                        "h2_carry".to_string(),
                    ],
                })
            ],
        }));
    }
}