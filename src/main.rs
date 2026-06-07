use nhdl::network::network::Network;
use nhdl::network::relation::Relation;
use nhdl::network::entity::Entity;
use nhdl::core::types::Logic;
use nhdl::core::operations::LogicOp;
use nhdl::file::write::write_file;

fn main() {
    println!("Hello World!");

    let mut network: Network<Logic, LogicOp> = Network::new();

    // 0
    network.entities.push(Entity {value: Logic::X, sinks: vec![0]});
    // 1
    network.entities.push(Entity {value: Logic::X, sinks: vec![0]});
    // 2
    network.entities.push(Entity {value: Logic::X, sinks: vec![]});

    // 0
    network.relations.push(Relation {op: LogicOp::NAND, a: 0, b: 1, out: 2});

    let inputs = vec![0, 1];
    let outputs = vec![2];

    write_file("test_output.txt", network, inputs, outputs)?;
}
