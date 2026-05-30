pub type NetId = usize;
pub type GateId = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Logic {
    ON,
    OFF,
    X,
}

pub struct Net {
    pub value: Logic,
    pub sinks: Vec<GateId>,
}

pub struct Gate {
    pub a: NetId,
    pub b: NetId,
    pub out: NetId,
}

pub struct Circuit {
    pub gates: Vec<Gate>,
    pub nets: Vec<Net>,
}

impl Circuit {
    pub fn new() -> Self {
        Self {
            gates: Vec::new(),
            nets: Vec::new(),
        }
    }
}