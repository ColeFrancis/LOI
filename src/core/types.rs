pub trait Resettable {
    fn reset() -> Self;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Logic {
    ON,
    OFF,
    X,
}

impl Resettable for Logic {
    fn reset() -> Self {
        Logic::X
    }
}

pub enum LogicOp {
    NAND,
}

#[derive(Clone, Copy, Debug)]
pub enum Real {
    Val(f64),
    X,
}

impl Resettable for Real {
    fn reset() -> Self {
        Real::X
    }
}

impl PartialEq for Real {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Real::X, Real::X) => true,
            _ => false,
        }
    }
}

pub enum RealOp {
    ADD,
    MUL,
}