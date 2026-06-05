use super::entity::EntityId;
use crate::network::network::Network;
use crate::logic::eval::Operator;

pub type RelationId = usize;

pub struct Relation<O> {
    pub op: O,
    pub a: EntityId,
    pub b: EntityId,
    pub out: EntityId,
}

impl<O> Relation<O> {
    pub fn eval<T>(&self, network: &Network<T, O>) -> T 
    where
        T: Copy,
        O: Operator<T>,
    {
        let a = network.entities[self.a].value;
        let b = network.entities[self.b].value;

        self.op.eval(a, b)
    }
}