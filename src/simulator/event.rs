use crate::network::entity::EntityId;

pub struct Event<T> {
    pub time: usize,
    pub entity: EntityId,
    pub new_value: T,
}