//! # write
//!
//! (TODO) This module provides functions for writing networks to files for storage
//!
//! ## Invariants
//!
//! - Files must follow this format: (metadata,[in_entity_id_0,...,],[out_entity_id_0,...,],[(const_0_id, const_0_val),...,],[[entity_0_sink_0,...,],...,],[(relation data,in_entity_a,in_entity_b,out_entity,),...,],)
//!
//! Author: Cole Francis
//!
//! Last Updated: 06/08/2026

use std::fs::File;
use std::io::Write;
use std::any::type_name;

use crate::network::network::Network;
use crate::network::entity::EntityId;
use crate::core::operations::Operator;


pub fn write_file<T: std::fmt::Debug, O: Operator<T>>(file_name: &str, inputs: &Vec<EntityId>, outputs: &Vec<EntityId>, constants: &Vec<(EntityId, T)>, network: &Network<T, O>) -> std::io::Result<()> {
    let mut file = File::create(file_name)?;

    write!(file, "(")?;

    // Metadata
    write!(file, "{},", type_name::<T>())?;

    // Inputs
    write!(file, "[")?;
    for id in inputs {
        write!(file, "{},", id)?;
    }
    write!(file,"],")?;

    // Outputs
    write!(file, "[")?;
    for id in outputs {
        write!(file, "{},", id)?;
    }
    write!(file,"],")?;

    // Constants
    write!(file, "[")?;
    for (entity, val) in constants {
        write!(file, "({},{:?}),", entity, val)?;
    }
    write!(file, "],")?;

    // Entities
    write!(file, "[")?;
    for entity in &network.entities {
        write!(file, "[")?;
        for sink in &entity.sinks {
            write!(file, "{},", sink)?;
        }
        write!(file, "],")?;
    }
    write!(file, "],")?;

    // Relations
    write!(file, "[")?;
    for relation in &network.relations {
        write!(file, "({},{},{},{},),", relation.op.name(), relation.a, relation.b, relation.out)?;
    }
    write!(file, "],")?;

    write!(file, ")")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::network::Network;
    use crate::network::relation::Relation;
    use crate::network::entity::Entity;
    use crate::core::types::Logic;
    use crate::core::operations::LogicOp;

    #[test]
    fn test_file_write() -> std::io::Result<()> {
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

        write_file("test_output.nhdl", &inputs, &outputs, &vec![], &network)?;

        // TODO: Write test using read.rs

        Ok(())
    }
}