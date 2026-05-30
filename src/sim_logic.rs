use std::collections::VecDeque;

use crate::core_structs::*;
use crate::core_logic::*;

pub struct Event {
    pub time: u64,
    pub net: NetId,
    pub new_value: Logic,
}

pub struct Simulation {
    queue: VecDeque<Event>,
    circuit: Circuit,
}

impl Simulation {
    pub fn new(circuit: Circuit) -> Self {
        Self {
            queue: VecDeque::new(),
            circuit,
        }
    }

    pub fn init_net(&mut self, net: NetId, level: Logic) {
        self.queue.push_back(Event {
            time: 0,
            net: net,
            new_value: level,
        });
    }

    pub fn read_net(&self, net: NetId) -> Logic {
        self.circuit.nets[net].value
    }

    pub fn run(&mut self) {
        while let Some(event) = self.queue.pop_front() {
            if self.circuit.nets[event.net].value == event.new_value {
                continue;
            }

            self.circuit.nets[event.net].value = event.new_value;

            let sinks = self.circuit.nets[event.net].sinks.clone();

            for gate_id in sinks {
                let gate = &self.circuit.gates[gate_id];
                
                let new_out = eval_gate(&self.circuit, gate);

                if new_out != self.circuit.nets[gate.out].value {
                    self.circuit.nets[gate.out].value = new_out;

                    for &out_net in &self.circuit.nets[gate.out].sinks {
                        self.queue.push_back(Event {
                            time: event.time + 1,
                            net: out_net,
                            new_value: new_out,
                        });
                    }
                }
            }
        }
    }

    pub fn reset(&mut self) {
        for net in &mut self.circuit.nets {
            net.value = Logic::X;
        }
    }
}