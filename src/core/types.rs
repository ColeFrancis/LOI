//! # types
//!
//! This module defines the core entity types of nhdl
//!
//! ## Invariants
//!
//! - Types must implement Copy, PartialEq, and Resettable
//!
//! Author: Cole Francis
//!
//! Last Updated: 06/04/2026

pub trait Resettable {
    fn reset() -> Self;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Logic {
    ON,
    OFF,
    X,
}

impl Resettable for Logic {
    fn reset() -> Self {
        Logic::X
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Real {
    Val(f64),
    X,
}

impl Resettable for Real {
    fn reset() -> Self {
        Real::X
    }
}

impl PartialEq for Real {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Real::X, Real::X) => true,
            _ => false,
        }
    }
}