use crate::core_structs::*;

fn eval_nand(a: Logic, b: Logic) -> Logic {
    match (a, b) {
        (Logic::ON, Logic::ON) => Logic::OFF,
        (Logic::OFF, _) | (_, Logic::OFF) => Logic::ON,
        _ => Logic::X,
    }
}

pub fn eval_gate(circuit: &Circuit, gate: &Gate) -> Logic {
    let a = circuit.nets[gate.a].value;
    let b = circuit.nets[gate.b].value;

    eval_nand(a, b)
}