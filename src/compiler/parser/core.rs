//! # core
//!
//! Handles the core of the parser
//!
//! ## Invariants
//!
//! - 
//!
//! Author: Cole Francis
//!
//! Last Updated: 07/06/2026

use super::Parser;
use crate::compiler::token::{Token, TokenKind};
use crate::compiler::ast::*;
use crate::compiler::diagnostics::Diagnostics;


impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token>, diagnostics: &'a mut Diagnostics) -> Self {
        Self {
            tokens,
            current: 0,
            diagnostics
        }
    }

    pub fn parse(&mut self) -> Program {
        let mut items = Vec::new();

        while self.peek().kind != TokenKind::Eof {
            let item = match self.next().kind {
                TokenKind::Let   => Item::Let(self.parse_let_stmt()),
                TokenKind::Ent_t => Item::Ent(self.parse_ent_t()),
                TokenKind::Rel_t => Item::Rel(self.parse_rel_t()),
                TokenKind::NetToken => Item::Net(self.parse_net()),
                other => panic!("Unexpected prefix token: {:?}", other),
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

    pub(super) fn expect(&mut self, expected: TokenKind) {
        let token = self.next();

        if token.kind != expected {
            panic!("expected {:?}, got {:?}\nTODO: elegant error handling", expected , token.kind);
        }
    }

    pub(super) fn expect_ident(&mut self) -> String {
        match self.next().kind {
            TokenKind::Ident(name) => name,
            other => panic!("Expected identifier, found {:?}", other),
        }
    }

    // Let token already consumed
    pub(super) fn parse_let_stmt(&mut self) -> LetStatement {
        let name = self.expect_ident();

        self.expect(TokenKind::Equals);

        let expr = self.parse_expr(0);

        self.expect(TokenKind::Semicolon);

        LetStatement {
            name,
            expr,
        }
    }

    pub(super) fn parse_type(&mut self) -> Type {
        match self.next().kind {
            TokenKind::Bool        => Type::Bool,
            TokenKind::Impulse     => Type::Impulse,
            TokenKind::Int         => Type::Int,
            TokenKind::Real        => Type::Real,
            TokenKind::Mod         => {
                self.expect(TokenKind::LParen);
                let n = match self.next().kind {
                    TokenKind::IntLiteral(n) => n,
                    other => panic!("Expected integer literal in mod(...), got {:?}", other),
                };
                self.expect(TokenKind::RParen);
                Type::Mod(n)
            }
            TokenKind::Ident(name) => Type::CustomType(name),
            other => panic!("Unexpected prefix token: {:?}", other),
        }
    }

    pub(super) fn parse_param(&mut self) -> Param {
        let name = self.expect_ident();

        self.expect(TokenKind::Colon);

        let param_type = self.parse_type();

        Param {
            name,
            param_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::token::{TokenKind::*, Span};
    use crate::compiler::lexer::Lexer;
    
    fn build_token_vec(tokens: Vec<TokenKind>) -> Vec<Token> {
        tokens
            .into_iter()
            .map(|x| Token {kind: x, span: Span{line: 0, col: 0}})
            .collect()
    }

    #[test]
    fn test_let_statement() {
        // let n = 1 + 2;
        let kinds: Vec<TokenKind> = vec![Ident("n".to_string()), Equals, IntLiteral(1), Plus, IntLiteral(2), Semicolon, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_let_stmt();

        assert_eq!(result, LetStatement {
            name: "n".to_string(),
            expr: Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Literal(Literal::Int(1))),
                op: BinaryOp::Add,
                right: Box::new(Expr::Literal(Literal::Int(2))),
            })
        });
    }

    #[test]
    fn test_parse() {
        // ent_t COIN = Bool;
        
        // let a = 1;

        // rel_t ONE : () -> Real = 1;

        // net EMPTY {}
        let kinds: Vec<TokenKind> = vec![Ent_t, Ident("COIN".to_string()), Equals, Bool, Semicolon,
            Let, Ident("a".to_string()), Equals, IntLiteral(1), Semicolon,
            Rel_t, Ident("ONE".to_string()), Colon, LParen, RParen, Arrow, Real, Equals, IntLiteral(1), Semicolon,
            NetToken, Ident("EMPTY".to_string()), LBrace, RBrace, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse();

        assert_eq!(result, Program { 
            items: vec![
                Item::Ent(EntType {
                    name: "COIN".to_string(),
                    expr: EntExpr::Type(Type::Bool),
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
    }

    // #[test]
    fn integrate_lexer_parser() {
        let mut diagnostics = Diagnostics::new();
        let mut lexer = Lexer::new("
            ent_t COIN = Bool;
        
            let a = 1;

            rel_t ONE : () -> Real = 1;

            net EMPTY {}
        ", &mut diagnostics);
        let tokens: Vec<Token> = lexer.tokenize();

        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse();

        assert_eq!(result, Program { 
            items: vec![
                Item::Ent(EntType {
                    name: "COIN".to_string(),
                    expr: EntExpr::Type(Type::Bool),
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

        // let lexer = Lexer::new("
        //     rel_t SYN: (in: Inpulse, weight: Real, last_pot: Real) -> Real = {
        //         match in {
        //             true => last_pot + weight,
        //             _ => last_pot,
        //         }
        //     }

        //     rel_t AXON: (pot: Real) -> Impulse = {
        //         let thresh = 10;

        //         match pot {
        //             >= thresh => true,
        //             _ => false,
        //         }
        //     }

        //     rel_t BODY: (spike: Impulse, last_pot: Real) -> Real {
        //         let refrac_pot = 0;
        //         let tau = 0.1;

        //         match spike {
        //             true => refrac_pot,
        //             _ => last_pot * (1 - tau),
        //         }
        //     }

        //     net Neuron {
        //         input in: Impulse;
        //         output out: Impulse;

        //         init weight: Real = 1.0;

        //         /*               
        //                                _________             ______
        //                   in -------->|         |           | AXON |
        //                               | SYNAPSE |---------->|______| ----------> out
        //             weight = 1 ------>|_________|   |               |
        //                                 ^        ___v__             |
        //                                 |-------| BODY |<------------
        //                                         |______|
        //         */

        //         next_pot := SYNAPSE(in, weight, curr_pot);

        //         out := AXON(next_pot);

        //         curr_pot := BODY(spike_out, next_pot);
        //     }
        // ");
    }
}