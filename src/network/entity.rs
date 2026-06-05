use super::relation::RelationId;

pub type EntityId = usize;

pub struct Entity<T> {
    pub value: T,
    pub sinks: Vec<RelationId>,
}