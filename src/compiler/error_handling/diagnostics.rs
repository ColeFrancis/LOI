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

use super::span::Span;
use crate::compiler::{
    lexer::token::TokenKind,
    objects::{symbol::SymbolKind, types::Type},
};

#[derive(PartialEq, Debug)]
pub struct Diagnostics {
    errors: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
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
        span: Span,
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
        last_span: Span,
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

impl std::fmt::Display for Diagnostics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for diagnostic in &self.errors {
            writeln!(f, "Compiler Error: {diagnostic}")?;
            writeln!(f)?;
        }

        writeln!(f, "{} Compiler Errors found.", self.num_errors())
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ///////////////
            // Lexer Errors
            ///////////////
            Diagnostic::UnknownToken { lexeme, span } => {
                write!(
                    f,
                    "Unknown token \"{}\"\nat line {}:{}",
                    lexeme, span.line, span.col
                )
            }

            Diagnostic::InvalidNum { lexeme, span } => {
                write!(
                    f,
                    "Invalid number \"{}\"\nat line {}:{}",
                    lexeme, span.line, span.col
                )
            }

            ////////////////
            // Parser Errors
            ////////////////
            Diagnostic::UnexpectedToken {
                expected,
                found,
                span,
            } => {
                let expected = expected
                    .iter()
                    .map(|token| format!("{token}"))
                    .collect::<Vec<_>>()
                    .join(", ");

                write!(
                    f,
                    "Unexpected token: {}. Expected one of: {}\nat line {}:{}",
                    found, expected, span.line, span.col
                )
            }

            Diagnostic::NestedTupleExpr { span } => {
                write!(
                    f,
                    "Nested Tuple Expressions\nat line {}:{}",
                    span.line, span.col
                )
            }

            ///////////////////////////
            // Semantic Analysis Errors
            ///////////////////////////
            Diagnostic::DuplicateDefinition {
                name,
                old_span,
                new_span,
            } => {
                write!(
                    f,
                    "Duplicate definition of \"{}\", previously defined at {}:{}\nat line {}:{}",
                    name, old_span.line, old_span.col, new_span.line, new_span.col
                )
            }

            Diagnostic::UndefinedIdent { name, span } => {
                write!(
                    f,
                    "Undefined identifier: \"{}\"\nat line {}:{}",
                    name, span.line, span.col
                )
            }

            Diagnostic::UndefinedPort { name, span } => {
                write!(
                    f,
                    "Undefined net port: \"{}\"\nat line {}:{}",
                    name, span.line, span.col
                )
            }

            Diagnostic::DuplicatePort { name, span } => {
                write!(
                    f,
                    "Port \"{}\" assigned multiple times\nat line {}:{}",
                    name, span.line, span.col
                )
            }

            Diagnostic::UnexpectedIdent {
                expected,
                found,
                span,
            } => {
                let expected = expected
                    .iter()
                    .map(|ident| format!("{ident}"))
                    .collect::<Vec<_>>()
                    .join(", ");

                write!(
                    f,
                    "Unexpected Identifer: {}. Expected one of: {}\nat line {}:{}",
                    found, expected, span.line, span.col
                )
            }

            Diagnostic::IncompatibleTypes {
                left,
                right,
                op_span,
            } => {
                write!(
                    f,
                    "Incompatible variable types: {} and {}\nat line {}:{}",
                    left, right, op_span.line, op_span.col
                )
            }

            Diagnostic::IncompatibleOp {
                expr_type,
                op,
                op_span,
            } => {
                write!(
                    f,
                    "Incompatible operation {} given expression type: {}\nat line {}:{}",
                    op, expr_type, op_span.line, op_span.col
                )
            }

            Diagnostic::NonRealProb {
                prob_type,
                arm_span,
            } => {
                write!(
                    f,
                    "Probability type must be Real. Instead found: {}\nat line {}:{}",
                    prob_type, arm_span.line, arm_span.col
                )
            }

            Diagnostic::UnequalTupleLength {
                left_len,
                right_len,
                right_span,
            } => {
                write!(
                    f,
                    "Tuple lengths are not equal: {} and {}\nat line {}:{}",
                    left_len, right_len, right_span.line, right_span.col
                )
            }

            Diagnostic::IllegalScrutineeExpr {
                expected,
                found,
                cases_span,
            } => {
                let expected = expected
                    .iter()
                    .map(|expr_type| format!("{expr_type}"))
                    .collect::<Vec<_>>()
                    .join(", ");

                write!(
                    f,
                    "Illegal scrutinee expression type: {}. Expected one of: {}\nat line {}:{}",
                    found, expected, cases_span.line, cases_span.col
                )
            }

            Diagnostic::IncompatibleReturnType {
                return_type,
                expr_type,
                rel_span,
            } => {
                write!(
                    f,
                    "Incompatible expression return type: {}. Expected type: {}\nat line {}:{}",
                    expr_type, return_type, rel_span.line, rel_span.col
                )
            }

            Diagnostic::MismatchedEntType {
                expected,
                found,
                span,
            } => {
                write!(
                    f,
                    "Mismatched entity types. Expected: {}, found: {}\nat line {}:{}",
                    expected, found, span.line, span.col
                )
            }

            Diagnostic::IncorrectNumberOfArgs {
                expected_len,
                actual_len,
                rel_span,
            } => {
                write!(
                    f,
                    "Incorrect number of args in relation instantiation. Expected: {}, found: {}\nat line {}:{}",
                    expected_len, actual_len, rel_span.line, rel_span.col
                )
            }

            Diagnostic::NonexistantNetPort { name, span } => {
                write!(
                    f,
                    "Net port \"{}\" used but not found.\nat line {}:{}",
                    name, span.line, span.col
                )
            }

            Diagnostic::DivideByZero { op_span } => {
                write!(
                    f,
                    "Divide by zero while folding expression.\nat line {}:{}",
                    op_span.line, op_span.line
                )
            }

            Diagnostic::NegExpOnInt { op_span } => {
                write!(
                    f,
                    "Integer raised to negative power.\nat line {}:{}",
                    op_span.line, op_span.line
                )
            }

            Diagnostic::NoReturnArm { span } => {
                write!(
                    f,
                    "No return arms on expression.\nat line {}:{}",
                    span.line, span.col
                )
            }

            Diagnostic::DuplicatePattern {
                old_arm_span,
                arm_span,
            } => {
                write!(
                    f,
                    "Duplicate pattern on cases arm. First pattern at {}:{}\nat line {}:{}",
                    old_arm_span.line, old_arm_span.col, arm_span.line, arm_span.col
                )
            }

            Diagnostic::NoDefaultPattern { cases_span } => {
                write!(
                    f,
                    "No default pattern given.\nat line {}:{}",
                    cases_span.line, cases_span.col
                )
            }

            Diagnostic::ProbOutOfRange {
                total_prob,
                val,
                span,
            } => match total_prob {
                true => write!(
                    f,
                    "Arm's probability value \"{}\" out of range. Should be >= 0 and <= 1.\nat line {}:{}",
                    val, span.line, span.col
                ),
                false => write!(
                    f,
                    "Total probability value \"{}\" out of range. Should be 1.\nat line {}:{}",
                    val, span.line, span.col
                ),
            },

            Diagnostic::MultipleDefaultProb { arm_span } => {
                write!(
                    f,
                    "Multiple default probabilites given.\nat line {}:{}",
                    arm_span.line, arm_span.col
                )
            }

            Diagnostic::MultipleEntDrivers {
                name,
                first_span,
                last_span,
            } => {
                write!(
                    f,
                    "Entity \"{}\" driven by multiple sources. First driven at {}:{}\nat line {}:{}",
                    name, first_span.line, first_span.col, last_span.line, last_span.col
                )
            }

            /////////////////
            // CodeGen Errors
            /////////////////
            Diagnostic::TooManySymbols { rel_name, rel_span } => {
                write!(
                    f,
                    "Too many internal registers needed to evaluate rel_t: \"{}\". Please reduce the number of variables, cases or probability arms, or simplify expressions.\nat line {}:{}\n\n",
                    rel_name, rel_span.line, rel_span.col
                )
            }

            ////////////////////
            // Syntehsize Errors
            ////////////////////
            Diagnostic::NonexistantTopLevelNet { name } => {
                write!(f, "Given top level net \"{}\" does not exist.", name)
            }
        }
    }
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

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::Cmp => write!(f, "Comparison"),
            Operation::Add => write!(f, "Add"),
            Operation::Sub => write!(f, "Subtract"),
            Operation::Mul => write!(f, "Multiply"),
            Operation::Div => write!(f, "Divide"),
            Operation::Pow => write!(f, "Power"),
            Operation::Or => write!(f, "Boolean Or"),
            Operation::And => write!(f, "Boolean And"),
            Operation::Not => write!(f, "Boolean Not"),
        }
    }
}

impl std::fmt::Display for ExprType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExprType::Literal => write!(f, "Literal"),
            ExprType::Ident => write!(f, "Ident"),
            ExprType::Unary => write!(f, "Unary"),
            ExprType::Binary => write!(f, "Binary"),
            ExprType::Tuple => write!(f, "Tuple"),
            ExprType::Block => write!(f, "Block"),
            ExprType::Cases => write!(f, "Cases"),
            ExprType::Sample => write!(f, "Sample"),
            ExprType::Error => write!(f, "Error"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::error_handling::span::Span;
    use crate::compiler::lexer::Lexer;
    use crate::compiler::parser::Parser;

    #[test]
    fn no_errors() {
        let mut diagnostics = Diagnostics::new();
        let tokens = Lexer::new(
            "
            ent_t COIN = {H, T};
        
            let a = 1;

            rel_t ONE () -> Real = 1;

            net EMPTY {}
        ",
            &mut diagnostics,
        )
        .tokenize();

        Parser::new(tokens, &mut diagnostics).parse();

        assert!(!diagnostics.has_errors());
    }

    #[test]
    fn lexer_1() {
        let mut diagnostics = Diagnostics::new();
        Lexer::new(
            "@ 9a
        ",
            &mut diagnostics,
        )
        .tokenize();

        assert_eq!(
            diagnostics.errors,
            vec![
                Diagnostic::UnknownToken {
                    lexeme: "@".to_string(),
                    span: Span { line: 1, col: 1 }
                },
                Diagnostic::InvalidNum {
                    lexeme: "9a".to_string(),
                    span: Span { line: 1, col: 3 }
                },
            ]
        );
    }

    #[test]
    fn rel() {
        let mut diagnostics = Diagnostics::new();
        let tokens = Lexer::new(
            "rel_t A :() -> Real a;
        ",
            &mut diagnostics,
        )
        .tokenize();

        Parser::new(tokens, &mut diagnostics).parse();

        assert_eq!(
            diagnostics.errors,
            vec![Diagnostic::UnexpectedToken {
                expected: vec![Expected::Token(TokenKind::LParen)],
                found: TokenKind::Colon,
                span: Span { line: 1, col: 9 }
            }]
        );
    }

    #[test]
    fn expr() {
        let mut diagnostics = Diagnostics::new();
        let tokens = Lexer::new(
            "let n = cases a {
    let => 1,
};",
            &mut diagnostics,
        )
        .tokenize();

        Parser::new(tokens, &mut diagnostics).parse();

        assert_eq!(
            diagnostics.errors,
            vec![Diagnostic::UnexpectedToken {
                expected: vec![Expected::Pattern],
                found: TokenKind::Let,
                span: Span { line: 2, col: 5 }
            }]
        );
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
let n = @;",
            &mut diagnostics,
        )
        .tokenize();

        Parser::new(tokens, &mut diagnostics).parse();

        assert_eq!(
            diagnostics.errors,
            vec![
                Diagnostic::InvalidNum {
                    lexeme: "9n".to_string(),
                    span: Span { line: 4, col: 5 }
                },
                Diagnostic::UnknownToken {
                    lexeme: "@".to_string(),
                    span: Span { line: 8, col: 9 }
                },
                Diagnostic::UnexpectedToken {
                    expected: vec![
                        Expected::Token(TokenKind::Let),
                        Expected::Token(TokenKind::Ent_t),
                        Expected::Token(TokenKind::Rel_t),
                        Expected::Token(TokenKind::NetToken),
                    ],
                    found: TokenKind::Ident("n".to_string()),
                    span: Span { line: 2, col: 1 }
                },
                Diagnostic::UnexpectedToken {
                    expected: vec![Expected::Ident],
                    found: TokenKind::ErrorToken,
                    span: Span { line: 4, col: 5 }
                },
                Diagnostic::UnexpectedToken {
                    expected: vec![Expected::Token(TokenKind::Semicolon)],
                    found: TokenKind::Let,
                    span: Span { line: 7, col: 1 }
                },
                Diagnostic::UnexpectedToken {
                    expected: vec![Expected::Expr],
                    found: TokenKind::ErrorToken,
                    span: Span { line: 8, col: 9 }
                },
            ]
        );
    }
}
