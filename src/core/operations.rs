//! # operations
//!
//! This module defines the operations for each types relations
//!
//! ## Invariants
//!
//! - operations must be binary (for now)
//!
//! Author: Cole Francis
//!
//! Last Updated: 06/06/2026

pub enum LogicOp {
    NAND,
    AND,
    XOR,
}

impl LogicOp {
    fn name (&self) -> &'static str {
        match self {
            LogicOp::NAND => "NAND",
            LogicOp::AND => "AND",
            LogicOp::XOR => "XOR",
        }
    }
}

pub enum RealOp {
    ADD,
    MUL,
}

impl RealOp {
    fn name (&self) -> &'static str {
        match self {
            RealOp::ADD => "ADD",
            RealOp::MUL => "MUL",
        }
    }
}