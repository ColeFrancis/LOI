use crate::core::types::{Logic, LogicOp};
use crate::core::types::{Real, RealOp};

pub trait Operator<T> {
    fn eval(&self, a: T, b: T) -> T;
}

impl Operator<Logic> for LogicOp {
    fn eval(&self, a: Logic, b: Logic) -> Logic {
        match self {
            LogicOp::NAND => {
                match (a, b) {
                    (Logic::ON, Logic::ON) => Logic::OFF,
                    (Logic::OFF, _) | (_, Logic::OFF) => Logic::ON,
                    _ => Logic::X,
                }
            }
        }
    }
}

impl Operator<Real> for RealOp {
    fn eval(&self, a: Real, b: Real) -> Real {
        match self {
            RealOp::ADD => {
                match (a, b) {
                    (Real::Val(val_a), Real::Val(val_b)) => Real::Val(val_a + val_b),
                    _ => Real::X,
                }
            },
            RealOp::MUL => {
                match (a, b) {
                    (Real::Val(val_a), Real::Val(val_b)) => Real::Val(val_a * val_b),
                    _ => Real::X,
                }
            }
        }
    }
}

/*pub fn eval_relation<T, O>(network: &Network<T, O>, relation: &Relation<O>,) -> T
where
    T: Copy,
    O: Operator<T>,
{
    let a = network.entities[relation.a].value;
    let b = network.entities[relation.b].value;

    relation.op.eval(a, b)
}*/