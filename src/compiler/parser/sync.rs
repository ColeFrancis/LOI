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

//! # sync
//!
//! Handles recovery after detecting an error in parsing
//!
//! ## Invariants
//!
//! - 
//!
//! Author: Cole Francis

use super::Parser;
use crate::compiler::token::{Token, TokenKind};
use crate::compiler::ast::*;

pub enum SyncRule {
    Item, // top level
    NetItem {
        depth: usize
    }, // inside net_t
    Statement,
    Expr {
        depth: usize
    },
}

impl<'a> Parser<'a> {
    // Never consume a } that belongs to the caller's block.
    // If you skip into a nested {...} block, skip the entire block before trying to resume.
    // Only recover at tokens that are valid starts of constructs at the current nesting level
    pub(super) fn sync(&mut self, rule: &SyncRule) {
        match rule {
            &SyncRule::Item => self.sync_item(),

            &SyncRule::NetItem {depth} => self.sync_net_item(depth),

            &SyncRule::Statement => self.sync_statement(),

            &SyncRule::Expr {depth} => self.sync_expr(depth),
        }
    }

    fn sync_item(&mut self) {
        let mut depth = 0;
        loop {
            match self.peek().kind {
                TokenKind::Eof => break,

                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => {
                    if depth > 0 {
                        depth -= 1;
                    }
                    else {
                        break;
                    }
                }
                
                TokenKind::Ent_t => break,
                TokenKind::Rel_t => break,
                TokenKind::NetToken => break,
                TokenKind::Let if depth == 0 => break,

                TokenKind::Semicolon if depth == 0 => {
                    self.next();
                    break;
                }

                _ => (),
            }

            self.next();
        }
    }

    fn sync_net_item(&mut self, mut depth: usize) {
        loop {
            match self.peek().kind {
                TokenKind::Eof => break,

                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => {
                    if depth > 0 {
                        depth -= 1;
                    }
                    else {
                        break;
                    }
                }

                TokenKind::Input => break,
                TokenKind::Output => break,
                TokenKind::Init => break,

                TokenKind::Ident(_) => match self.peek_n(2).kind {
                    TokenKind::Connect => break,
                    TokenKind::LBrace => break,
                    _ => (),
                }

                TokenKind::Semicolon if depth == 0 => {
                    self.next();
                    break;
                }

                _ => (),
            }

            self.next();
        }
    }

    fn sync_statement(&mut self) {
        let mut depth = 0;

        loop {
            match self.peek().kind {
                TokenKind::Eof => break,

                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => {
                    if depth > 0 {
                        depth -= 1;
                    }
                    else {
                        break;
                    }
                }

                TokenKind::Let if depth == 0 => break,

                TokenKind::Semicolon if depth == 0 => {
                    self.next();
                    break;
                }

                _ => (),
            }

            self.next();
        }
    }

    // Passing in depth allows for proper recovery from inside of sample {} and match {} brackets
    fn sync_expr(&mut self, mut depth: usize) {
        loop {
            match self.peek().kind {
                TokenKind::Eof => break,

                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => {
                    if depth > 0 {
                        depth -= 1;
                    }
                    else {
                        break;
                    }
                }
        
                TokenKind::Semicolon if depth == 0 => break,
                TokenKind::Comma if depth == 0 => break,

                _ => (),
            }

            self.next();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    
}