//! # sim
//!
//! This module forms the core of the simulator of nhdl
//!
//! ## Invariants
//!
//! - This module shall be able to simulate any network with any set of valid input events and
//! track the state of any specified entity in the network at any time
//! - The simulator shall handle all events sequentially
//! - The simulator shall ensure the network stays at a valid state at all times
//! - The simulator shall be as simple and generic as possible while still maintining its intended use
//!
//! Author: Cole Francis
//!
//! Last Updated: 06/02/2026

use rand::seq::SliceRandom;

use crate::network::entity::EntityId;
use crate::network::network::Network;
use super::event::Event;
use super::scheduler::Wheel;
use super::watcher::Watcher;
use crate::core::types::Resettable;
use crate::core::operations::Operator;

pub struct Simulator<T, O> {
    wheel: Wheel<256, T>,
    network: Network<T, O>,
    watchers: Vec<Watcher<T>>,
}

impl<T, O> Simulator<T, O> 
where 
    T: Resettable + Copy + std::cmp::PartialEq,
    O: Operator<T>,
{
    pub fn new(network: Network<T, O>) -> Self {
        Self {
            wheel: Wheel::new(),
            network,
            watchers: Vec::new(),
        }
    }

    pub fn schedule_event(&mut self, time: usize, entity: EntityId, val: T) {
        self.wheel.push(Event {time: time, entity: entity, new_value: val});
    }

    /*pub fn read_entity(&self, entity: EntityId) -> &T {
        &self.network.entities[entity].value
    }*/
    pub fn read_entity(&self, entity: EntityId) -> T {
        self.network.entities[entity].value
    }

    pub fn create_watcher(&mut self, entity: EntityId) {
        self.watchers.push(Watcher::new(entity));
    }

    pub fn read_watcher(& self, entity: EntityId) -> Option<&[T]> {
        for watcher in &self.watchers {
            if watcher.entity == entity {
                return Some(&watcher.outputs);
            }
        }

        None
    }
    
    pub fn run(&mut self, max_steps: usize, only_necessary_events: bool) -> Option<usize> {
        // Pseudocode:
        //  grab next events if there is one
        //  loop for event in events in a random order:
        //      if the entities value is not changing and only necessary events:
        //          skip event
        //
        //      set entities value to next value
        //
        //      Loop for relation in entities sinks:
        //          evaluate relation
        //          
        //          if the relation's output changes or allow unnecessary events:
        //              push new event with relation delay
        //
        //  update watchers
        //  check if number of steps has exceeded max
        
        let mut step: usize = 0;
        
        while let Some(mut curr_events) = self.wheel.pop() {
            curr_events.shuffle(&mut rand::rng());

            for event in curr_events {
                if self.network.entities[event.entity].value == event.new_value && only_necessary_events {
                    continue;
                }

                self.network.entities[event.entity].value = event.new_value;

                for &relation_id in &self.network.entities[event.entity].sinks {
                    let relation = &self.network.relations[relation_id];
                    
                    let old_out = self.network.entities[relation.out].value;
                    let new_out = relation.eval(&self.network); // eval_relation(&self.network, relation);

                    if old_out != new_out || !only_necessary_events {
                        self.wheel.push(Event {
                            time: event.time+1, 
                            entity:relation.out, 
                            new_value: new_out
                        });
                    }
                }
            }

            for watcher in &mut self.watchers {
                let value = self.network.entities[watcher.entity].value;

                watcher.outputs.push(value);
            }

            step += 1;
            if step >= max_steps {
                return None;
            }
        }

        return Some(step);
    }

    pub fn reset(&mut self) where T: Resettable {
        for entity in &mut self.network.entities {
            entity.value = T::reset();
        }
        
        self.wheel.reset();

        for watcher in &mut self.watchers {
            watcher.reset();
        }
    }

    pub fn into_network(self) -> Network<T, O> {
        self.network
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::network::Network;
    use crate::network::relation::Relation;
    use crate::network::entity::Entity;
    use crate::core::types::Logic;
    use crate::core::operations::LogicOp;
    use crate::core::types::Real;
    use crate::core::operations::RealOp;

    fn assert_real_slice_eq(actual: &[Real], expected: &[Real], eps: f64) {
        assert_eq!(actual.len(), expected.len());

        for (a, e) in actual.iter().zip(expected.iter()) {
            match (a, e) {
                (Real::Val(a), Real::Val(e)) => {
                    assert!((a - e).abs() < eps);
                }
                _ => {
                    assert!(false);
                }
            }
        }
    }

    #[test]
    fn nand_gate() {
        let mut network: Network<Logic, LogicOp> = Network::new();

        // 0
        network.entities.push(Entity {value: Logic::X, sinks: vec![0]});
        // 1
        network.entities.push(Entity {value: Logic::X, sinks: vec![0]});
        // 2
        network.entities.push(Entity {value: Logic::X, sinks: vec![]});

        // 0
        network.relations.push(Relation {op: LogicOp::NAND, a: 0, b: 1, out: 2});

        let mut sim: Simulator<Logic, LogicOp> = Simulator::new(network);

        sim.schedule_event(0, 0, Logic::OFF);
        sim.schedule_event(0, 1, Logic::OFF);

        sim.run(256, true);

        let output = sim.read_entity(2);

        assert_eq!(output, Logic::ON, "OFF, OFF gave {:?}", output);

        sim.reset();

        sim.schedule_event(0, 0, Logic::ON);
        sim.schedule_event(0, 1, Logic::OFF);

        sim.run(256, true);

        let output = sim.read_entity(2);

        assert_eq!(output, Logic::ON, "ON, OFF gave {:?}", output);

        sim.reset();

        sim.schedule_event(0, 0, Logic::ON);
        sim.schedule_event(0, 1, Logic::ON);

        sim.run(256, true);

        let output = sim.read_entity(2);

        assert_eq!(output, Logic::OFF, "ON, ON gave {:?}", output);
    }

    #[test]
    fn xor_gate() {
        let mut network: Network<Logic, LogicOp> = Network::new();

        // 0
        network.entities.push(Entity {value: Logic::X, sinks: vec![0, 1]});
        // 1
        network.entities.push(Entity {value: Logic::X, sinks: vec![0, 2]});
        // 2
        network.entities.push(Entity {value: Logic::X, sinks: vec![1, 2]});
        // 3
        network.entities.push(Entity {value: Logic::X, sinks: vec![3]});
        // 4
        network.entities.push(Entity {value: Logic::X, sinks: vec![3]});
        // 5
        network.entities.push(Entity {value: Logic::X, sinks: vec![]});

        // 0
        network.relations.push(Relation {op: LogicOp::NAND, a: 0, b: 1, out: 2});
        // 1
        network.relations.push(Relation {op: LogicOp::NAND, a: 0, b: 2, out: 3});
        // 2
        network.relations.push(Relation {op: LogicOp::NAND, a: 1, b: 2, out: 4});
        // 3
        network.relations.push(Relation {op: LogicOp::NAND, a: 3, b: 4, out: 5});

        let mut sim: Simulator<Logic, LogicOp> = Simulator::new(network);

        sim.schedule_event(0, 0, Logic::OFF);
        sim.schedule_event(0, 1, Logic::OFF);

        sim.run(256, true);

        println!("Entity 0: {:?}, Entity 1: {:?}, Entity 2: {:?}, Entity 3: {:?}, Entity 4: {:?}, Entity 5: {:?}",
            sim.read_entity(0), sim.read_entity(1), sim.read_entity(2), sim.read_entity(3), sim.read_entity(4), sim.read_entity(5));

        let output = sim.read_entity(5);

        assert_eq!(output, Logic::OFF, "OFF, OFF gave {:?}", output);

        sim.reset();

        sim.schedule_event(0, 0, Logic::ON);
        sim.schedule_event(0, 1, Logic::OFF);

        sim.run(256, true);

        let output = sim.read_entity(5);

        assert_eq!(output, Logic::ON, "ON, OFF gave {:?}", output);

        sim.reset();

        sim.schedule_event(0, 0, Logic::ON);
        sim.schedule_event(0, 1, Logic::ON);

        sim.run(256, true);

        let output = sim.read_entity(5);

        assert_eq!(output, Logic::OFF, "ON, ON gave {:?}", output);
    }

    #[test]
    fn integrator() {
        let mut network: Network<Real, RealOp> = Network::new();

        // 0
        network.entities.push(Entity {value: Real::X, sinks: vec![0]});
        // 1
        network.entities.push(Entity {value: Real::Val(1.0), sinks: vec![0]});

        // 1
        network.relations.push(Relation {op: RealOp::ADD, a: 0, b: 1, out: 1});

        let mut sim: Simulator<Real, RealOp> = Simulator::new(network);

        sim.create_watcher(1);

        sim.schedule_event(0, 0, Real::Val(1.0));

        sim.run(3, true);

        let output: &[Real] = sim.read_watcher(1).unwrap();
        for val in output {
            println!("{:?}", val);
        }

        assert_real_slice_eq(output, &vec![Real::Val(1.0), Real::Val(2.0), Real::Val(3.0)], 1e-9);
    }

    #[test]
    fn random_state_selection() {
        let num_iters = 50;

        let mut network: Network<Logic, LogicOp> = Network::new();

        // 0
        network.entities.push(Entity {value: Logic::X, sinks: vec![0]});
        // 1
        network.entities.push(Entity {value: Logic::X, sinks: vec![1]});
        // 2
        network.entities.push(Entity {value: Logic::X, sinks: vec![1]});
        // 3
        network.entities.push(Entity {value: Logic::X, sinks: vec![0]});

        // 0
        network.relations.push(Relation {op: LogicOp::NAND, a: 0, b: 3, out: 2});
        // 1
        network.relations.push(Relation {op: LogicOp::NAND, a: 1, b: 2, out: 3});

        let mut sim: Simulator<Logic, LogicOp> = Simulator::new(network);

        let mut outputs: Vec<Logic> = Vec::new();

        for _ in 0..num_iters {
            sim.schedule_event(0, 0, Logic::OFF);
            sim.schedule_event(0, 1, Logic::OFF);

            sim.schedule_event(5, 0, Logic::ON);
            sim.schedule_event(5, 1, Logic::ON);

            sim.run(256, true);

            outputs.push(sim.read_entity(2));

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
        let mut network: Network<Logic, LogicOp> = Network::new();

        network.entities.push(Entity {value: Logic::X, sinks: vec![]});

        let mut sim: Simulator<Logic, LogicOp> = Simulator::new(network);
        sim.create_watcher(0);

        sim.schedule_event(0, 0, Logic::ON);
        sim.schedule_event(2, 0, Logic::OFF);

        sim.run(256, true);

        let output: &[Logic] = sim.read_watcher(0).unwrap();
        for val in output {
            println!("{:?}", val);
        }

        assert_eq!(output, &[Logic::ON, Logic::ON, Logic::OFF]);
    }

    #[test]
    fn start_stop() {
        let mut network: Network<Logic, LogicOp> = Network::new();

        // 0
        network.entities.push(Entity {value: Logic::X, sinks: vec![0]});
        // 1
        network.entities.push(Entity {value: Logic::X, sinks: vec![1]});
        // 2
        network.entities.push(Entity {value: Logic::X, sinks: vec![2]});
        // 3
        network.entities.push(Entity {value: Logic::X, sinks: vec![0]});

        // 0
        network.relations.push(Relation {op: LogicOp::NAND, a: 0, b: 0, out: 1});
        // 1
        network.relations.push(Relation {op: LogicOp::NAND, a: 1, b: 1, out: 2});
        // 0
        network.relations.push(Relation {op: LogicOp::NAND, a: 2, b: 2, out: 3});

        let mut sim: Simulator<Logic, LogicOp> = Simulator::new(network);

        sim.create_watcher(1);
        sim.create_watcher(3);

        sim.schedule_event(0, 0, Logic::ON);

        sim.run(2, true);

        let output_1: &[Logic] = sim.read_watcher(1).unwrap();
        let output_3: &[Logic] = sim.read_watcher(3).unwrap();

        assert_eq!(output_1, &[Logic::X, Logic::OFF]);
        assert_eq!(output_3, &[Logic::X, Logic::X]);

        sim.run(10, true);

        let output_1: &[Logic] = sim.read_watcher(1).unwrap();
        let output_3: &[Logic] = sim.read_watcher(3).unwrap();

        assert_eq!(output_1, &[Logic::X, Logic::OFF, Logic::OFF, Logic::OFF]);
        assert_eq!(output_3, &[Logic::X, Logic::X, Logic::X, Logic::OFF]);
    }

    #[test]
    fn only_neccessary() {
        let mut network: Network<Logic, LogicOp> = Network::new();

        // 0
        network.entities.push(Entity {value: Logic::X, sinks: vec![0]});
        // 1
        network.entities.push(Entity {value: Logic::X, sinks: vec![0]});
        // 2
        network.entities.push(Entity {value: Logic::X, sinks: vec![1]});
        // 3
        network.entities.push(Entity {value: Logic::X, sinks: vec![]});

        // 0
        network.relations.push(Relation {op: LogicOp::NAND, a: 0, b: 1, out: 2});
        // 1
        network.relations.push(Relation {op: LogicOp::NAND, a: 2, b: 2, out: 3});

        let mut sim: Simulator<Logic, LogicOp> = Simulator::new(network);

        sim.schedule_event(0, 0, Logic::ON);

        let only_neccessary = sim.run(256, true).unwrap();

        assert_eq!(only_neccessary, 1);

        sim.reset();

        sim.schedule_event(0, 0, Logic::ON);

        let all = sim.run(256, false).unwrap();

        assert_eq!(all, 3);
    }

    #[test]
    fn oscillation() {
        let mut network: Network<Logic, LogicOp> = Network::new();

        // 0
        network.entities.push(Entity {value: Logic::X, sinks: vec![0]});
        // 1
        network.entities.push(Entity {value: Logic::X, sinks: vec![0]});

        // 0
        network.relations.push(Relation {op: LogicOp::NAND, a: 0, b: 1, out: 1});

        let mut sim: Simulator<Logic, LogicOp> = Simulator::new(network);

        sim.create_watcher(1);

        sim.schedule_event(0, 0, Logic::OFF);
        sim.schedule_event(3, 0, Logic::ON);
        sim.schedule_event(6, 0, Logic::OFF);

        sim.run(256, true);

        let output: &[Logic] = sim.read_watcher(1).unwrap();
        
        println!("{:?}", output);

        assert_eq!(output, &[Logic::X, Logic::ON, Logic::ON, Logic::ON, Logic::OFF, Logic::ON, Logic::OFF, Logic::ON]);
    }
}
