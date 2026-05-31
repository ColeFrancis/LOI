use rand::seq::SliceRandom;

use crate::core::types::{NetId, Logic};
use crate::circuit::circuit::Circuit;
use super::event::Event;
use super::scheduler::Wheel;
use super::watcher::Watcher;

use crate::logic::eval::eval_gate;

pub struct Simulator {
    wheel: Wheel<256>,
    circuit: Circuit,
    watchers: Vec<Watcher>,
}

impl Simulator {
    pub fn new(circuit: Circuit) -> Self {
        Self {
            wheel: Wheel::new(),
            circuit,
            watchers: Vec::new(),
        }
    }

    pub fn schedule_event(&mut self, time: usize, net: NetId, level: Logic) {
        self.wheel.push(Event {time: time, net: net, new_value: level});
    }

    pub fn read_net(&self, net: NetId) -> Logic {
        self.circuit.nets[net].value
    }

    pub fn create_watcher(&mut self, net: NetId) {
        self.watchers.push(Watcher::new(net));
    }

    pub fn read_watcher(& self, net: NetId) -> Option<&Vec<Logic>> {
        for watcher in &self.watchers {
            if watcher.net == net {
                return Some(&watcher.outputs);
            }
        }

        None
    }
    
    pub fn run(&mut self, max_steps: usize) {
        // Pseudocode:
        //  grab next events if there is one
        //  loop for event in events in a random order:
        //      if the net's value is not changing:
        //          skip event
        //
        //      set net's value to next value
        //
        //      Loop for gate in net's sinks:
        //          evaluate gate
        //          
        //          if the gate's output changes:
        //              push new event with gate delay
        //
        //  update watchers
        
        let mut step: usize = 0;
        
        while let Some(mut curr_events) = self.wheel.pop() {
            curr_events.shuffle(&mut rand::rng());

            for event in curr_events {
                if self.circuit.nets[event.net].value == event.new_value {
                    continue;
                }

                self.circuit.nets[event.net].value = event.new_value;

                let sinks = self.circuit.nets[event.net].sinks.clone();

                for gate_id in sinks {
                    let gate = &self.circuit.gates[gate_id];
                    
                    let old_out = self.circuit.nets[gate.out].value;
                    let new_out = eval_gate(&self.circuit, gate);

                    if old_out != new_out {
                        self.wheel.push(Event {
                            time: event.time+1, 
                            net:gate.out, 
                            new_value: new_out
                        });
                    }
                }
            }

            for watcher in &mut self.watchers {
                let value = self.circuit.nets[watcher.net].value;

                watcher.outputs.push(value);
            }

            step += 1;
            if step >= max_steps {
                break;
            }
        }
    }

    pub fn reset(&mut self) {
        for net in &mut self.circuit.nets {
            net.value = Logic::X;
        }
        
        self.wheel.reset();

        for watcher in &mut self.watchers {
            watcher.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Logic;
    use crate::circuit::circuit::Circuit;
    use crate::circuit::gate::Gate;
    use crate::circuit::net::Net;

    #[test]
    fn nand_gate() {
        let mut circuit = Circuit::new();

        // 0
        circuit.nets.push(Net {value: Logic::X, sinks: vec![0]});
        // 1
        circuit.nets.push(Net {value: Logic::X, sinks: vec![0]});
        // 2
        circuit.nets.push(Net {value: Logic::X, sinks: vec![]});

        // 0
        circuit.gates.push(Gate {a: 0, b: 1, out: 2});

        let mut sim = Simulator::new(circuit);

        sim.schedule_event(0, 0, Logic::OFF);
        sim.schedule_event(0, 1, Logic::OFF);

        sim.run(256);

        let output = sim.read_net(2);

        assert_eq!(output, Logic::ON, "OFF, OFF gave {:?}", output);

        sim.reset();

        sim.schedule_event(0, 0, Logic::ON);
        sim.schedule_event(0, 1, Logic::OFF);

        sim.run(256);

        let output = sim.read_net(2);

        assert_eq!(output, Logic::ON, "ON, OFF gave {:?}", output);

        sim.reset();

        sim.schedule_event(0, 0, Logic::ON);
        sim.schedule_event(0, 1, Logic::ON);

        sim.run(256);

        let output = sim.read_net(2);

        assert_eq!(output, Logic::OFF, "ON, ON gave {:?}", output);
    }

    #[test]
    fn xor_gate() {
        let mut circuit = Circuit::new();

        // 0
        circuit.nets.push(Net {value: Logic::X, sinks: vec![0, 1]});
        // 1
        circuit.nets.push(Net {value: Logic::X, sinks: vec![0, 2]});
        // 2
        circuit.nets.push(Net {value: Logic::X, sinks: vec![1, 2]});
        // 3
        circuit.nets.push(Net {value: Logic::X, sinks: vec![3]});
        // 4
        circuit.nets.push(Net {value: Logic::X, sinks: vec![3]});
        // 5
        circuit.nets.push(Net {value: Logic::X, sinks: vec![]});

        // 0
        circuit.gates.push(Gate {a: 0, b: 1, out: 2});
        // 1
        circuit.gates.push(Gate {a: 0, b: 2, out: 3});
        // 2
        circuit.gates.push(Gate {a: 1, b: 2, out: 4});
        // 3
        circuit.gates.push(Gate {a: 3, b: 4, out: 5});

        let mut sim = Simulator::new(circuit);

        sim.schedule_event(0, 0, Logic::OFF);
        sim.schedule_event(0, 1, Logic::OFF);

        sim.run(256);

        println!("Net 0: {:?}, Net 1: {:?}, Net 2: {:?}, Net 3: {:?}, Net 4: {:?}, Net 5: {:?}",
            sim.read_net(0), sim.read_net(1), sim.read_net(2), sim.read_net(3), sim.read_net(4), sim.read_net(5));

        let output = sim.read_net(5);

        assert_eq!(output, Logic::OFF, "OFF, OFF gave {:?}", output);

        sim.reset();

        sim.schedule_event(0, 0, Logic::ON);
        sim.schedule_event(0, 1, Logic::OFF);

        sim.run(256);

        let output = sim.read_net(5);

        assert_eq!(output, Logic::ON, "ON, OFF gave {:?}", output);

        sim.reset();

        sim.schedule_event(0, 0, Logic::ON);
        sim.schedule_event(0, 1, Logic::ON);

        sim.run(256);

        let output = sim.read_net(5);

        assert_eq!(output, Logic::OFF, "ON, ON gave {:?}", output);
    }

    #[test]
    fn random_state_selection() {
        let num_iters = 50;

        let mut circuit = Circuit::new();

        // 0
        circuit.nets.push(Net {value: Logic::X, sinks: vec![0]});
        // 1
        circuit.nets.push(Net {value: Logic::X, sinks: vec![1]});
        // 2
        circuit.nets.push(Net {value: Logic::X, sinks: vec![1]});
        // 3
        circuit.nets.push(Net {value: Logic::X, sinks: vec![0]});

        // 0
        circuit.gates.push(Gate {a: 0, b: 3, out: 2});
        // 1
        circuit.gates.push(Gate {a: 1, b: 2, out: 3});

        let mut sim = Simulator::new(circuit);

        let mut outputs: Vec<Logic> = Vec::new();

        for _ in 0..num_iters {
            sim.schedule_event(0, 0, Logic::OFF);
            sim.schedule_event(0, 1, Logic::OFF);

            sim.schedule_event(5, 0, Logic::ON);
            sim.schedule_event(5, 1, Logic::ON);

            sim.run(256);

            outputs.push(sim.read_net(2));

            sim.reset();
        }
    
        let count: usize = outputs
            .iter()
            .filter(|&&x| x == Logic::ON)
            .count();

        let proportion: f64 = count as f64 / num_iters as f64;
        
        // Make sure proportion is within 2 standard deviations
        assert!((proportion - 0.5).abs() <= 0.14, "proportion that are ON is not random but {}", proportion);
    }

    #[test]
    fn watcher() {
        let mut circuit = Circuit::new();

        circuit.nets.push(Net {value: Logic::X, sinks: vec![]});

        let mut sim = Simulator::new(circuit);
        sim.create_watcher(0);

        sim.schedule_event(0, 0, Logic::ON);
        sim.schedule_event(2, 0, Logic::OFF);

        sim.run(256);

        let output: &Vec<Logic> = sim.read_watcher(0).unwrap();
        for val in output {
            println!("{:?}", val);
        }

        assert_eq!(output, &vec![Logic::ON, Logic::ON, Logic::OFF]);
    }

    #[test]
    fn oscillation() {
        let mut circuit = Circuit::new();

        // 0
        circuit.nets.push(Net {value: Logic::X, sinks: vec![0]});
        // 1
        circuit.nets.push(Net {value: Logic::X, sinks: vec![0]});

        // 0
        circuit.gates.push(Gate {a: 0, b: 1, out: 1});

        let mut sim = Simulator::new(circuit);

        sim.create_watcher(1);

        sim.schedule_event(0, 0, Logic::OFF);
        sim.schedule_event(0, 1, Logic::ON);

        sim.run(5);

        let output: &Vec<Logic> = sim.read_watcher(1).unwrap();
       
        assert!(false);
    }
}