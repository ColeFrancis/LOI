use super::event::Event;

pub struct Wheel<const WHEEL_SIZE: usize, T> {
    curr_time: usize,
    event_count: usize,
    events: [Vec<Event<T>>; WHEEL_SIZE],
}

impl<const WHEEL_SIZE: usize, T> Wheel<WHEEL_SIZE, T> {
    pub fn new() -> Self {
        Wheel {
            curr_time: 0,
            event_count: 0,
            events: std::array::from_fn(|_| Vec::new()),
        }
    }

    pub fn push(&mut self, event: Event<T>) {
        self.events[event.time % WHEEL_SIZE].push(event);
        self.event_count += 1;
    }

    pub fn pop(&mut self) -> Option<Vec<Event<T>>> {
        if self.event_count == 0 {
            return None;
        }

        let curr_events: Vec<Event<T>> = self.events[self.curr_time % WHEEL_SIZE]
            .extract_if(.., |e| e.time == self.curr_time)
            .collect();
        self.curr_time += 1;
        self.event_count -= curr_events.len();

        Some(curr_events)
    }

    pub fn reset(&mut self) {
        self.curr_time = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::entity::EntityId;
    use crate::core::types::Logic;

    const TEST_SIZE: usize = 4;

    fn make_event(time: usize, entity: EntityId, value: Logic) -> Event<Logic> {
        Event {
            time,
            entity,
            new_value: value,
        }
    }

    #[test]
    fn basic_push_pop() {
        let mut wheel: Wheel<TEST_SIZE, Logic> = Wheel::new();

        wheel.push(make_event(0, 1, Logic::ON));
        wheel.push(make_event(2, 2, Logic::OFF));

        let first_pop = wheel.pop();
        assert!(first_pop.is_some());
        let events = first_pop.unwrap();
        assert!(!events.is_empty());
        assert_eq!(events[0].new_value, Logic::ON);

        let second_pop = wheel.pop();
        assert!(second_pop.is_some());
        let events = second_pop.unwrap();
        assert!(events.is_empty());

        let third_pop = wheel.pop();
        let events = third_pop.unwrap();
        assert!(!events.is_empty());
        assert_eq!(events[0].new_value, Logic::OFF);

        let forth_pop = wheel.pop();
        assert!(!forth_pop.is_some());
    }

    #[test]
    fn simultaneous_and_no_events() {
        let mut wheel: Wheel<TEST_SIZE, Logic> = Wheel::new();

        wheel.push(make_event(0, 1, Logic::ON));
        wheel.push(make_event(0, 2, Logic::ON));
        wheel.push(make_event(2, 2, Logic::OFF));
        wheel.push(make_event(2, 3, Logic::OFF));
        let total_events = 4;

        let mut steps = 0;
        let mut popped_events = 0;

        while let Some(events) = wheel.pop() {
            println!("Popped {} events.", events.len());

            popped_events += events.len();
            steps += 1;

            if steps > 20 {
                panic!("Wheel did not terminate");
            }

            println!("{} step(s) taken", steps);

        }

        assert!(popped_events == total_events, "pushed {} events but popped {}", total_events, popped_events);
    }

    #[test]
    fn wrap_around() {
        let mut wheel: Wheel<TEST_SIZE, Logic> = Wheel::new();

        wheel.push(make_event(0, 0, Logic::OFF));
        wheel.push(make_event(4, 0, Logic::ON));

        let first_pop = wheel.pop();
        assert!(first_pop.is_some());
        let first_event = first_pop.unwrap();
        assert_eq!(first_event.len(), 1);

        for _ in 0..3 {
            wheel.pop();
        }

        let last_pop = wheel.pop();
        assert!(last_pop.is_some());
        let last_event = last_pop.unwrap();
        assert_eq!(last_event.len(), 1);
    }
}