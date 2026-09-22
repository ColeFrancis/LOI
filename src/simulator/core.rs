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

//! # core
//!
//! simulates netlists
//!
//! ## Invariants
//!
//! Author: Cole Francis

use std::collections::HashMap;

use super::Simulator;
use super::scheduler::Scheduler;
use super::rel_interpreter::RelInterpreter;
use super::event::Event;

use crate::compiler::{
    netlist::{Netlist, EntId},
    compiled_rel::CompiledRel,
};

impl Simulator {
    pub fn new(netlist: Netlist, relations: Vec<CompiledRel>, inits: Vec<Event>) -> Self {
        let mut watcher = HashMap::<EntId, Vec<(usize, u64)>>::new();

        for (_, output_ent_id) in &netlist.outputs {
            watcher.insert(*output_ent_id, Vec::new());
        }

        Self {
            netlist,
            scheduler: Scheduler::new(inits),
            interpreter: RelInterpreter::new(relations),
            watcher,
        }
    }

    pub fn load_inputs(&mut self, inputs: Vec<Event>) {
        for event in inputs {
            self.scheduler.push(event);
        }
    }

    // returns all the outputs in watcher at the end of the simulation
    // pub fn dump_outputs(&mut self) ->

    // Returns none if the simulator reached max steps. Else if the simulator runs N timesteps it returns Some(N)
    pub fn run (&mut self, max_steps: usize) -> Option<usize> {
        let mut rel_last_called = vec![0; self.netlist.relations.len()]; // make sure each relation called once
        let mut ent_last_driven = vec![0; self.netlist.ents.len()]; // make sure each entitiy only driven once

        // initialized to 1 to use with rel_last_called
        let mut step: usize = 1;

        while let Some(curr_events) = self.scheduler.pop() {
            let mut relations_to_call = Vec::new();

            for event in curr_events {
                // ensure entites aren't driven twice
                if ent_last_driven[event.entity] != step {
                    ent_last_driven[event.entity] = step;
                    self.netlist.ents[event.entity].val = Some(event.new_val);
                }
                else {
                                                                                     // TODO: call runtime error
                    return None;
                }
                

                // Record output value change
                if let Some(updates) = self.watcher.get_mut(&event.entity) {
                    updates.push((self.scheduler.curr_time, event.new_val));
                }
                
                for rel_id in &self.netlist.ents[event.entity].sinks {
                    if rel_last_called[*rel_id] != step {
                        rel_last_called[*rel_id] = step;
                        relations_to_call.push(*rel_id);
                    }
                }
            }
            for rel_id in relations_to_call {
                let timestep = self.scheduler.curr_time;
                let delay = self.netlist.relations[rel_id].delay;
                let output_ent_id = self.netlist.relations[rel_id].output_ent;

                let args: Option<Vec<u64>> = self.netlist.relations[rel_id]
                    .input_ents
                    .iter()
                    .map(|&value| self.netlist.ents[value].val)
                    .collect();

                // dont execute relation if any inputs are none;
                let Some(args) = args else {
                    continue;
                };

                let old_val = self.netlist.ents[output_ent_id].val;
                let new_val = self.interpreter.evaluate(rel_id, &args, timestep, delay);
                                                                                        // TODO: Handling of runtime errors

                if old_val != Some(new_val) {
                    self.scheduler.push(Event {
                        timestep: self.scheduler.curr_time + delay,
                        entity: output_ent_id,
                        new_val: new_val,
                    });
                }
            }

            step += 1;
            if step > max_steps {
                return None;
            }
        }

        Some(step - 1)
    }
}

// TODO: test that relations are only executed if no inputs are None