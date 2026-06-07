//! # entity
//!
//! This module defines the entity type which form the edges of a network
//!
//! ## Invariants
//!
//! - Entity must contain a value field for its current state and a list of
//! the relations it attaches to
//!
//! Author: Cole Francis
//!
//! Last Updated: 06/02/2026

use super::relation::RelationId;

pub type EntityId = usize;

pub struct Entity<T> {
    pub value: T,
    pub sinks: Vec<RelationId>,
}