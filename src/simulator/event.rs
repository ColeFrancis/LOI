//! # event
//!
//! This module defines the event type for use in the sim module
//!
//! ## Invariants
//!
//! - Events must contain a timestamp and an id for the entity they affect
//!
//! Author: Cole Francis
//!
//! Last Updated: 06/02/2026

use crate::network::entity::EntityId;

pub struct Event<T> {
    pub time: usize,
    pub entity: EntityId,
    pub new_value: T,
}