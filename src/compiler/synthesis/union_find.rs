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

//! # union_find
//!
//! implements a union-find datastructure to handle ent aliases in syntehesis
//!
//! ## Invariants
//!
//! - whenever two elements are unioned, the smaller parent idx becomes the root
//!
//! Author: Cole Francis

#[derive(Debug, Default)]
pub struct UnionFind {
    parent: Vec<usize>,
}

impl UnionFind {
    pub fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }

    pub fn len(& self) -> usize {
        self.parent.len()
    }

    pub fn add(&mut self) -> usize {
        let idx = self.parent.len();
        self.parent.push(idx);
        idx
    }

    pub fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }

        self.parent[x]
    }

    pub fn union(&mut self, x: usize, y: usize) {
        let root_x = self.find(x);
        let root_y = self.find(y);

        if root_x != root_y {
            let (smaller, larger) = if root_x < root_y {
                (root_x, root_y)
            } else {
                (root_y, root_x)
            };

            self.parent[larger] = smaller;
        }
    }
}