//! # network
//!
//! This module defines the network type which forms the basis of nhdl
//!
//! ## Invariants
//!
//! - Netwokrs must be made up of only relations and entities
//!
//! Author: Cole Francis
//!
//! Last Updated: 06/02/2026

use super::relation::Relation;
use super::entity::Entity;

pub struct Network<T, O> {
    pub relations: Vec<Relation<O>>,
    pub entities: Vec<Entity<T>>,
}

impl<T, O> Network<T, O> {
    pub fn new() -> Self {
        Self {
            relations: Vec::new(),
            entities: Vec::new(),
        }
    }
}