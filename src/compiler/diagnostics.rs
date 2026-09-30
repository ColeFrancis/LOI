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

//! # diagnostics
//!
//! Defines error types from the lexer, parser, and semantic analysis
//!
//! ## Invariants
//!
//! - All compiler errors will be defined in the Diagnostic enum
//!
//! Author: Cole Francis

use super::lexer::token::TokenKind;
use super::sem_analyzer::types::Type;
use super::symbol::SymbolKind;

#[derive(PartialEq, Debug)]
pub struct Diagnostics {
    errors: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
        }
    }

    pub fn error(&mut self, error: Diagnostic) {
        self.errors.push(error);
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn num_errors(&self) -> usize {
        self.errors.len()
    }

    pub fn errors(&self) -> &[Diagnostic] {
        &self.errors
    }

    pub fn debug_print(&self) {
        println!("{} error(s):", self.errors.len());

        for (i, error) in self.errors.iter().enumerate() {
            println!("{}: {:#?}", i + 1, error);
        }
    }

    pub fn print(&self) {
        for diagnostic in &self.errors {
            diagnostic.print();
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Diagnostic {
    ////////////////////
    // Lexer
    ////////////////////

    UnknownToken {
        lexeme: String,
        span: Span,
    },

    InvalidNum {
        lexeme: String,
        span: Span,
    },

    ////////////////////
    // Parser
    ////////////////////

    UnexpectedToken {
        expected: Vec<Expected>,
        found: TokenKind,
        span: Span,
    },

    // No nesting tuples in cases
    NestedTupleExpr {
        span: Span,
    },

    ////////////////////
    // Semantic Analysis
    ////////////////////

    DuplicateDefinition {
        name: String,
        old_span: Span,
        new_span: Span,
    },

    // When searching up a symbol resolving names
    UndefinedIdent {
        name: String,
        span: Span,
    },

    // When resolving names of NetInst
    UndefinedPort {
        name: String,
        span: Span,
    },

    // When resolving names of NetInst
    DuplicatePort {
        name: String,
        span: Span,
    },
    
    // When processing instantiations
    UnexpectedIdent {
        expected: Vec<SymbolKind>,
        found: SymbolKind,
        span: Span,
    },

    // In binary expressions or matching case scrutinee and arms
    // and for inits on entities
    IncompatibleTypes {
        left: Type,
        right: Type,
        op_span: Span,
    },

    // Unary/binary expressions
    IncompatibleOp { 
        expr_type: Type,
        op: Operation,
        op_span: Span,
    },

    // Prob expr literal but incorrect type
    NonRealProb {
        prob_type: Type,
        arm_span: Span,
    },

    UnequalTupleLength {
        left_len: usize,
        right_len: usize,
        right_span: Span,
    },

    IllegalScrutineeExpr {
        expected: Vec<ExprType>,
        found: ExprType,
        cases_span: Span,
    },

    IncompatibleReturnType {
        return_type: Type,
        expr_type: Type,
        rel_span: Span,
    },

    // For re_inst and net_inst
    MismatchedEntType {
        expected: Type,
        found: Type,
        span: Span
    },

    // for rel_inst
    IncorrectNumberOfArgs { 
        expected_len: usize,
        actual_len: usize,
        rel_span: Span,
    },

    NonexistantNetPort {
        name: String,
        span: Span,
    },

    // When folding expressions
    DivideByZero {
        op_span: Span,
    },

    // When folding expressions
    NegExpOnInt {
        op_span: Span,
    },

    // Checked when folding expressions
    NoReturnArm {
        span: Span,
    },

    // when folding cases expression
    DuplicatePattern {
        old_arm_span: Span,
        arm_span: Span,
    },

    NoDefaultPattern {
        cases_span: Span,
    },

    ProbOutOfRange {
        total_prob: bool,
        val: f64,
        span: Span,
    },

    MultipleDefaultProb {
        arm_span: Span,
    },

    MultipleEntDrivers {
        name: String,
        first_span: Span,
        last_span: Span
    },

    ///////////////
    // CodeGen
    ///////////////

    // There are only 62 registers available for use in the interpreter
    TooManySymbols { 
        rel_name: String,
        rel_span: Span,
    },

    ////////////////
    // Synthesize
    ////////////////

    NonexistantTopLevelNet {
        name: String,
    },
}

#[derive(Debug, PartialEq)]
pub enum Expected {
    Token(TokenKind),
    Expr,
    Pattern,
    Ident,
    IntLiteral,
}

impl std::fmt::Display for Expected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expected::Token(token_kind) => write!(f, "{token_kind}"),
            Expected::Expr => write!(f, "Expression"),
            Expected::Pattern => write!(f, "Pattern"),
            Expected::Ident => write!(f, "Identifier"),
            Expected::IntLiteral => write!(f, "Int Literal"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Operation {
    Cmp,
    Add,
    Sub,
    Mul,
    Div, 
    Pow,
    Or,
    And,
    Not,
}

#[derive(Debug, PartialEq)]
pub enum ExprType {
    Literal,
    Ident,
    Unary,
    Binary,
    Tuple,
    Block,
    Cases,
    Sample,
    Error,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

impl Diagnostic {
    fn print(&self) {
        match self {
            ///////////////
            // Lexer Errors
            ///////////////
            Diagnostic::UnknownToken {lexeme, span} => {
                eprintln!("Compiler Error: Unknown token \"{}\"\nat line {}:{}\n\n", lexeme, span.line, span.col);
            }

            Diagnostic::InvalidNum {lexeme, span} => {
                eprintln!("Compiler Error: Invalid number \"{}\"\nat line {}:{}\n\n", lexeme, span.line, span.col);
            }

            ////////////////
            // Parser Errors
            ////////////////
            Diagnostic::UnexpectedToken {expected, found, span} => {
                let expected = expected.iter().map(|token| format!("{token}")).collect::<Vec<_>>().join(", ");

                eprintln!("Compiler Error: Unexpected token: {}. Expected one of: {}\nat line {}:{}\n\n", found, expected, span.line, span.col);
            }

            Diagnostic::NestedTupleExpr {span} => {
                eprintln!("Compiler Error: Nested Tuple Expressions\nat line {}:{}\n\n", span.line, span.col);
            }

            ///////////////////////////
            // Semantic Analysis Errors
            ///////////////////////////
            Diagnostic::DuplicateDefinition {name, old_span, new_span} => {
                eprintln!("Compiler Error: Duplicate definition of \"{}\", previously defined at {}:{}\nat line {}:{}\n\n", name, old_span.line, old_span.col, new_span.line, new_span.col);
            }

            Diagnostic::UndefinedIdent {name, span} => {
                eprintln!("Compiler Error: Undefined identifier: \"{}\"\nat line {}:{}\n\n", name, span.line, span.col);
            }

            Diagnostic::UndefinedPort {name, span} => {
                eprintln!("Compiler Error: Undefined net port: \"{}\"\nat line {}:{}\n\n", name, span.line, span.col);
            }

            Diagnostic::DuplicatePort {name, span} => {
                eprintln!("Compiler Error: Port \"{}\" assigned multiple times\nat line {}:{}\n\n", name, span.line, span.col);
            }

            Diagnostic::UnexpectedIdent {expected, found, span} => {
                let expected = expected.iter().map(|ident| format!("{ident}")).collect::<Vec<_>>().join(", ");

                eprintln!("Compiler Error: Unexpected Identifer: {}. Expected one of: {}\nat line {}:{}\n\n", found, expected, span.line, span.col);
            }

            Diagnostic::IncompatibleTypes {left, right, op_span} => {
                eprintln!("Compiler Error: Incompatible variable types: {} and {}\nat line {}:{}\n\n", left, right, op_span.line, op_span.col);
            }

            Diagnostic::IncompatibleOp {expr_type, op, op_span} => {
                //eprintln!("Compiler Error: \nat line {}:{}\n\n");
            }

            Diagnostic::NonRealProb {prob_type, arm_span} => {
                //eprintln!("Compiler Error: \nat line {}:{}\n\n");
            }

            Diagnostic::UnequalTupleLength {left_len, right_len, right_span} => {
                //eprintln!("Compiler Error: \nat line {}:{}\n\n");
            }

            Diagnostic::IllegalScrutineeExpr {expected, found, cases_span} => {
                //eprintln!("Compiler Error: \nat line {}:{}\n\n");
            }

            Diagnostic::IncompatibleReturnType {return_type, expr_type, rel_span} => {
                //eprintln!("Compiler Error: \nat line {}:{}\n\n");
            }

            Diagnostic::MismatchedEntType {expected, found, span} => {
                //eprintln!("Compiler Error: \nat line {}:{}\n\n");
            }

            Diagnostic::IncorrectNumberOfArgs {expected_len, actual_len, rel_span} => {
                //eprintln!("Compiler Error: \nat line {}:{}\n\n");
            }

            Diagnostic::NonexistantNetPort {name, span} => {
                //eprintln!("Compiler Error: \nat line {}:{}\n\n");
            }

            Diagnostic::DivideByZero {op_span} => {
                //eprintln!("Compiler Error: \nat line {}:{}\n\n");
            }

            Diagnostic::NegExpOnInt {op_span} => {
                //eprintln!("Compiler Error: \nat line {}:{}\n\n");
            }

            Diagnostic::NoReturnArm {span} => {
                //eprintln!("Compiler Error: \nat line {}:{}\n\n");
            }

            Diagnostic::DuplicatePattern {old_arm_span, arm_span} => {
                //eprintln!("Compiler Error: \nat line {}:{}\n\n");
            }

            Diagnostic::NoDefaultPattern {cases_span} => {
                //eprintln!("Compiler Error: \nat line {}:{}\n\n");
            }

            Diagnostic::ProbOutOfRange {total_prob, val, span} => {
                //eprintln!("Compiler Error: \nat line {}:{}\n\n");
            }

            Diagnostic::MultipleDefaultProb {arm_span} => {
                //eprintln!("Compiler Error: \nat line {}:{}\n\n");
            }

            Diagnostic::MultipleEntDrivers {name, first_span, last_span} => {
                //eprintln!("Compiler Error: \nat line {}:{}\n\n");
            }

            /////////////////
            // CodeGen Errors
            /////////////////
            Diagnostic::TooManySymbols {rel_name, rel_span} => {
                //eprintln!("Compiler Error: \nat line {}:{}\n\n");
            }

            ////////////////////
            // Syntehsize Errors
            ////////////////////
            Diagnostic::NonexistantTopLevelNet {name} => {
                //eprintln!("Compiler Error: \nat line {}:{}\n\n");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer::Lexer;
    use crate::compiler::parser::Parser;

    #[test]
    fn no_errors() {
        let mut diagnostics = Diagnostics::new();
        let tokens = Lexer::new("
            ent_t COIN = {H, T};
        
            let a = 1;

            rel_t ONE : () -> Real = 1;

            net EMPTY {}
        ", &mut diagnostics).tokenize();

        Parser::new(tokens, &mut diagnostics).parse();

        assert!(!diagnostics.has_errors());
    }

    #[test]
    fn lexer_1() {
        let mut diagnostics = Diagnostics::new();
        Lexer::new(
        "@ 9a
        ", &mut diagnostics).tokenize();

        assert_eq!(diagnostics.errors, vec![
            Diagnostic::UnknownToken{
                lexeme: "@".to_string(),
                span: Span {
                    line: 1,
                    col: 1
                }
            },
            Diagnostic::InvalidNum{
                lexeme: "9a".to_string(),
                span: Span {
                    line: 1,
                    col: 3
                }
            },
        ]);
    }

    #[test]
    fn rel() {
        let mut diagnostics = Diagnostics::new();
        let tokens = Lexer::new(
        "rel_t A () -> Real a;
        ", &mut diagnostics).tokenize();

        Parser::new(tokens, &mut diagnostics).parse();

        assert_eq!(diagnostics.errors, vec![
            Diagnostic::UnexpectedToken {
                expected: vec![Expected::Token(TokenKind::Colon)],
                found: TokenKind::LParen,
                span: Span {
                    line: 1,
                    col: 9
                }
            }
        ]);
    }

    #[test]
    fn expr() {
        let mut diagnostics = Diagnostics::new();
        let tokens = Lexer::new(
"let n = cases a {
    let => 1,
};", &mut diagnostics).tokenize();

        Parser::new(tokens, &mut diagnostics).parse();

        assert_eq!(diagnostics.errors, vec![
            Diagnostic::UnexpectedToken {
                expected: vec![Expected::Pattern],
                found: TokenKind::Let,
                span: Span {
                    line: 2,
                    col: 5,
                }
            }
        ]);
    }

    #[test]
    fn multiple_errors_1() {
        let mut diagnostics = Diagnostics::new();
        let tokens = Lexer::new(
"let n = 1;
n = 2;
let n = 3;
let 9n = 4;
let n = 5;
let n = 6
let n = 7;
let n = @;", &mut diagnostics).tokenize();

        Parser::new(tokens, &mut diagnostics).parse();

        assert_eq!(diagnostics.errors, vec![
            Diagnostic::InvalidNum {
                lexeme: "9n".to_string(),
                span: Span {
                    line: 4,
                    col: 5,
                }
            },
            Diagnostic::UnknownToken {
                lexeme: "@".to_string(),
                span: Span {
                    line: 8,
                    col: 9,
                }
            },
            Diagnostic::UnexpectedToken {
                expected: vec![
                    Expected::Token(TokenKind::Let),
                    Expected::Token(TokenKind::Ent_t),
                    Expected::Token(TokenKind::Rel_t),
                    Expected::Token(TokenKind::NetToken),
                ],
                found: TokenKind::Ident("n".to_string()),
                span: Span {
                    line: 2,
                    col: 1,
                }
            },
            Diagnostic::UnexpectedToken {
                expected: vec![Expected::Ident],
                found: TokenKind::ErrorToken,
                span: Span {
                    line: 4,
                    col: 5,
                }
            },
            Diagnostic::UnexpectedToken {
                expected: vec![Expected::Token(TokenKind::Semicolon)],
                found: TokenKind::Let,
                span: Span {
                    line: 7,
                    col: 1,
                }
            },
            Diagnostic::UnexpectedToken {
                expected: vec![Expected::Expr],
                found: TokenKind::ErrorToken,
                span: Span {
                    line: 8,
                    col: 9,
                }
            },
        ]);
    }

    use crate::compiler::ast::Ident;

    // #[test]
    // fn print_test() {
    //     Diagnostic::IncompatibleTypes {
    //         left: Type::Int, 
    //         right: Type::Custom(Ident::Str{val: "A".to_string(), span:Span{line: 0, col: 0}}), 
    //         op_span: Span{line: 111, col: 2},
    //     }.print();

    //     assert!(false);
    // }
}
