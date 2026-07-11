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
//! Last Updated: 07/10/2026

use super::Parser;
use super::sync::SyncRule;
use crate::compiler::{
    token::{Token, TokenKind},
    ast::*,
    diagnostics::{Diagnostics, CompilerError, Expected},
};


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

                    self.sync(SyncRule::Item);

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

    // TODO: add sync
    pub(super) fn expect(&mut self, expected: TokenKind)-> Option<()> {
        let token = self.next();

        if token.kind == expected {
            return Some(());
        } else {
            self.diagnostics.error(CompilerError::UnexpectedToken {
                expected: vec![Expected::Token(TokenKind::Equals)],
                found: token.kind,
                span: token.span,
            });
            return None;
        }
    }

    // TODO: add sync
    pub(super) fn expect_ident(&mut self) -> Option<String> {
        let token = self.next();

        match token.kind {
            TokenKind::Ident(name) => Some(name),
            other => {
                self.diagnostics.error(CompilerError::UnexpectedToken {
                    expected: vec![Expected::Ident],
                    found: other,
                    span: token.span,
                });

                None
            }
        }
    }

    // Let token already consumed
    pub(super) fn parse_let_stmt(&mut self) -> Option<LetStatement> {
        let name = self.expect_ident()?;

        self.expect(TokenKind::Equals)?;

        let expr = match self.parse_expr(0) {
            Some(expr) => expr,
            None => Expr::Error,
        };

        self.expect(TokenKind::Semicolon)?;

        Some(LetStatement {
            name,
            expr,
        })
    }
    
    // TODO: add sync
    pub(super) fn parse_type(&mut self) -> Option<Type> {
        let token = self.next();
        
        match token.kind {
            TokenKind::Bool        => Some(Type::Bool),
            TokenKind::Impulse     => Some(Type::Impulse),
            TokenKind::Int         => Some(Type::Int),
            TokenKind::Real        => Some(Type::Real),
            TokenKind::Mod         => {
                self.expect(TokenKind::LParen)?;
                let token = self.next();
                let n = match token.kind {
                    TokenKind::IntLiteral(n) => n,
                    other => {
                        self.diagnostics.error(CompilerError::UnexpectedToken {
                            expected: vec![Expected::IntLiteral],
                            found: other,
                            span: token.span,
                        });
                        
                        return None;
                    }
                };
                self.expect(TokenKind::RParen)?;
                
                Some(Type::Mod(n))
            }
            TokenKind::Ident(name) => Some(Type::CustomType(name)),
            
            other => {
                self.diagnostics.error(CompilerError::UnexpectedToken {
                    expected: vec![
                        Expected::Token(TokenKind::Bool), 
                        Expected::Token(TokenKind::Impulse), 
                        Expected::Token(TokenKind::Int), 
                        Expected::Token(TokenKind::Real),
                        Expected::Token(TokenKind::Mod),
                        Expected::Ident,
                    ],
                    found: other,
                    span: token.span,
                });
                    
                None
            }
        }
    }

    pub(super) fn parse_param(&mut self) -> Option<Param> {
        let name = self.expect_ident()?;

        self.expect(TokenKind::Colon)?;

        let param_type = self.parse_type()?;

        Some(Param {
            name,
            param_type,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::token::TokenKind::*;
    use crate::compiler::diagnostics::Span;
    use crate::compiler::lexer::Lexer;
    
    fn build_token_vec(tokens: Vec<TokenKind>) -> Vec<Token> {
        tokens
            .into_iter()
            .map(|x| Token {kind: x, span: Span{line: 0, col: 0}})
            .collect()
    }

    #[test]
    fn let_statement() {
        // let n = 1 + 2;
        let kinds: Vec<TokenKind> = vec![Ident("n".to_string()), Equals, IntLiteral(1), Plus, IntLiteral(2), Semicolon, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_let_stmt();

        assert_eq!(result, Some(LetStatement {
            name: "n".to_string(),
            expr: Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Literal(Literal::Int(1))),
                op: BinaryOp::Add,
                right: Box::new(Expr::Literal(Literal::Int(2))),
            })
        }));
        assert!(!diagnostics.has_errors());
    }

    #[test] 
    fn bad_let_statement() {
        // let n = 1 + 2
        let kinds: Vec<TokenKind> = vec![Ident("n".to_string()), Equals, IntLiteral(1), Plus, IntLiteral(2), Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut diagnostics = Diagnostics::new();
        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_let_stmt();

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 1); // unexpected token
        
        // let 9n = 1;
        let mut diagnostics = Diagnostics::new();
        let mut lexer = Lexer::new("9a = 1;", &mut diagnostics);
        let tokens: Vec<Token> = lexer.tokenize();

        let mut parser = Parser::new(tokens, &mut diagnostics);

        let result = parser.parse_let_stmt();

        assert_eq!(result, None);
        assert_eq!(diagnostics.num_errors(), 2); // invalid num, unexpected token
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
        assert!(!diagnostics.has_errors());
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
        assert!(!diagnostics.has_errors());

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

    // #[test]
    // fn multiple_errors() {
    //     let mut diagnostics = Diagnostics::new();
    //     let mut lexer = Lexer::new("
    //         let n = 1;
    //         n = 2;
    //         let n = 3;
    //         let 9n = 4;
    //         let n = 5;
    //         let n = 6
    //         let n = 7;
    //     ", &mut diagnostics);
    //     let tokens: Vec<Token> = lexer.tokenize();

    //     let mut parser = Parser::new(tokens, &mut diagnostics);

    //     let result = parser.parse();

    //     assert_eq!(result, Program {
    //         items: vec![
    //             Item::Let(LetStatement {
    //                 name: "n".to_string(),
    //                 expr: Expr::Literal(Literal::Int(1)),
    //             }),
    //             Item::Error,
    //             Item::Let(LetStatement {
    //                 name: "n".to_string(),
    //                 expr: Expr::Literal(Literal::Int(3)),
    //             }),
    //             Item::Error,
    //             Item::Let(LetStatement {
    //                 name: "n".to_string(),
    //                 expr: Expr::Literal(Literal::Int(5)),
    //             }),
    //             Item::Error,
    //             Item::Let(LetStatement {
    //                 name: "n".to_string(),
    //                 expr: Expr::Literal(Literal::Int(7)),
    //             }),
    //         ]
    //     });
    //     assert_eq!(diagnostics.num_errors(), 4);
    // }
}
