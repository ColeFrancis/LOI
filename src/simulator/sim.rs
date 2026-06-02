use rand::seq::SliceRandom;

use crate::core::types::{EntityId, Logic};
use crate::network::network::Network;
use super::event::Event;
use super::scheduler::Wheel;
use super::watcher::Watcher;

use crate::logic::eval::eval_relation;

pub struct Simulator {
    wheel: Wheel<256>,
    network: Network,
    watchers: Vec<Watcher>,
}

impl Simulator {
    pub fn new(network: Network) -> Self {
        Self {
            wheel: Wheel::new(),
            network,
            watchers: Vec::new(),
        }
    }

    pub fn schedule_event(&mut self, time: usize, entity: EntityId, level: Logic) {
        self.wheel.push(Event {time: time, entity: entity, new_value: level});
    }

    pub fn read_entity(&self, entity: EntityId) -> Logic {
        self.network.entities[entity].value
    }

    pub fn create_watcher(&mut self, entity: EntityId) {
        self.watchers.push(Watcher::new(entity));
    }

    pub fn read_watcher(& self, entity: EntityId) -> Option<&Vec<Logic>> {
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

                let sinks = self.network.entities[event.entity].sinks.clone();

                for relation_id in sinks {
                    let relation = &self.network.relations[relation_id];
                    
                    let old_out = self.network.entities[relation.out].value;
                    let new_out = eval_relation(&self.network, relation);

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

    pub fn reset(&mut self) {
        for entity in &mut self.network.entities {
            entity.value = Logic::X;
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
    use crate::network::network::Network;
    use crate::network::relation::Relation;
    use crate::network::entity::Entity;

    #[test]
    fn nand_gate() {
        let mut network = Network::new();

        // 0
        network.entities.push(Entity {value: Logic::X, sinks: vec![0]});
        // 1
        network.entities.push(Entity {value: Logic::X, sinks: vec![0]});
        // 2
        network.entities.push(Entity {value: Logic::X, sinks: vec![]});

        // 0
        network.relations.push(Relation::new(0, 1, 2));

        let mut sim = Simulator::new(network);

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
        let mut network = Network::new();

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
        network.relations.push(Relation::new(0, 1, 2));
        // 1
        network.relations.push(Relation::new(0, 2, 3));
        // 2
        network.relations.push(Relation::new(1, 2, 4));
        // 3
        network.relations.push(Relation::new(3, 4, 5));

        let mut sim = Simulator::new(network);

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
    fn random_state_selection() {
        let num_iters = 50;

        let mut network = Network::new();

        // 0
        network.entities.push(Entity {value: Logic::X, sinks: vec![0]});
        // 1
        network.entities.push(Entity {value: Logic::X, sinks: vec![1]});
        // 2
        network.entities.push(Entity {value: Logic::X, sinks: vec![1]});
        // 3
        network.entities.push(Entity {value: Logic::X, sinks: vec![0]});

        // 0
        network.relations.push(Relation::new(0, 3, 2));
        // 1
        network.relations.push(Relation::new(1, 2, 3));

        let mut sim = Simulator::new(network);

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
        let mut network = Network::new();

        network.entities.push(Entity {value: Logic::X, sinks: vec![]});

        let mut sim = Simulator::new(network);
        sim.create_watcher(0);

        sim.schedule_event(0, 0, Logic::ON);
        sim.schedule_event(2, 0, Logic::OFF);

        sim.run(256, true);

        let output: &Vec<Logic> = sim.read_watcher(0).unwrap();
        for val in output {
            println!("{:?}", val);
        }

        assert_eq!(output, &vec![Logic::ON, Logic::ON, Logic::OFF]);
    }

    #[test]
    fn start_stop() {
        let mut network = Network::new();

        // 0
        network.entities.push(Entity {value: Logic::X, sinks: vec![0]});
        // 1
        network.entities.push(Entity {value: Logic::X, sinks: vec![1]});
        // 2
        network.entities.push(Entity {value: Logic::X, sinks: vec![2]});
        // 3
        network.entities.push(Entity {value: Logic::X, sinks: vec![0]});

        // 0
        network.relations.push(Relation::new(0, 0, 1));
        // 1
        network.relations.push(Relation::new(1, 1, 2));
        // 0
        network.relations.push(Relation::new(2, 2, 3));

        let mut sim = Simulator::new(network);

        sim.create_watcher(1);
        sim.create_watcher(3);

        sim.schedule_event(0, 0, Logic::ON);

        sim.run(2, true);

        let output_1: &Vec<Logic> = sim.read_watcher(1).unwrap();
        let output_3: &Vec<Logic> = sim.read_watcher(3).unwrap();

        assert_eq!(output_1, &vec![Logic::X, Logic::OFF]);
        assert_eq!(output_3, &vec![Logic::X, Logic::X]);

        sim.run(10, true);

        let output_1: &Vec<Logic> = sim.read_watcher(1).unwrap();
        let output_3: &Vec<Logic> = sim.read_watcher(3).unwrap();

        assert_eq!(output_1, &vec![Logic::X, Logic::OFF, Logic::OFF, Logic::OFF]);
        assert_eq!(output_3, &vec![Logic::X, Logic::X, Logic::X, Logic::OFF]);
    }

    #[test]
    fn only_neccessary() {
        let mut network = Network::new();

        // 0
        network.entities.push(Entity {value: Logic::X, sinks: vec![0]});
        // 1
        network.entities.push(Entity {value: Logic::X, sinks: vec![0]});
        // 2
        network.entities.push(Entity {value: Logic::X, sinks: vec![1]});
        // 3
        network.entities.push(Entity {value: Logic::X, sinks: vec![]});

        // 0
        network.relations.push(Relation::new(0, 1, 2));
        // 1
        network.relations.push(Relation::new(2, 2, 3));

        let mut sim = Simulator::new(network);

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
        let mut network = Network::new();

        // 0
        network.entities.push(Entity {value: Logic::X, sinks: vec![0]});
        // 1
        network.entities.push(Entity {value: Logic::X, sinks: vec![0]});

        // 0
        network.relations.push(Relation::new(0, 1, 1));

        let mut sim = Simulator::new(network);

        sim.create_watcher(1);

        sim.schedule_event(0, 0, Logic::OFF);
        sim.schedule_event(3, 0, Logic::ON);
        sim.schedule_event(6, 0, Logic::OFF);

        sim.run(256, true);

        let output: &Vec<Logic> = sim.read_watcher(1).unwrap();
        
        println!("{:?}", output);

        assert_eq!(output, &vec![Logic::X, Logic::ON, Logic::ON, Logic::ON, Logic::OFF, Logic::ON, Logic::OFF, Logic::ON]);
    }
}
