// use nhdl::network::network::Network;
// use nhdl::network::relation::Relation;
// use nhdl::network::entity::Entity;
// use nhdl::core::types::Logic;
// use nhdl::core::operations::LogicOp;
// use nhdl::file::write::write_file;
use nhdl::file::write::write_file;

use nhdl::network::network::Network;
use nhdl::network::entity::Entity;
use nhdl::network::relation::Relation;

use nhdl::core::types::Logic;
use nhdl::core::operations::LogicOp;

fn main() -> std::io::Result<()> {
    println!("Hello World!");

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

    let inputs = vec![0, 1];
    let outputs = vec![5];

    write_file("test_output.nhdl", &inputs, &outputs, &vec![], &network)?;

    Ok(())
}
