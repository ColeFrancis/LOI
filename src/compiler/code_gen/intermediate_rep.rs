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

//! # intermediate_rep
//!
//! provides an intermediate bytecode representation before it is turned to u8
//!
//! ## Invariants
//!
//! - 
//!
//! Author: Cole Francis

#[derive(PartialEq, Debug, Clone)]
pub enum Source {
    RegInter(usize), // Registers for intermediate values that can be overridden
    RegVar(usize), // Registers that should not be overridden
    Bool(bool),
    Int(i64),
    Float(f64),
}

#[derive(PartialEq, Debug)]
pub enum Instruction {
    IADD {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    ISUB {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    IMUL {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    IDIV {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    IPOW {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    IABS {
        dest: usize,
        src: Source,
    },
    MOD {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    FADD {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    FSUB {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    FMUL {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    FDIV {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    FPOW {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    FABS {
        dest: usize,
        src: Source,
    },
    AND {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    OR {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    NOT {
        dest: usize,
        src: Source,
    },
    XOR {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    I2F {
        dest: usize,
        src: Source,
    },
    JMP {
        offset: u8,
    },
    IJEQ {
        offset: u8,
        src1: Source,
        src2: Source,
    },
    IJNE {
        offset: u8,
        src1: Source,
        src2: Source,
    },
    IJLT {
        offset: u8,
        src1: Source,
        src2: Source,
    },
    IJGT {
        offset: u8,
        src1: Source,
        src2: Source,
    },
    IJLE {
        offset: u8,
        src1: Source,
        src2: Source,
    },
    IJGE {
        offset: u8,
        src1: Source,
        src2: Source,
    },
    FJEQ {
        offset: u8,
        src1: Source,
        src2: Source,
    },
    FJNE {
        offset: u8,
        src1: Source,
        src2: Source,
    },
    FJLT {
        offset: u8,
        src1: Source,
        src2: Source,
    },
    FJGT {
        offset: u8,
        src1: Source,
        src2: Source,
    },
    FJLE {
        offset: u8,
        src1: Source,
        src2: Source,
    },
    FJGE {
        offset: u8,
        src1: Source,
        src2: Source,
    },
    IEQ {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    INE {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    ILT {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    IGT {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    ILE {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    IGE {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    FEQ {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    FNE {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    FLT {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    FGT {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    FLE {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    FGE {
        dest: usize,
        src1: Source,
        src2: Source,
    },
    MOV {
        dest: usize,
        src: Source,
    },
    RET {
        src: Source,
    },
    ERR {
        code: u8,
        src: Option<Source>,
    },
    RND {
        dest: usize,
    },
}