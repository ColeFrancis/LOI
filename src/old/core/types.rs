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

//! # types
//!
//! This module defines the core entity types of nhdl
//!
//! ## Invariants
//!
//! - Types must implement Copy, PartialEq, and Resettable
//!
//! Author: Cole Francis

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