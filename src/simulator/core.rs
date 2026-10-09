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
//! - Relation outputs are only registered as events if the value changes
//! - Deterministic relations are only evaluated if one or more inputs change
//! - Nondetrministic relations are evaluated every simulation timestep
//!
//! Author: Cole Francis

use std::collections::HashMap;

use super::{
    IoVal, Simulator,
    error_handling::runtime_diagnostics::RuntimeError,
    objects::{event::Event, scheduler::Scheduler},
    rel_interpreter::RelInterpreter,
};

use crate::compiler::objects::{
    ast::Ident,
    compiled_rel::CompiledRel,
    netlist::{EntId, Interface, Netlist},
    types::Type,
};

impl Simulator {
    pub fn new(
        netlist: Netlist,
        interface: Interface,
        compiled_relations: Vec<CompiledRel>,
        inits: Vec<Event>,
    ) -> Self {
        let mut watcher = HashMap::<EntId, Vec<(usize, u64)>>::new();
        let mut nondeterministic_rel_ids = Vec::new();

        for output_ent_id in interface.outputs.keys() {
            watcher.insert(*output_ent_id, Vec::new());
        }

        for (id, relation) in netlist.relations.iter().enumerate() {
            if !compiled_relations[relation.idx].deterministic {
                nondeterministic_rel_ids.push(id);
            }
        }

        Self {
            netlist,
            interface,
            scheduler: Scheduler::new(inits),
            interpreter: RelInterpreter::new(compiled_relations),
            watcher,
            nondeterministic_rel_ids,
        }
    }

    pub fn load_inputs(
        &mut self,
        inputs: Vec<(String, Vec<(usize, IoVal)>)>,
    ) -> Result<(), RuntimeError> {
        for (ent_name, trace) in inputs {
            let (ent_id, ent_type) = self
                .interface
                .inputs
                .get(&ent_name)
                .ok_or_else(|| RuntimeError::NonexistantInput(ent_name))?;

            for (timestep, io_val) in trace {
                let new_val = match (&io_val, ent_type) {
                    (IoVal::Bool(b), Type::Bool) => *b as u64,
                    (IoVal::Bool(b), Type::Impulse) => match b {
                        true => timestep as u64,
                        false if timestep == 0 => u64::MAX, // 0 is the default value when needing to set impulse to 0, except for events at timestep 0 (like inits)
                        _ => 0 as u64,
                    },

                    (IoVal::Int(i), Type::Int) => *i as u64,
                    (IoVal::Int(i), Type::Float) => (*i as f64).to_bits(),

                    (IoVal::Float(f), Type::Float) => f.to_bits(),

                    (IoVal::Custom(given_str), Type::Custom(Ident::Str { val, .. })) => self
                        .interface
                        .custom_type_maps
                        .get(val)
                        .and_then(|members| {
                            members
                                .iter()
                                .find(|(member, _)| member == given_str)
                                .map(|(_, mapping)| *mapping)
                        })
                        .ok_or_else(|| RuntimeError::NonexistantInputValue(given_str.clone()))?,

                    _ => {
                        let found = match io_val {
                            IoVal::Bool(_) => Type::Bool,
                            IoVal::Int(_) => Type::Int,
                            IoVal::Float(_) => Type::Float,
                            IoVal::Custom(_) => Type::Unknown,
                        };

                        return Err(RuntimeError::IncompatibleTypes {
                            ent_id: *ent_id,
                            expected: ent_type.clone(),
                            found,
                        });
                    }
                };

                self.scheduler.push(Event {
                    timestep,
                    new_val,
                    ent_id: *ent_id,
                });
            }
        }

        Ok(())
    }

    // returns all the outputs in watcher at the end of the simulation, also clearing vectors in watcher
    pub fn dump_outputs(&mut self) -> Vec<(String, Vec<(usize, IoVal)>)> {
        let mut outputs = Vec::with_capacity(self.interface.outputs.len());

        for (ent_id, (ent_name, ent_type)) in &self.interface.outputs {
            if let Some(trace) = self.watcher.get_mut(&ent_id) {
                let trace = std::mem::take(trace);
                let mut new_trace = Vec::new();

                for (timestep, raw_val) in trace {
                    let io_val = match ent_type {
                        Type::Bool => IoVal::Bool(raw_val != 0),
                        Type::Impulse if raw_val == timestep as u64 => IoVal::Bool(true),
                        Type::Impulse if raw_val != timestep as u64 => continue,
                        Type::Int => IoVal::Int(raw_val as i64),
                        Type::Float => IoVal::Float(f64::from_bits(raw_val)),
                        Type::Mod(_) => IoVal::Int(raw_val as i64),
                        Type::Custom(Ident::Str { val, .. }) => {
                            let name = self
                                .interface
                                .custom_type_maps
                                .get(val)
                                .and_then(|members| {
                                    members
                                        .iter()
                                        .find(|(_, mapping)| mapping == &raw_val)
                                        .map(|(member, _)| member)
                                })
                                .unwrap();

                            IoVal::Custom(name.to_string())
                        }
                        _ => unreachable!("Type should not be any other"),
                    };

                    new_trace.push((timestep, io_val));
                }
                outputs.push((ent_name.clone(), new_trace));
            }
        }

        // Sort because iterating thrugh hashmap isnt guarenteed to be in order
        // Maybe we shouldn't care about order
        outputs.sort_by_key(|(name, _)| {
            self.interface
                .outputs
                .iter()
                .find(|(_, (ent_name, _))| ent_name == name)
                .map(|(id, _)| *id)
                .unwrap()
        });

        outputs
    }

    // returns the number of timesteps that were ran if no runtime errors, otherwise the error is returned
    pub fn run(
        &mut self,
        max_steps: usize,
        stop_on_next_output: bool,
    ) -> Result<usize, RuntimeError> {
        let mut stopping = false;
        let mut rel_last_called = vec![0; self.netlist.relations.len()]; // make sure each relation called once
        let mut ent_last_driven = vec![0; self.netlist.ents.len()]; // make sure each entitiy only driven once

        // If there are nondeterministic relations, 
        while self.scheduler.has_events() || !self.nondeterministic_rel_ids.is_empty() {
            let curr_events = self.scheduler.pop();
            
            let mut relations_to_call = Vec::new();

            for event in curr_events {
                // ensure entites aren't driven twice
                if ent_last_driven[event.ent_id] != self.scheduler.curr_time {
                    ent_last_driven[event.ent_id] = self.scheduler.curr_time;
                    self.netlist.ents[event.ent_id].val = Some(event.new_val);
                } else if self.netlist.ents[event.ent_id].val != Some(event.new_val) {
                    // mutliple events with conflicting vals drive an ent is an error
                    return Err(RuntimeError::SimultaneousDrivers {
                        ent_id: event.ent_id,
                        timestep: self.scheduler.curr_time - 1,
                    });
                }

                // Record output value change
                if let Some(updates) = self.watcher.get_mut(&event.ent_id) {
                    updates.push((self.scheduler.curr_time - 1, event.new_val));

                    if stop_on_next_output {
                        stopping = true;
                    }
                }

                // Schedule relations driven by this entity
                for &rel_id in &self.netlist.ents[event.ent_id].sinks {
                    if rel_last_called[rel_id] != self.scheduler.curr_time {
                        rel_last_called[rel_id] = self.scheduler.curr_time;
                        relations_to_call.push(rel_id);
                    }
                }
            }
            // Schedule all nondeterministic relations
            for &rel_id in &self.nondeterministic_rel_ids {
                if rel_last_called[rel_id] != self.scheduler.curr_time {
                    rel_last_called[rel_id] = self.scheduler.curr_time;
                    relations_to_call.push(rel_id);
                }
            }
            // Evaluate needed relations
            for rel_id in relations_to_call {
                let timestep = self.scheduler.curr_time - 1;
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
                let new_val =
                    match self
                        .interpreter
                        .evaluate(compiled_rel_id, &args, timestep, delay)
                    {
                        Ok(val) => val,
                        Err(err) => {
                            return Err(RuntimeError::Interpreter {
                                err,
                                rel_name: self.interpreter.get_rel_name(compiled_rel_id),
                                rel_id,
                                timestep,
                            });
                        }
                    };

                if old_val != Some(new_val) {
                    self.scheduler.push(Event {
                        timestep: timestep + delay, // -1 necessary because curr_time in scheduler increments after pop
                        ent_id: output_ent_id,
                        new_val: new_val,
                    });
                }
            }

            if self.scheduler.curr_time > max_steps || stopping {
                return Ok(self.scheduler.curr_time - 1);
            }
        }

        Ok(self.scheduler.curr_time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::error_handling::span::Span;
    use crate::compiler::objects::netlist::{Entity, Relation};
    use crate::simulator::error_handling::runtime_diagnostics::InterpreterError;
    use crate::simulator::rel_interpreter::test_assembler::assemble;

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
            relations: vec![Relation {
                idx: 0,
                delay: 1,
                input_ents: vec![0, 1],
                output_ent: 2,
            }],
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
        let interface = Interface {
            inputs: HashMap::from([
                ("a".to_string(), (0, Type::Int)),
                ("b".to_string(), (1, Type::Int)),
            ]),
            outputs: HashMap::from([(2, ("q".to_string(), Type::Int))]),
            custom_type_maps: HashMap::new(),
        };

        let relations = vec![CompiledRel {
            name: "ADD".to_string(),
            complexity: 0,
            deterministic: true,
            bytecode: assemble(
                "
                    IADD r4 r2 r3
                    RET r4
                ",
            )
            .unwrap(),
        }];

        let inits = vec![];

        let mut sim = Simulator::new(netlist, interface, relations, inits);

        let inputs = vec![
            ("a".to_string(), vec![(1, IoVal::Int(1))]),
            ("b".to_string(), vec![(2, IoVal::Int(1))]),
        ];

        let success = sim.load_inputs(inputs);

        assert_eq!(success, Ok(()));

        let _steps = sim.run(256, false);

        let output = sim.dump_outputs();

        assert_eq!(
            output,
            vec![("q".to_string(), vec![(3 as usize, IoVal::Int(2)),]),]
        );
    }

    #[test]
    fn non_existant_input() {
        // net A {
        //     input a: Int;
        //     output a: Int;
        // }
        let netlist = Netlist {
            relations: vec![],
            ents: vec![Entity {
                val: None,
                sinks: vec![],
            }],
        };
        let interface = Interface {
            inputs: HashMap::from([("a".to_string(), (0, Type::Int))]),
            outputs: HashMap::from([(0, ("a".to_string(), Type::Int))]),
            custom_type_maps: HashMap::new(),
        };

        let relations = vec![];

        let inits = vec![];

        let mut sim = Simulator::new(netlist, interface, relations, inits);

        let inputs = vec![("b".to_string(), vec![(2, IoVal::Int(1))])];

        let success = sim.load_inputs(inputs);

        assert_eq!(
            success,
            Err(RuntimeError::NonexistantInput("b".to_string()))
        );
    }

    #[test]
    fn type_conversion() {
        // net A {
        //     input a: Float;
        //     output a: Float;
        // }
        let netlist = Netlist {
            relations: vec![],
            ents: vec![Entity {
                val: None,
                sinks: vec![],
            }],
        };
        let interface = Interface {
            inputs: HashMap::from([("a".to_string(), (0, Type::Float))]),
            outputs: HashMap::from([(0, ("a".to_string(), Type::Float))]),
            custom_type_maps: HashMap::new(),
        };

        let relations = vec![];

        let inits = vec![];

        let mut sim = Simulator::new(netlist, interface, relations, inits);

        let inputs = vec![("a".to_string(), vec![(2, IoVal::Int(1))])];

        let success = sim.load_inputs(inputs);

        assert_eq!(success, Ok(()));

        let _steps = sim.run(5, false);

        let output = sim.dump_outputs();

        assert_eq!(
            output,
            vec![("a".to_string(), vec![(2, IoVal::Float(1.0)),]),]
        );
    }

    #[test]
    fn incorrect_input_type() {
        // net A {
        //     input a: Int;
        //     output a: Int;
        // }
        let netlist = Netlist {
            relations: vec![],
            ents: vec![Entity {
                val: None,
                sinks: vec![],
            }],
        };
        let interface = Interface {
            inputs: HashMap::from([("a".to_string(), (0, Type::Int))]),
            outputs: HashMap::from([(0, ("a".to_string(), Type::Int))]),
            custom_type_maps: HashMap::new(),
        };

        let relations = vec![];

        let inits = vec![];

        let mut sim = Simulator::new(netlist, interface, relations, inits);

        let inputs = vec![("a".to_string(), vec![(2, IoVal::Float(1.0))])];

        let success = sim.load_inputs(inputs);

        assert_eq!(
            success,
            Err(RuntimeError::IncompatibleTypes {
                ent_id: 0,
                expected: Type::Int,
                found: Type::Float,
            })
        );
    }

    #[test]
    fn nonexistant_input_value() {
        // ent_t TEST = {VAL1};
        // net A {
        //     input a: TEST;
        //     output a: TEST;
        // }
        let netlist = Netlist {
            relations: vec![],
            ents: vec![Entity {
                val: None,
                sinks: vec![],
            }],
        };
        let interface = Interface {
            inputs: HashMap::from([(
                "a".to_string(),
                (
                    0,
                    Type::Custom(Ident::Str {
                        val: "TEST".to_string(),
                        span: Span { line: 0, col: 0 },
                    }),
                ),
            )]),
            outputs: HashMap::from([(
                0,
                (
                    "a".to_string(),
                    Type::Custom(Ident::Str {
                        val: "TEST".to_string(),
                        span: Span { line: 0, col: 0 },
                    }),
                ),
            )]),
            custom_type_maps: HashMap::from([("TEST".to_string(), vec![("VAL1".to_string(), 0)])]),
        };

        let relations = vec![];

        let inits = vec![];

        let mut sim = Simulator::new(netlist, interface, relations, inits);

        let inputs = vec![(
            "a".to_string(),
            vec![(2, IoVal::Custom("VAL2".to_string()))],
        )];

        let success = sim.load_inputs(inputs);

        assert_eq!(
            success,
            Err(RuntimeError::NonexistantInputValue("VAL2".to_string()))
        );
    }

    #[test]
    fn simultaneous_drivers_error() {
        // net A {
        //     input a: Int;
        //     output a: Int;
        // }
        let netlist = Netlist {
            relations: vec![],
            ents: vec![Entity {
                val: None,
                sinks: vec![],
            }],
        };
        let interface = Interface {
            inputs: HashMap::from([("a".to_string(), (0, Type::Int))]),
            outputs: HashMap::from([(0, ("a".to_string(), Type::Int))]),
            custom_type_maps: HashMap::new(),
        };

        let relations = vec![];

        let inits = vec![];

        let mut sim = Simulator::new(netlist, interface, relations, inits);

        let inputs = vec![(
            "a".to_string(),
            vec![(1, IoVal::Int(1)), (1, IoVal::Int(2))],
        )];

        let success = sim.load_inputs(inputs);

        assert_eq!(success, Ok(()));

        let result = sim.run(10, false);

        assert_eq!(
            result,
            Err(RuntimeError::SimultaneousDrivers {
                ent_id: 0,
                timestep: 1,
            })
        );
    }

    #[test]
    fn interpreter_error() {
        // rel_t DIV : (a: Int, b: Int) -> Int = a/b;

        // net A {
        //     input a: Int;
        //     input b: Int;
        //     output q: Int;

        //     q := DIV(a, b);
        // }
        let netlist = Netlist {
            relations: vec![Relation {
                idx: 0,
                delay: 1,
                input_ents: vec![0, 1],
                output_ent: 2,
            }],
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
        let interface = Interface {
            inputs: HashMap::from([
                ("a".to_string(), (0, Type::Int)),
                ("b".to_string(), (1, Type::Int)),
            ]),
            outputs: HashMap::from([(2, ("q".to_string(), Type::Int))]),
            custom_type_maps: HashMap::new(),
        };

        let relations = vec![CompiledRel {
            name: "DIV".to_string(),
            complexity: 0,
            deterministic: true,
            bytecode: assemble(
                "
                    IDIV r4 r2 r3
                    RET r4
                ",
            )
            .unwrap(),
        }];

        let inits = vec![];

        let mut sim = Simulator::new(netlist, interface, relations, inits);

        let inputs = vec![
            ("a".to_string(), vec![(1, IoVal::Int(1))]),
            ("b".to_string(), vec![(2, IoVal::Int(0))]),
        ];

        let success = sim.load_inputs(inputs);

        assert_eq!(success, Ok(()));

        let result = sim.run(256, false);

        assert_eq!(
            result,
            Err(RuntimeError::Interpreter {
                err: InterpreterError::DivisionByZero,
                rel_name: "DIV".to_string(),
                rel_id: 0,
                timestep: 2,
            })
        );
    }

    #[test]
    fn max_steps() {
        // net A {
        //     input a: Int;
        //     output a: Int;
        // }
        let netlist = Netlist {
            relations: vec![],
            ents: vec![Entity {
                val: None,
                sinks: vec![],
            }],
        };
        let interface = Interface {
            inputs: HashMap::from([("a".to_string(), (0, Type::Int))]),
            outputs: HashMap::from([(0, ("a".to_string(), Type::Int))]),
            custom_type_maps: HashMap::new(),
        };

        let relations = vec![];

        let inits = vec![];

        let mut sim = Simulator::new(netlist, interface, relations, inits);

        let inputs = vec![("a".to_string(), vec![(5, IoVal::Int(1))])];

        let success = sim.load_inputs(inputs);

        assert_eq!(success, Ok(()));

        let result = sim.run(10, false);

        assert_eq!(result, Ok(6));
    }

    #[test]
    fn finish_early() {
        // net A {
        //     input a: Int;
        //     output a: Int;
        // }
        let netlist = Netlist {
            relations: vec![],
            ents: vec![Entity {
                val: None,
                sinks: vec![],
            }],
        };
        let interface = Interface {
            inputs: HashMap::from([("a".to_string(), (0, Type::Int))]),
            outputs: HashMap::from([(0, ("a".to_string(), Type::Int))]),
            custom_type_maps: HashMap::new(),
        };

        let relations = vec![];

        let inits = vec![];

        let mut sim = Simulator::new(netlist, interface, relations, inits);

        let inputs = vec![("a".to_string(), vec![(50, IoVal::Int(1))])];

        let success = sim.load_inputs(inputs);

        assert_eq!(success, Ok(()));

        let result = sim.run(10, false);

        assert_eq!(result, Ok(10));
    }

    #[test]
    fn nondeterministic_runtime() {
        // net A {
        //     input a: Int;
        //     output b: Int;

        //     RND(a) := b;
        // }
        let netlist = Netlist {
            relations: vec![
                Relation {
                    idx: 0,
                    delay: 1,
                    input_ents: vec![0],
                    output_ent: 1,
                },
            ],
            ents: vec![
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
        let interface = Interface {
            inputs: HashMap::from([("a".to_string(), (0, Type::Int))]),
            outputs: HashMap::from([(1, ("b".to_string(), Type::Int))]),
            custom_type_maps: HashMap::new(),
        };

        let relations = vec![
            CompiledRel {
                name: "RND".to_string(),
                complexity: 0,
                deterministic: false,
                bytecode: assemble(
                "
                    RET i0
                ",
            )
            .unwrap(),
            },
        ];

        let inits = vec![];

        let mut sim = Simulator::new(netlist, interface, relations, inits);

        let inputs = vec![("a".to_string(), vec![(5, IoVal::Int(1))])];

        let success = sim.load_inputs(inputs);

        assert_eq!(success, Ok(()));

        let result = sim.run(10, false);

        assert_eq!(result, Ok(10));

        let output = sim.dump_outputs();

        assert_eq!(
            output,
            vec![(
                "b".to_string(),
                vec![
                    (6 as usize, IoVal::Int(0)),
                ]
            ),]
        );
    }

    #[test]
    fn nand_vs_and_and_not() {
        let netlist = Netlist {
            relations: vec![Relation {
                idx: 0,
                delay: 2,
                input_ents: vec![0, 1],
                output_ent: 2,
            }],
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
        let interface = Interface {
            inputs: HashMap::from([
                ("a".to_string(), (0, Type::Bool)),
                ("b".to_string(), (1, Type::Bool)),
            ]),
            outputs: HashMap::from([(2, ("q".to_string(), Type::Bool))]),
            custom_type_maps: HashMap::new(),
        };

        let relations = vec![CompiledRel {
            name: "NAND".to_string(),
            complexity: 0,
            deterministic: true,
            bytecode: assemble(
                "
                    AND r4 r2 r3
                    NOT r4 r4
                    RET r4
                ",
            )
            .unwrap(),
        }];

        let inits = vec![];

        let mut sim = Simulator::new(netlist, interface, relations, inits);

        let inputs = vec![
            (
                "a".to_string(),
                vec![
                    (1, IoVal::Bool(true)),
                    (5, IoVal::Bool(true)),
                    (9, IoVal::Bool(false)),
                    (13, IoVal::Bool(false)),
                ],
            ),
            (
                "b".to_string(),
                vec![
                    (1, IoVal::Bool(true)),
                    (5, IoVal::Bool(false)),
                    (9, IoVal::Bool(true)),
                    (13, IoVal::Bool(false)),
                ],
            ),
        ];

        let success = sim.load_inputs(inputs);

        assert_eq!(success, Ok(()));

        let _steps = sim.run(256, false);

        let output_1 = sim.dump_outputs();

        let netlist = Netlist {
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
                    sinks: vec![1],
                },
            ],
        };
        let interface = Interface {
            inputs: HashMap::from([
                ("a".to_string(), (0, Type::Bool)),
                ("b".to_string(), (1, Type::Bool)),
            ]),
            outputs: HashMap::from([(2, ("q".to_string(), Type::Bool))]),
            custom_type_maps: HashMap::new(),
        };

        let relations = vec![
            CompiledRel {
                name: "AND".to_string(),
                complexity: 0,
                deterministic: true,
                bytecode: assemble(
                    "
                    AND r4 r2 r3
                    RET r4
                ",
                )
                .unwrap(),
            },
            CompiledRel {
                name: "NOT".to_string(),
                complexity: 0,
                deterministic: true,
                bytecode: assemble(
                    "
                    NOT r4 r3
                    RET r4
                ",
                )
                .unwrap(),
            },
        ];

        let inits = vec![];

        let mut sim = Simulator::new(netlist, interface, relations, inits);

        let inputs = vec![
            (
                "a".to_string(),
                vec![
                    (1, IoVal::Bool(true)),
                    (5, IoVal::Bool(true)),
                    (9, IoVal::Bool(false)),
                    (13, IoVal::Bool(false)),
                ],
            ),
            (
                "b".to_string(),
                vec![
                    (1, IoVal::Bool(true)),
                    (5, IoVal::Bool(false)),
                    (9, IoVal::Bool(true)),
                    (13, IoVal::Bool(false)),
                ],
            ),
        ];

        let success = sim.load_inputs(inputs);

        assert_eq!(success, Ok(()));

        let _steps = sim.run(256, false);

        let output_2 = sim.dump_outputs();

        assert_eq!(output_1, output_2);
    }

    #[test]
    fn stop_on_next_output() {
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
        let interface = Interface {
            inputs: HashMap::from([
                ("a".to_string(), (0, Type::Bool)),
                ("b".to_string(), (1, Type::Bool)),
            ]),
            outputs: HashMap::from([(2, ("q".to_string(), Type::Bool))]),
            custom_type_maps: HashMap::new(),
        };

        let relations = vec![CompiledRel {
            name: "NAND".to_string(),
            complexity: 0,
            deterministic: true,
            bytecode: assemble(
                "
                    AND r4 r2 r3
                    NOT r4 r4
                    RET r4
                ",
            )
            .unwrap(),
        }];

        let inits = vec![];

        let mut sim = Simulator::new(netlist, interface, relations, inits);

        let inputs = vec![
            (
                "a".to_string(),
                vec![(1, IoVal::Bool(false)), (5, IoVal::Bool(true))],
            ),
            (
                "b".to_string(),
                vec![(1, IoVal::Bool(false)), (9, IoVal::Bool(true))],
            ),
        ];

        let success = sim.load_inputs(inputs);

        assert_eq!(success, Ok(()));

        let _steps = sim.run(256, true);

        let output_1 = sim.dump_outputs();

        assert_eq!(
            output_1,
            vec![("q".to_string(), vec![(4 as usize, IoVal::Bool(false)),]),]
        );

        let _steps = sim.run(256, false);

        let output_2 = sim.dump_outputs();

        assert_eq!(
            output_2,
            vec![(
                "q".to_string(),
                vec![
                    (7 as usize, IoVal::Bool(true)),
                    (12 as usize, IoVal::Bool(false)),
                ]
            ),]
        );
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
            relations: vec![Relation {
                idx: 0,
                delay: 1,
                input_ents: vec![0],
                output_ent: 0,
            }],
            ents: vec![Entity {
                val: None,
                sinks: vec![0],
            }],
        };
        let interface = Interface {
            inputs: HashMap::new(),
            outputs: HashMap::from([(0, ("a".to_string(), Type::Int))]),
            custom_type_maps: HashMap::new(),
        };

        let relations = vec![CompiledRel {
            name: "NAND".to_string(),
            complexity: 0,
            deterministic: true,
            bytecode: assemble(
                "
                    IADD r4 r2 i1
                    RET r4
                ",
            )
            .unwrap(),
        }];

        let inits = vec![Event {
            timestep: 0,
            ent_id: 0,
            new_val: 0,
        }];

        let mut sim = Simulator::new(netlist, interface, relations, inits);

        let _steps = sim.run(5, false);

        let output = sim.dump_outputs();

        assert_eq!(
            output,
            vec![(
                "a".to_string(),
                vec![
                    (0 as usize, IoVal::Int(0)),
                    (1 as usize, IoVal::Int(1)),
                    (2 as usize, IoVal::Int(2)),
                    (3 as usize, IoVal::Int(3)),
                    (4 as usize, IoVal::Int(4)),
                    (5 as usize, IoVal::Int(5)),
                ]
            ),]
        );
    }

    #[test]
    fn impulse_oscillator() {
        // rel_t imp_delay : (a: Impulse) -> Impulse = a;
        // rel_t read_imp: (a: Impulse) -> Int = {
        //     cases a {
        //         true: 1,
        //         _ : 0,
        //     }
        // };
        // net OSC {
        //     output i: Impulse
        //     output a: Int;

        //     init imp_1: Impulse = true; // will cause impulse on sim step 1

        //     imp_1 := imp_delay(imp_2);
        //     imp_2 := imp_delay(imp_1);

        //     i := imp_delay(imp_2);

        //     a := read_imp(i);
        // }
        let netlist = Netlist {
            relations: vec![
                Relation {
                    idx: 0,
                    delay: 1,
                    input_ents: vec![3],
                    output_ent: 2,
                },
                Relation {
                    idx: 0,
                    delay: 1,
                    input_ents: vec![2],
                    output_ent: 3,
                },
                Relation {
                    idx: 0,
                    delay: 1,
                    input_ents: vec![3],
                    output_ent: 0,
                },
                Relation {
                    idx: 1,
                    delay: 1,
                    input_ents: vec![0],
                    output_ent: 1,
                },
            ],
            ents: vec![
                Entity {
                    // i
                    val: None,
                    sinks: vec![3],
                },
                Entity {
                    // a
                    val: None,
                    sinks: vec![],
                },
                Entity {
                    // imp_1
                    val: None,
                    sinks: vec![1],
                },
                Entity {
                    // imp_2
                    val: None,
                    sinks: vec![0, 2],
                },
            ],
        };
        let interface = Interface {
            inputs: HashMap::new(),
            outputs: HashMap::from([
                (0, ("i".to_string(), Type::Impulse)),
                (1, ("a".to_string(), Type::Int)),
            ]),
            custom_type_maps: HashMap::new(),
        };

        let relations = vec![
            CompiledRel {
                name: "imp_delay".to_string(),
                complexity: 0,
                deterministic: true,
                bytecode: assemble(
                    "
                    IADD r3 r2 r1
                    RET r3
                ",
                )
                .unwrap(),
            },
            CompiledRel {
                name: "read_imp".to_string(),
                complexity: 0,
                deterministic: true,
                bytecode: assemble(
                    "
                    IEQ r3 r2 r0
                    IJNE o13 r3 i1
                    MOV r4 i1
                    JMP o10
                    MOV r4 i0
                    RET r4
                ",
                )
                .unwrap(),
            },
        ];

        let inits = vec![Event {
            timestep: 0,
            ent_id: 2,
            new_val: 0,
        }];

        let mut sim = Simulator::new(netlist, interface, relations, inits);

        let _steps = sim.run(10, false);

        let output = sim.dump_outputs();

        assert_eq!(
            output,
            vec![
                (
                    "i".to_string(),
                    vec![
                        (2, IoVal::Bool(true)),
                        (4, IoVal::Bool(true)),
                        (6, IoVal::Bool(true)),
                        (8, IoVal::Bool(true)),
                        (10, IoVal::Bool(true)),
                    ]
                ),
                ("a".to_string(), vec![(3, IoVal::Int(1)),]),
            ]
        );
    }

    #[test]
    fn custom_type_state_machine() {
        // ent_t LET = {A, B, C};
        // rel_t SHIFT : (a: LET) ->  LET = {
        //     cases a {
        //         A : B,
        //         B : C,
        //         _ : A,
        //     }
        // };
        // net DUO {
        //     output a: LET;

        //     init a: LET = A;
        //     init b: LET = B;

        //     a := SHIFT(b);
        //     b := SHIFT(a);
        // }
        // (a,b): (A,B), (C,B), (C,A), (B,A), (B,C), (A,C), repeat
        let net = Netlist {
            relations: vec![
                Relation {
                    idx: 0,
                    delay: 1,
                    input_ents: vec![1],
                    output_ent: 0,
                },
                Relation {
                    idx: 0,
                    delay: 1,
                    input_ents: vec![0],
                    output_ent: 1,
                },
            ],
            ents: vec![
                Entity {
                    // a
                    val: None,
                    sinks: vec![1],
                }, // b
                Entity {
                    val: None,
                    sinks: vec![0],
                },
            ],
        };

        let interface = Interface {
            inputs: HashMap::new(),
            outputs: HashMap::from([(
                0,
                (
                    "a".to_string(),
                    Type::Custom(Ident::Str {
                        val: "LET".to_string(),
                        span: Span { line: 0, col: 0 },
                    }),
                ),
            )]),
            custom_type_maps: HashMap::from([(
                "LET".to_string(),
                vec![
                    ("A".to_string(), 0),
                    ("B".to_string(), 1),
                    ("C".to_string(), 2),
                ],
            )]),
        };

        let relations = vec![CompiledRel {
            name: "SHIFT".to_string(),
            complexity: 0,
            deterministic: true,
            bytecode: assemble(
                "
                    IJNE o13 r2 i0
                    MOV r3 i1
                    JMP o35
                    IJNE o13 r2 i1
                    MOV r3 i2
                    JMP o10
                    MOV r3 i0
                    RET r3
                ",
            )
            .unwrap(),
        }];

        let inits = vec![
            Event {
                timestep: 0,
                ent_id: 0,
                new_val: 0,
            },
            Event {
                timestep: 0,
                ent_id: 1,
                new_val: 1,
            },
        ];

        let mut sim = Simulator::new(net, interface, relations, inits);

        let steps = sim.run(5, false);

        assert_eq!(steps, Ok(5));

        let output = sim.dump_outputs();

        assert_eq!(
            output,
            vec![(
                "a".to_string(),
                vec![
                    (0, IoVal::Custom("A".to_string())),
                    (1, IoVal::Custom("C".to_string())),
                    (3, IoVal::Custom("B".to_string())),
                    (5, IoVal::Custom("A".to_string())),
                ]
            ),]
        );
    }
}
