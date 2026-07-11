//! # sync
//!
//! Handles recovery after detecting an error in parsing
//!
//! ## Invariants
//!
//! - 
//!
//! Author: Cole Francis
//!
//! Last Updated: 07/010/2026

use super::Parser;
use crate::compiler::token::{Token, TokenKind};
use crate::compiler::ast::*;

pub enum SyncRule {
    Item, // top level
    Inst, // inside net_t
    Statement,
    Expr,
}

impl<'a> Parser<'a> {
    // Never consume a } that belongs to the caller's block.
    // If you skip into a nested {...} block, skip the entire block before trying to resume.
    // Only recover at tokens that are valid starts of constructs at the current nesting level
    pub(super) fn sync(&mut self, rule: SyncRule) {
        match rule {
            SyncRule::Item => self.sync_item(),

            SyncRule::Inst => self.sync_inst(),

            SyncRule::Statement => self.sync_statement(),

            SyncRule::Expr => self.sync_expr(),
        }
    }

    fn sync_item(&mut self) {

    }

    fn sync_inst(&mut self) {

    }

    fn sync_statement(&mut self) {

    }

    fn sync_expr(&mut self) {

    }
}