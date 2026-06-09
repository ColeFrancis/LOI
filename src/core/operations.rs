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

use super::types::Logic;
use super::types::Real;

pub trait Operator<T> {
    fn eval(&self, a: T, b: T) -> T;
    fn name(&self) -> &'static str;
}

pub enum LogicOp {
    NAND,
    AND,
    XOR,
}

impl Operator<Logic> for LogicOp {
    fn eval(&self, a: Logic, b: Logic) -> Logic {
        match self {
            LogicOp::NAND => {
                match (a, b) {
                    (Logic::ON, Logic::ON) => Logic::OFF,
                    (Logic::OFF, _) | (_, Logic::OFF) => Logic::ON,
                    _ => Logic::X,
                }
            }
            LogicOp::AND => {
                match (a, b) {
                    (Logic::ON, Logic::ON) => Logic::ON,
                    (Logic::OFF, _) | (_, Logic::OFF) => Logic::OFF,
                    _ => Logic::X,
                }
            }
            LogicOp::XOR => {
                match (a, b) {
                    (Logic::ON, Logic::ON) | (Logic::OFF, Logic::OFF) => Logic::OFF,
                    (Logic::ON, Logic::OFF) | (Logic::OFF, Logic::ON) => Logic::ON,
                    _ => Logic::X,
                }
            }
        }
    }

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

impl Operator<Real> for RealOp {
    fn eval(&self, a: Real, b: Real) -> Real {
        match self {
            RealOp::ADD => {
                match (a, b) {
                    (Real::Val(val_a), Real::Val(val_b)) => Real::Val(val_a + val_b),
                    _ => Real::X,
                }
            },
            RealOp::MUL => {
                match (a, b) {
                    (Real::Val(val_a), Real::Val(val_b)) => Real::Val(val_a * val_b),
                    _ => Real::X,
                }
            }
        }
    }

    fn name (&self) -> &'static str {
        match self {
            RealOp::ADD => "ADD",
            RealOp::MUL => "MUL",
        }
    }
}
