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
use super::runtime_diagnostics::RuntimeError;

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

    pub fn load_inputs(&mut self, inputs: Vec<(String, Vec<(usize, u64)>)>) -> Result<(), RuntimeError> {
        for (ent, trace) in inputs {
            let ent_id = self.netlist.inputs
                .iter()
                .find(|(name, _)| name == &ent)
                .map(|(_, id)| *id)
                .ok_or_else(|| RuntimeError::NonexistantInput(ent))?;
            
            for (timestep, new_val) in trace {
                self.scheduler.push(Event {
                    timestep,
                    new_val,
                    ent_id,
                });
            }
        }

        Ok(())
    }

    // returns all the outputs in watcher at the end of the simulation, also clearing vectors in watcher
    pub fn dump_outputs(&mut self) -> Vec<(String, Vec<(usize, u64)>)> {
        let mut outputs = Vec::with_capacity(self.netlist.outputs.len());

        for (name, ent_id) in &self.netlist.outputs {
            if let Some(trace) = self.watcher.get_mut(ent_id) {
                outputs.push((name.clone(), std::mem::take(trace)));
            }
        }

        outputs
    }

    // returns the number of timesteps that were ran if no runtime errors, otherwise the error is returned
    pub fn run (&mut self, max_steps: usize) -> Result<usize, RuntimeError> {
        let mut rel_last_called = vec![0; self.netlist.relations.len()]; // make sure each relation called once
        let mut ent_last_driven = vec![0; self.netlist.ents.len()]; // make sure each entitiy only driven once

        while let Some(curr_events) = self.scheduler.pop() {
            let mut relations_to_call = Vec::new();

            for event in curr_events {
                println!("{:?}", event);
                // ensure entites aren't driven twice
                if ent_last_driven[event.ent_id] != self.scheduler.curr_time {
                    ent_last_driven[event.ent_id] = self.scheduler.curr_time;
                    self.netlist.ents[event.ent_id].val = Some(event.new_val);
                }
                else if self.netlist.ents[event.ent_id].val != Some(event.new_val) { // mutliple events with conflicting vals drive an ent is an error
                    return Err(RuntimeError::SimultaneousDrivers {
                        ent_id: event.ent_id,
                        timestep: self.scheduler.curr_time-1,
                    });
                }
                else {
                    println!("mutliple events, same val");
                }
                

                // Record output value change
                if let Some(updates) = self.watcher.get_mut(&event.ent_id) {
                    updates.push((self.scheduler.curr_time-1, event.new_val));
                }
                
                for rel_id in &self.netlist.ents[event.ent_id].sinks {
                    if rel_last_called[*rel_id] != self.scheduler.curr_time {
                        rel_last_called[*rel_id] = self.scheduler.curr_time;
                        relations_to_call.push(*rel_id);
                    }
                }
            }
            for rel_id in relations_to_call {
                let timestep = self.scheduler.curr_time;
                let compiled_rel_id = self.netlist.relations[rel_id].idx;
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
                let new_val = match self.interpreter.evaluate(compiled_rel_id, &args, timestep, delay) {
                    Ok(val) => val,
                    Err(err) => return Err(RuntimeError::Interpreter {
                        err,
                        rel_id,
                        timestep: self.scheduler.curr_time-1,
                    }),
                };
                                                                            
                if old_val != Some(new_val) {
                    self.scheduler.push(Event {
                        timestep: self.scheduler.curr_time-1 + delay, // -1 necessary because curr_time in scheduler increments after pop
                        ent_id: output_ent_id,
                        new_val: new_val,
                    });
                }
            }

            if self.scheduler.curr_time > max_steps {
                return Ok(self.scheduler.curr_time-1);
            }
        }

        Ok(self.scheduler.curr_time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::netlist::{Entity, Relation};
    use crate::simulator::rel_interpreter::test_assembler::assemble;
    use crate::simulator::runtime_diagnostics::InterpreterError;

    #[test]
    fn only_known_inputs() {
        // rel_t ADD : (a: Int, b: Int) -> Int = a + b;

        // net A {
        //     input a: Int;
        //     input b: Int;
        //     output q: Int;

        //     q := ADD(a, b);
        // }
        let netlist = Netlist {
            inputs: vec![
                ("a".to_string(), 0),
                ("b".to_string(), 1),
            ],
            outputs: vec![
                ("q".to_string(), 2),
            ],
            relations: vec![
                Relation {
                    idx: 0,
                    delay: 1,
                    input_ents: vec![0, 1],
                    output_ent: 2,
                },
            ],
            ents: vec![
                Entity {
                    val: None,
                    sinks: vec![0],
                },
                Entity {
                    val: None,
                    sinks: vec![0],
                },
                Entity {
                    val: None,
                    sinks: vec![],
                },
            ],
        };

        let relations = vec![
            CompiledRel {
                name: "ADD".to_string(),
                complexity: 0,
                bytecode: assemble("
                    IADD r4 r2 r3
                    RET r4
                ").unwrap(),
            },
        ];

        let inits = vec![];

        let mut sim = Simulator::new(netlist, relations, inits);

        let inputs = vec![
            ("a".to_string(), vec![
                (1, 1),
            ]),
            ("b".to_string(), vec![
                (2, 1),
            ]),
        ];

        let success = sim.load_inputs(inputs);

        assert_eq!(success, Ok(()));

        let _steps = sim.run(256);

        let output = sim.dump_outputs();

        assert_eq!(output, vec![
            ("q".to_string(), vec![
                (3 as usize, 2 as u64),
            ]),
        ]);
    }

    #[test]
    fn non_existant_input_error() {
        // net A {
        //     input a: Int;
        //     output a: Int;
        // }
        let netlist = Netlist {
            inputs: vec![
                ("a".to_string(), 0),
            ],
            outputs: vec![
                ("a".to_string(), 0),
            ],
            relations: vec![],
            ents: vec![
                Entity {
                    val: None,
                    sinks: vec![],
                },
            ],
        };

        let relations = vec![];

        let inits = vec![];

        let mut sim = Simulator::new(netlist, relations, inits);

        let inputs = vec![
            ("b".to_string(), vec![
                (2, 1),
            ]),
        ];

        let success = sim.load_inputs(inputs);

        assert_eq!(success, Err(RuntimeError::NonexistantInput("b".to_string())));
    }

    #[test]
    fn simultaneous_drivers_error() {
        // net A {
        //     input a: Int;
        //     output a: Int;
        // }
        let netlist = Netlist {
            inputs: vec![
                ("a".to_string(), 0),
            ],
            outputs: vec![
                ("a".to_string(), 0),
            ],
            relations: vec![],
            ents: vec![
                Entity {
                    val: None,
                    sinks: vec![],
                },
            ],
        };

        let relations = vec![];

        let inits = vec![];

        let mut sim = Simulator::new(netlist, relations, inits);

        let inputs = vec![
            ("a".to_string(), vec![
                (1, 1),
                (1, 2),
            ]),
        ];

        let success = sim.load_inputs(inputs);

        assert_eq!(success, Ok(()));

        let result = sim.run(10);

        assert_eq!(result, Err(RuntimeError::SimultaneousDrivers{
            ent_id: 0,
            timestep: 1,
        }));
    }

    #[test]
    fn interpreter_error() {
        // rel_t DIV : (a: Int, b: Int) -> Real = a/b;

        // net A {
        //     input a: Real;
        //     input b: Real;
        //     output q: Real;

        //     q := DIV(a, b);
        // }
        let netlist = Netlist {
            inputs: vec![
                ("a".to_string(), 0),
                ("b".to_string(), 1),
            ],
            outputs: vec![
                ("q".to_string(), 2),
            ],
            relations: vec![
                Relation {
                    idx: 0,
                    delay: 1,
                    input_ents: vec![0, 1],
                    output_ent: 2,
                },
            ],
            ents: vec![
                Entity {
                    val: None,
                    sinks: vec![0],
                },
                Entity {
                    val: None,
                    sinks: vec![0],
                },
                Entity {
                    val: None,
                    sinks: vec![],
                },
            ],
        };

        let relations = vec![
            CompiledRel {
                name: "DIV".to_string(),
                complexity: 0,
                bytecode: assemble("
                    IDIV r4 r2 r3
                    RET r4
                ").unwrap(),
            },
        ];

        let inits = vec![];

        let mut sim = Simulator::new(netlist, relations, inits);

        let inputs = vec![
            ("a".to_string(), vec![
                (1, 1),
            ]),
            ("b".to_string(), vec![
                (2, 0),
            ]),
        ];

        let success = sim.load_inputs(inputs);

        assert_eq!(success, Ok(()));

        let result = sim.run(256);

        assert_eq!(result, Err(RuntimeError::Interpreter {
            err: InterpreterError::DivisionByZero,
            rel_id: 0,
            timestep: 2,
        }));
    }

    #[test]
    fn max_steps() {
        // net A {
        //     input a: Int;
        //     output a: Int;
        // }
        let netlist = Netlist {
            inputs: vec![
                ("a".to_string(), 0),
            ],
            outputs: vec![
                ("a".to_string(), 0),
            ],
            relations: vec![],
            ents: vec![
                Entity {
                    val: None,
                    sinks: vec![],
                },
            ],
        };

        let relations = vec![];

        let inits = vec![];

        let mut sim = Simulator::new(netlist, relations, inits);

        let inputs = vec![
            ("a".to_string(), vec![
                (5, 1),
            ]),
        ];

        let success = sim.load_inputs(inputs);

        assert_eq!(success, Ok(()));

        let result = sim.run(10);

        assert_eq!(result, Ok(6));
    }

    #[test]
    fn finish_early() {
        // net A {
        //     input a: Int;
        //     output a: Int;
        // }
        let netlist = Netlist {
            inputs: vec![
                ("a".to_string(), 0),
            ],
            outputs: vec![
                ("a".to_string(), 0),
            ],
            relations: vec![],
            ents: vec![
                Entity {
                    val: None,
                    sinks: vec![],
                },
            ],
        };

        let relations = vec![];

        let inits = vec![];

        let mut sim = Simulator::new(netlist, relations, inits);

        let inputs = vec![
            ("a".to_string(), vec![
                (50, 1),
            ]),
        ];

        let success = sim.load_inputs(inputs);

        assert_eq!(success, Ok(()));

        let result = sim.run(10);

        assert_eq!(result, Ok(10));
    }

    #[test]
    fn nand_vs_and_and_not() {
        let netlist = Netlist {
            inputs: vec![
                ("a".to_string(), 0),
                ("b".to_string(), 1),
            ],
            outputs: vec![
                ("q".to_string(), 2),
            ],
            relations: vec![
                Relation {
                    idx: 0,
                    delay: 2,
                    input_ents: vec![0, 1],
                    output_ent: 2,
                },
            ],
            ents: vec![
                Entity {
                    val: None,
                    sinks: vec![0],
                },
                Entity {
                    val: None,
                    sinks: vec![0],
                },
                Entity {
                    val: None,
                    sinks: vec![],
                },
            ],
        };

        let relations = vec![
            CompiledRel {
                name: "NAND".to_string(),
                complexity: 0,
                bytecode: assemble("
                    AND r4 r2 r3
                    NOT r4 r4
                    RET r4
                ").unwrap(),
            },
        ];

        let inits = vec![];

        let mut sim = Simulator::new(netlist, relations, inits);

        let inputs = vec![
            ("a".to_string(), vec![
                (1, 1),
                (5, 1),
                (9, 0),
                (13, 0),
            ]),
            ("b".to_string(), vec![
                (1, 1),
                (5, 0),
                (9, 1),
                (13, 0),
            ]),
        ];

        let success = sim.load_inputs(inputs);

        assert_eq!(success, Ok(()));

        let _steps = sim.run(256);

        let output_1 = sim.dump_outputs();

        let netlist = Netlist {
            inputs: vec![
                ("a".to_string(), 0),
                ("b".to_string(), 1),
            ],
            outputs: vec![
                ("q".to_string(), 2),
            ],
            relations: vec![
                Relation {
                    idx: 0,
                    delay: 1,
                    input_ents: vec![0, 1],
                    output_ent: 3,
                },
                Relation {
                    idx: 1,
                    delay: 1,
                    input_ents: vec![3],
                    output_ent: 2,
                },
            ],
            ents: vec![
                Entity {
                    val: None,
                    sinks: vec![0],
                },
                Entity {
                    val: None,
                    sinks: vec![0],
                },
                Entity {
                    val: None,
                    sinks: vec![],
                },
                Entity {
                    val: None,
                    sinks: vec![1]
                }
            ],
        };

        let relations = vec![
            CompiledRel {
                name: "AND".to_string(),
                complexity: 0,
                bytecode: assemble("
                    AND r4 r2 r3
                    RET r4
                ").unwrap(),
            },
            CompiledRel {
                name: "NOT".to_string(),
                complexity: 0,
                bytecode: assemble("
                    NOT r4 r3
                    RET r4
                ").unwrap(),
            },
        ];

        let inits = vec![];

        let mut sim = Simulator::new(netlist, relations, inits);

        let inputs = vec![
            ("a".to_string(), vec![
                (1, 1),
                (5, 1),
                (9, 0),
                (13, 0),
            ]),
            ("b".to_string(), vec![
                (1, 1),
                (5, 0),
                (9, 1),
                (13, 0),
            ]),
        ];

        let success = sim.load_inputs(inputs);

        assert_eq!(success, Ok(()));

        let _steps = sim.run(256);

        let output_2 = sim.dump_outputs();

        assert_eq!(output_1, output_2);
    }

    #[test]
    fn xor_with_nand() {
        // rel_t NAND : (a: Bool, b: Bool) -> Bool = ~(a & b);

        // net XOR {
        //     input a: Bool;
        //     input b: Bool;
        //     output q: Bool;

        //     net_3 := NAND(a, b);

        //     net_4 := NAND(a, net_3);

        //     net_5 := NAND(b, net_3);

        //     q := NAND(net_4, net_5);
        // }
        let netlist = Netlist {
            inputs: vec![
                ("a".to_string(), 0),
                ("b".to_string(), 1),
            ],
            outputs: vec![
                ("q".to_string(), 2),
            ],
            relations: vec![
                Relation {
                    idx: 0,
                    delay: 1,
                    input_ents: vec![0, 1],
                    output_ent: 3,
                },
                Relation {
                    idx: 0,
                    delay: 1,
                    input_ents: vec![0, 3],
                    output_ent: 4,
                },
                Relation {
                    idx: 0,
                    delay: 1,
                    input_ents: vec![1, 3],
                    output_ent: 5,
                },
                Relation {
                    idx: 0,
                    delay: 1,
                    input_ents: vec![4, 5],
                    output_ent: 2,
                },
            ],
            ents: vec![
                Entity {
                    val: None,
                    sinks: vec![0, 1],
                },
                Entity {
                    val: None,
                    sinks: vec![0, 2],
                },
                Entity {
                    val: None,
                    sinks: vec![],
                },
                Entity {
                    val: None,
                    sinks: vec![1, 2],
                },
                Entity {
                    val: None,
                    sinks: vec![3],
                },
                Entity {
                    val: None,
                    sinks: vec![3],
                },
            ],
        };

        let relations = vec![
            CompiledRel {
                name: "NAND".to_string(),
                complexity: 0,
                bytecode: assemble("
                    AND r4 r2 r3
                    NOT r4 r4
                    RET r4
                ").unwrap(),
            },
        ];

        let inits = vec![];

        let mut sim = Simulator::new(netlist, relations, inits);

        let inputs = vec![
            ("a".to_string(), vec![
                (1, 0),
                (5, 1),
            ]),
            ("b".to_string(), vec![
                (1, 0),
                (9, 1),
            ]),
        ];

        let success = sim.load_inputs(inputs);

        assert_eq!(success, Ok(()));

        let _steps = sim.run(256);

        let output = sim.dump_outputs();

        assert_eq!(output, vec![
            ("q".to_string(), vec![
                (4 as usize, 0 as u64),
                (7 as usize, 1 as u64),
                (12 as usize, 0 as u64),
            ]),
        ]);
    }

    #[test]
    fn integrator() {
        // rel_t ADD_1 : (a: Int) -> Int = a + 1;

        // net COUNTER {
        //     output a: Int;

        //     init a: Int = 0;

        //     a := ADD_1(a);
        // }
        let netlist = Netlist {
            inputs: vec![],
            outputs: vec![
                ("a".to_string(), 0),
            ],
            relations: vec![
                Relation {
                    idx: 0,
                    delay: 1,
                    input_ents: vec![0],
                    output_ent: 0,
                },
            ],
            ents: vec![
                Entity {
                    val: None,
                    sinks: vec![0],
                },
            ],
        };

        let relations = vec![
            CompiledRel {
                name: "NAND".to_string(),
                complexity: 0,
                bytecode: assemble("
                    IADD r4 r2 i1
                    RET r4
                ").unwrap(),
            },
        ];

        let inits = vec![
            Event {
                timestep: 0,
                ent_id: 0,
                new_val: 0,
            },
        ];

        let mut sim = Simulator::new(netlist, relations, inits);

        let _steps = sim.run(5);

        let output = sim.dump_outputs();

        assert_eq!(output, vec![
            ("a".to_string(), vec![
                (0 as usize, 0 as u64),
                (1 as usize, 1 as u64),
                (2 as usize, 2 as u64),
                (3 as usize, 3 as u64),
                (4 as usize, 4 as u64),
                (5 as usize, 5 as u64),
            ]),
        ]);
    }
}