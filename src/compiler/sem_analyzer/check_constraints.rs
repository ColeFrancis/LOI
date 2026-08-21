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

//! # check_constraints
//!
//! Handles final error checking before the compiler enters backend stages
//!
//! ## Invariants
//!
//! - After this point, all errors catchable by the front-end will be caught
//!
//! Author: Cole Francis

use std::collections::HashSet;

use super::SemAnalyzer;

use crate::compiler::{
    ast::*,
    symbol::SymbolKind,
    diagnostics::{CompilerError, Span},
};

impl <'a> SemAnalyzer<'a> {
    pub(super) fn check_constraints(&mut self) {
        let items = std::mem::take(&mut self.ast.items);
        self.ast.items = Vec::with_capacity(items.len());

        for item in items {
            let checked_item = self.check_constraints_item(item).unwrap_or(Item::Error);
            self.ast.items.push(checked_item);
        }
    }

    fn check_constraints_item(&mut self, item: Item) -> Option<Item> {
        match item {
            Item::Net(net) => self.check_constraints_net(net).map(Item::Net),
            
            other => Some(other),
        }
    }

    // TODO: reason more about if I need to check more than just multiple driver errors
    fn check_constraints_net(&mut self, mut net: Net) -> Option<Net> { 
        let mut has_errors = false; 
        let mut driven_ents = HashSet::new();

        for item in &net.items {
            match item {
                NetItem::Input(input_ent) => {
                    let Ident::Symbol(symbol_id) = input_ent.param.name else {
                        return None; // Not reachable
                    };

                    if !driven_ents.insert(symbol_id) {
                        self.diagnostics.error(CompilerError::MultipleEntDrivers {
                            name: self.symbols[symbol_id].name.to_string(),
                            first_span: Span{line: 0, col: 0},
                            last_span: Span{line: 0, col: 0},                            // TODO: Figure out span
                        });

                        has_errors = true;
                    }
                }

                NetItem::RelInst(rel_inst) => {
                    let Ident::Symbol(asignee_symbol_id) = rel_inst.asignee else {
                        return None; // Not reachable
                    };

                    if !driven_ents.insert(asignee_symbol_id) {
                        self.diagnostics.error(CompilerError::MultipleEntDrivers {
                            name: self.symbols[asignee_symbol_id].name.to_string(),
                            first_span: Span{line: 0, col: 0},
                            last_span: rel_inst.span.clone(),                            // TODO: Figure out span
                        });

                        has_errors = true;
                    }
                }

                NetItem::NetInst(net_inst) => {
                    let Ident::Symbol(net_symbol_id) = net_inst.net else {
                        return None; // not reachable
                    };

                    let SymbolKind::Net{ports} = &self.symbols[net_symbol_id].kind else {
                        return None; // not reachable
                    };

                    for connection in &net_inst.connections {
                        let Ident::Symbol(port_symbol_id) = connection.port else {
                            return None; // not reachable
                        };

                        let Some(net_port) = ports.get(&self.symbols[port_symbol_id].name) else {
                            return None; // not reachable
                        };

                        if !net_port.input {
                            let Ident::Symbol(ent_symbol_id) = connection.ent else {
                                return None; // not reachable
                            };

                            if !driven_ents.insert(ent_symbol_id) {
                                self.diagnostics.error(CompilerError::MultipleEntDrivers {
                                    name: self.symbols[ent_symbol_id].name.to_string(),
                                    first_span: Span{line: 0, col: 0},
                                    last_span: connection.span.clone(),                            // TODO: Figure out span
                                });

                                has_errors = true;
                            }
                        }
                    }
                }

                _ => {}
            }
        }

        if has_errors {
            None
        }
        else {
            Some(net)
        }
    }
}