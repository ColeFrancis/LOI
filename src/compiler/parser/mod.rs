mod core;
mod ent;
mod rel;
mod net;
mod expr;
mod sync;

use crate::compiler::token::Token;
use crate::compiler::diagnostics::Diagnostics;

pub struct Parser<'a> {
    tokens: Vec<Token>,
    current: usize,
    diagnostics: &'a mut Diagnostics,
}