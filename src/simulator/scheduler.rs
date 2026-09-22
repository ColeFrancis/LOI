// Copyright 2026 Cole Francis
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # scheduler
//!
//! This module forms the core event scheduler of the simulator
//!
//! ## Invariants
//!
//! - The scheduler shall be able to maintain the order of events, and return all events occuring at a particular time
//! - One should be able to push events to the scheduler at any time as long as they occur later than the current simulator time
//!
//! Author: Cole Francis

use super::event::Event;

const WHEEL_SIZE: usize = 256;

pub struct Scheduler {
    pub curr_time: usize,
    event_count: usize,
    events: [Vec<Event>; WHEEL_SIZE],
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            curr_time: 0,
            event_count: 0,
            events: std::array::from_fn(|_| Vec::new()),
        }
    }

    pub fn push(&mut self, event: Event) {
        self.events[event.timestep % WHEEL_SIZE].push(event);
        self.event_count += 1;
    }

    pub fn push_many(&mut self, events: Vec<Event>) {
        for event in events {
            self.events[event.timestep % WHEEL_SIZE].push(event);
            self.event_count += 1;
        }
    }

    pub fn pop(&mut self) -> Option<Vec<Event>> {
        if self.event_count == 0 {
            return None;
        }

        let curr_events: Vec<Event> = self.events[self.curr_time % WHEEL_SIZE]
            .extract_if(.., |e| e.timestep == self.curr_time)
            .collect();

        self.event_count -= curr_events.len();
        self.curr_time += 1;

        Some(curr_events)
    }
}