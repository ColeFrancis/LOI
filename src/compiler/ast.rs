//! # ast
//!
//! Holds the structures used in creating the ast
//!
//! ## Invariants
//!
//! - Grammar shall be obeyed. It is the source of truth.
//!
//! Author: Cole Francis
//!
//! Last Updated: 07/07/2026

#[derive(PartialEq, Debug)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(PartialEq, Debug)]
pub enum Item {
    Let(LetStatement),
    Ent(EntType),
    Rel(RelType),
    Net(Net),
    Error,
}

////////////////////////////////////////////////////////////////////////////////
/// Common AST elements
////////////////////////////////////////////////////////////////////////////////

type Ident = String;

#[derive(PartialEq, Debug)]
pub enum Type {
    Bool,
    Impulse,
    Int,
    Real,
    Mod(i64),
    CustomType(Ident),
    // Error, // Maybe?
}

#[derive(PartialEq, Debug)]
pub enum Expr {
    Literal(Literal),
    Ident(Ident),
    Unary(UnaryExpr),
    Binary(BinaryExpr),
    Tuple(Vec<Expr>),
    Match(MatchExpr),
    Sample(Vec<SampleArm>),
    // Error,
}

#[derive(PartialEq, Debug)]
pub enum Literal {
    Bool(bool),
    Int(i64),
    Real(f64),
}

#[derive(PartialEq, Debug)]
pub struct UnaryExpr {
    pub op: UnaryOp,
    pub expr: Box<Expr>,
}

#[derive(PartialEq, Debug)]
pub enum UnaryOp {
    Neg,    // -
    BitNot, // ~
}

#[derive(PartialEq, Debug)]
pub struct BinaryExpr {
    pub left: Box<Expr>,
    pub op: BinaryOp,
    pub right: Box<Expr>,
}

#[derive(PartialEq, Debug)]
pub enum BinaryOp {
    Lt,         // <
    Gt,         // >
    Le,         // <=
    Ge,         // >=
    Add,        // +
    Sub,        // -
    Mul,        // *
    Div,        // /
    Pow,        // ^
}

#[derive(PartialEq, Debug)]
pub enum CompOp {
    Lt,         // <
    Gt,         // >
    Le,         // <=
    Ge,         // >=
}

#[derive(PartialEq, Debug)]
pub struct MatchExpr {
    pub scrutinee: Box<Expr>,
    pub arms: Vec<MatchArm>,
}

#[derive(PartialEq, Debug)]
pub struct MatchArm {
    pub pattern: Vec<SimplePattern>,
    pub expr: Expr,
}

#[derive(PartialEq, Debug)]
pub enum SimplePattern {
    Default,
    Literal(Literal),
    Ident(Ident),
    Tuple(Vec<SimplePattern>),
    Comparison(ComparisonPattern),
}

#[derive(PartialEq, Debug)]
pub struct ComparisonPattern {
    pub op: CompOp,
    pub expr: Box<Expr>,
}

#[derive(PartialEq, Debug)]
pub struct SampleArm {
    pub prob: Prob,
    pub expr: Expr,
}

#[derive(PartialEq, Debug)]
pub enum Prob {
    Default,
    Expr(Expr),
}

#[derive(PartialEq, Debug)]
pub struct Param {
    pub name: Ident,
    pub param_type: Type,
}

////////////////////////////////////////////////////////////////////////////////
/// Entities
////////////////////////////////////////////////////////////////////////////////

#[derive(PartialEq, Debug)]
pub struct EntType {
    pub name: Ident,
    pub expr: EntExpr,
}

#[derive(PartialEq, Debug)]
pub enum EntExpr {
    Type(Type),
    SetEnt(Vec<Ident>),
}

////////////////////////////////////////////////////////////////////////////////
/// Relations
////////////////////////////////////////////////////////////////////////////////

#[derive(PartialEq, Debug)]
pub struct RelType {
    pub name: Ident,
    pub params: Vec<Param>,
    pub return_type: Type,
    pub body: RelBody,
}

#[derive(PartialEq, Debug)]
pub enum RelBody {
    Expr(Expr),
    Block(BlockExpr),
}

#[derive(PartialEq, Debug)]
pub struct BlockExpr {
    pub statements: Vec<Statement>,
    pub expr: Expr,
}

#[derive(PartialEq, Debug)]
pub enum Statement {
    Let(LetStatement),
    Error,
}

#[derive(PartialEq, Debug)]
pub struct LetStatement {
    pub name: Ident,
    pub expr: Expr,
}

////////////////////////////////////////////////////////////////////////////////
/// Networks
////////////////////////////////////////////////////////////////////////////////

#[derive(PartialEq, Debug)]
pub struct Net {
    pub name: Ident,
    pub items: Vec<NetItem>,
}

#[derive(PartialEq, Debug)]
pub enum NetItem {
    Input(Param),
    Output(Param),
    Init(EntInit),
    RelInst(RelInst),
    NetInst(NetInst),
    // Error,
}

#[derive(PartialEq, Debug)]
pub struct EntInit {
    pub param: Param,
    pub val: Expr,
}

#[derive(PartialEq, Debug)]
pub struct RelInst {
    pub asignee: Ident,
    pub rel: Ident,
    pub args: Vec<Ident>, 
}

#[derive(PartialEq, Debug)]
pub struct NetInst {
    pub net: Ident,
    pub connections: Vec<Connection>,
}

#[derive(PartialEq, Debug)]
pub struct Connection {
    pub port: Ident,
    pub net: Ident,
}
