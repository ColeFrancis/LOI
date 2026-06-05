use super::relation::Relation;
use super::entity::Entity;

pub struct Network<T, O> {
    pub relations: Vec<Relation<O>>,
    pub entities: Vec<Entity<T>>,
}

impl<T, O> Network<T, O> {
    pub fn new() -> Self {
        Self {
            relations: Vec::new(),
            entities: Vec::new(),
        }
    }
}