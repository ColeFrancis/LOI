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

mod core;
pub mod event;
mod scheduler;
pub mod rel_interpreter;
pub mod runtime_diagnostics;
pub mod file_io;

use std::collections::HashMap;

use crate::simulator::{
    scheduler::Scheduler,
    rel_interpreter::RelInterpreter,
};

use crate::compiler::netlist::{Netlist, Interface, EntId};

#[derive(PartialEq, Debug, Clone)]
pub enum IoVal {
    Bool(bool),
    Int(i64),
    Real(f64),
    Custom(String),
}

impl std::fmt::Display for IoVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IoVal::Bool(value) => write!(f, "{value}"),
            IoVal::Int(value) => write!(f, "{value}"),
            IoVal::Real(value) if value.fract() == 0.0 => write!(f, "{value:.1}"),
            IoVal::Real(value) => write!(f, "{value}"),
            IoVal::Custom(value) => write!(f, "{value}"),
        }
    }
}

pub struct Simulator {
    netlist: Netlist,
    interface: Interface,
    scheduler: Scheduler,
    interpreter: RelInterpreter,
    watcher: HashMap<EntId, Vec<(usize, u64)>>,
}