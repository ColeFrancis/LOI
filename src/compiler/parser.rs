//! # Parser
//!
//! Parses tokens into an AST 
//!
//! ## Invariants
//!
//! - 
//!
//! Author: Cole Francis
//!
//! Last Updated: 06/27/2026

use super::token::{Token, TokenKind};
use super::ast::*;

struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            current: 0,
        }
    }

    fn peek (&self) -> &Token {
        &self.tokens[self.current]
    }

    fn next(&mut self) -> Token {
        let token = self.tokens[self.current].clone();
        self.current += 1;
        token
    }

    fn at_end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    fn expect(&mut self, expected: TokenKind) {
        let token = self.next();

        if token.kind != expected {
            panic!("expected {:?}, got {:?}\nTODO: elegant error handling", expected , token.kind);
        }
    }

    // Pratt Parser for expressions
    //     If calling to parse expr, use min_bp = 0
    fn parse_expr(&mut self, min_bp: u8) -> Expr {
        let mut lhs = self.parse_prefix();

        loop {
            let Some((op, left_bp, right_bp)) = 
                self.infix_into(&self.peek().kind)
            else {
                break;
            };

            if left_bp < min_bp {
                break;
            }

            self.next();
            
            let rhs = self.parse_expr(right_bp);

            lhs = Expr::Binary(BinaryExpr {
                left: Box::new(lhs),
                op,
                right: Box::new(rhs),
            });
        }

        lhs
    }

    fn parse_prefix(&mut self) -> Expr {
        match self.next().kind {
            TokenKind::BoolLiteral(n) => Expr::Literal(Literal::Bool(n)),
            TokenKind::IntLiteral(n)  => Expr::Literal(Literal::Int(n)),
            TokenKind::RealLiteral(n) => Expr::Literal(Literal::Real(n)),
            
            TokenKind::Ident(str) => Expr::Ident(str),

            TokenKind::Minus => {
                let rhs = self.parse_expr(25); // Unary binding power
                Expr::Unary(UnaryExpr {op: UnaryOp::Neg, expr: Box::new(rhs)})
            }
            TokenKind::BitNot => {
                let rhs = self.parse_expr(25); // Unary binding power
                Expr::Unary(UnaryExpr {op: UnaryOp::BitNot, expr: Box::new(rhs)})
            }

            TokenKind::LParen => {
                let first = self.parse_expr(0);
            
                match self.next().kind {
                    TokenKind::RParen => {
                        first
                    }
            
                    TokenKind::Comma => {
                        let mut elements = vec![first];
                        elements.push(self.parse_expr(0));
                        
                        while self.peek().kind == TokenKind::Comma {
                            self.next();
                            elements.push(self.parse_expr(0));
                        }
            
                        self.expect(TokenKind::RParen);
                        Expr::Tuple(TupleExpr { elements })
                    }
            
                    _ => panic!("Expected ',' or ')'. TODO: elegant error handling"),
                }
            }

            TokenKind::Match => {
                self.parse_match()
            }
            // TokenKind::Sample => {

            // }
            _ => panic!("TODO: Elegent Error Handling"),
        }
    }

    fn infix_into(&self, op: &TokenKind) -> Option<(BinaryOp, u8, u8)> {
        match op {
            TokenKind::Gt       => Some((BinaryOp::Gt,   1,  2)),
            TokenKind::Lt       => Some((BinaryOp::Lt,   1,  2)),
            TokenKind::Ge       => Some((BinaryOp::Ge,   1,  2)),
            TokenKind::Le       => Some((BinaryOp::Le,   1,  2)),
            TokenKind::Plus     => Some((BinaryOp::Add, 10, 11)),
            TokenKind::Minus    => Some((BinaryOp::Sub, 10, 11)),
            TokenKind::Asterisk => Some((BinaryOp::Mul, 20, 21)),
            TokenKind::Slash    => Some((BinaryOp::Div, 20, 21)),
            TokenKind::Caret    => Some((BinaryOp::Pow, 31, 30)),
            _ => None,
        }
    }

    fn parse_match(&mut self) -> Expr {
        let scrutinee = self.parse_expr(0);

        self.expect(TokenKind::LBrace);

        let mut arms = Vec::new();

        while self.peek().kind != TokenKind::RBrace {
            arms.push(self.parse_match_arm());

            if self.peek().kind == TokenKind::Comma {
                self.next();
            } else {
                break;
            }
        }

        self.expect(TokenKind::RBrace);

        Expr::Match(MatchExpr {
            scrutinee: Box::new(scrutinee),
            arms,
        })
    }

    fn parse_match_arm(&mut self) -> MatchArm {
        let pattern = self.parse_pattern();

        self.expect(TokenKind::FatArrow);

        let expr = self.parse_expr(0);

        MatchArm {
            pattern,
            expr,
        }
    }

    fn parse_pattern(&mut self) -> Vec<SimplePattern> {
        let mut patterns = vec![self.parse_simple_pattern()];

        while self.peek().kind == TokenKind::Pipe {
            self.next();
            patterns.push(self.parse_simple_pattern());
        }

        patterns
    }

    fn parse_simple_pattern(&mut self) -> SimplePattern {
        match self.next().kind {
            TokenKind::Underscore     => SimplePattern::Default,

            TokenKind::BoolLiteral(n) => SimplePattern::Literal(Literal::Bool(n)),
            TokenKind::IntLiteral(n)  => SimplePattern::Literal(Literal::Int(n)),
            TokenKind::RealLiteral(n) => SimplePattern::Literal(Literal::Real(n)),

            TokenKind::Ident(name)    => SimplePattern::Ident(name),

            TokenKind::LParen         => self.parse_tuple_pattern(),

            TokenKind::Gt             => self.parse_comparison_pattern(CompOp::Gt),
            TokenKind::Lt             => self.parse_comparison_pattern(CompOp::Lt),
            TokenKind::Ge             => self.parse_comparison_pattern(CompOp::Ge),
            TokenKind::Le             => self.parse_comparison_pattern(CompOp::Le),

            token => panic!("Unexpected token in pattern: {:?}", token),
        }
    }

    fn parse_tuple_pattern(&mut self) -> SimplePattern {
        let mut items = vec![self.parse_simple_pattern()];

        self.expect(TokenKind::Comma);

        items.push(self.parse_simple_pattern());

        while self.peek().kind == TokenKind::Comma {
            self.next();
            items.push(self.parse_simple_pattern());
        }

        self.expect(TokenKind::RParen);

        SimplePattern::Tuple(items)
    }

    fn parse_comparison_pattern(&mut self, op: CompOp) -> SimplePattern {
        let expr = self.parse_expr(0);

        SimplePattern::Comparison(ComparisonPattern {
            op,
            expr: Box::new(expr),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::token::{TokenKind::*, Span};

    fn build_token_vec(tokens: Vec<TokenKind>) -> Vec<Token> {
        tokens
            .into_iter()
            .map(|x| Token {kind: x, span: Span{line: 0, col: 0}})
            .collect()
    }

    fn build_s_expr(expr: &Expr) -> String {
        match expr {
            Expr::Literal(Literal::Int(n)) => n.to_string(),
            Expr::Literal(Literal::Bool(b)) => b.to_string(),
            Expr::Literal(Literal::Real(x)) => x.to_string(),

            Expr::Ident(name) => name.clone(),

            Expr::Unary(unary) => {
                format!(
                    "({} {})",
                    unary_op_to_str(&unary.op),
                    build_s_expr(&unary.expr),
                )
            }

            Expr::Binary(binary) => {
                format!(
                    "({} {} {})",
                    binary_op_to_str(&binary.op),
                    build_s_expr(&binary.left),
                    build_s_expr(&binary.right),
                )
            }
            _ => {
                panic!("build_s_expression can only handle Literals, Identifiers, Unary Expressions, and Binary Expressions");
            }
        }
    }

    fn unary_op_to_str(op: &UnaryOp) -> &'static str {
        match op {
            UnaryOp::Neg    => "-",
            UnaryOp::BitNot => "~",
        }
    }

    fn binary_op_to_str(op: &BinaryOp) -> &'static str {
        match op {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Pow => "^",
            BinaryOp::Gt  => ">",
            BinaryOp::Lt  => "<",
            BinaryOp::Ge  => ">=",
            BinaryOp::Le  => "<=",
        }
    }

    #[test]
    fn test_build_s_expr() {
        let start: Expr = Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Unary(UnaryExpr {
                op: UnaryOp::Neg,
                expr: Box::new(Expr::Literal(Literal::Int(5))),
            })),
            op: BinaryOp::Add,
            right: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Literal(Literal::Int(2))),
                op: BinaryOp::Mul,
                right: Box::new(Expr::Ident("a".to_string())),
            })),
        });

        let result: String = build_s_expr(&start);

        assert_eq!(result, "(+ (- 5) (* 2 a))".to_string());
    }

    #[test]
    fn test_literal_and_ident_expr() {
        let kinds: Vec<TokenKind> = vec![IntLiteral(3), Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut parser = Parser::new(tokens);

        let result: Expr = parser.parse_prefix();

        assert_eq!(result, Expr::Literal(Literal::Int(3)));
    
        let kinds: Vec<TokenKind> = vec![Ident("hey".to_string()), Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut parser = Parser::new(tokens);

        let result: Expr = parser.parse_prefix();

        assert_eq!(result, Expr::Ident("hey".to_string()));
    }
    
    #[test]
    fn test_unary_expr() {
        // ---6
        let kinds: Vec<TokenKind> = vec![Minus, Minus, Minus, IntLiteral(6), Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);
        
        let mut parser = Parser::new(tokens);

        let result: Expr = parser.parse_prefix();
        
        let result_str: String = build_s_expr(&result);

        assert_eq!(result_str, "(- (- (- 6)))".to_string());
    }

    #[test]
    fn test_binary_expr() {
        // -5 + 2 * a + b
        let kinds: Vec<TokenKind> = vec![Minus, IntLiteral(5), Plus, 
            IntLiteral(2), Asterisk, Ident("a".to_string()), Plus, 
            Ident("b".to_string()), Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut parser = Parser::new(tokens);

        let result: Expr = parser.parse_expr(0);

        let result_str: String = build_s_expr(&result);

        assert_eq!(result_str, "(+ (+ (- 5) (* 2 a)) b)".to_string()); 
    
        // (9 + 10) * 5
        let kinds: Vec<TokenKind> = vec![LParen, IntLiteral(9), Plus, 
        IntLiteral(10), RParen, Asterisk, IntLiteral(5), Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut parser = Parser::new(tokens);

        let result: Expr = parser.parse_expr(0);

        let result_str: String = build_s_expr(&result);

        assert_eq!(result_str, "(* (+ 9 10) 5)".to_string()); 
    
        //-3^(-7)^(8-2-4/-1)
        let kinds: Vec<TokenKind> = vec![Minus, IntLiteral(3), Caret, LParen, 
            Minus, IntLiteral(7), RParen, Caret, LParen, IntLiteral(8), Minus,
            IntLiteral(2), Minus, IntLiteral(4), Slash, Minus, IntLiteral(1), 
            RParen, Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut parser = Parser::new(tokens);

        let result: Expr = parser.parse_expr(0);

        let result_str: String = build_s_expr(&result);

        assert_eq!(result_str, "(- (^ 3 (^ (- 7) (- (- 8 2) (/ 4 (- 1))))))".to_string()); 
    }
    
    #[test]
    fn test_tuple_expr() {
        assert!(false); // TODO
    }

    #[test]
    fn test_match() {
        // match a {
        //     1 => 1+a,
        //     _ => a,
        // } - 1
        let kinds: Vec<TokenKind> = vec![Match, Ident("a".to_string()), LBrace, 
            IntLiteral(1), FatArrow, IntLiteral(1), Plus, Ident("a".to_string()), Comma,
            Underscore, FatArrow, Ident("a".to_string()), Comma,
            RBrace, Minus, IntLiteral(2), Eof];
        let tokens: Vec<Token> = build_token_vec(kinds);

        let mut parser = Parser::new(tokens);

        let result: Expr = parser.parse_expr(0);

        assert_eq!(result, Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Match(MatchExpr {
                scrutinee: Box::new(Expr::Ident("a".to_string())),
                arms: vec![MatchArm {
                    pattern: vec![SimplePattern::Literal(Literal::Int(1))],
                    expr: Expr::Binary(BinaryExpr {
                        left: Box::new(Expr::Literal(Literal::Int(1))),
                        op: BinaryOp::Add,
                        right: Box::new(Expr::Ident("a".to_string())),
                    })
                }, MatchArm {
                    pattern: vec![SimplePattern::Default],
                    expr: Expr::Ident("a".to_string()),
                }]
            })),
            op: BinaryOp::Sub,
            right: Box::new(Expr::Literal(Literal::Int(2))),
        }));

        // TODO: Finish tuple expression parsing first
        // match (a, b) {
        //     (1, 0) | (0, 1) => 1,
        //     _ => 0,
        // }
        // let kinds: Vec<TokenKind> = vec![Match, Lparen, Ident("a".to_string()), Comma,  Ident("b".to_string()), RParen, LBrace,
        //     Lparen, IntLiteral(1), Comma, IntLiteral(0), RParen, Pipe, Lparen, IntLiteral(0), Comma, IntLiteral(1), RParen, FatArrow, IntLiteral(1),
        //     Underscore, FatArrow, IntLiteral(0),
        //     RBrace, Eof];
        // let tokens: Vec<Token> = build_token_vec(kinds);

        // let mut parser = Parser::new(tokens);

        // let result: Expr = parser.parse_expr(0);

        // assert_eq!(result, Expr::Match(MatchExpr {
        //     scrutinee: Box::new(),
        //     arms: vec![],
        // }));
    }
}