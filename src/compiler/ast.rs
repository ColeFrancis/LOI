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
//! Last Updated: 06/27/2026

#[derive(PartialEq, Debug)]
pub struct Program {
    items: Vec<Item>,
}

#[derive(PartialEq, Debug)]
pub enum Item {
    Ent(EntDecl),
    Rel(RelDecl),
    Net(NetDecl),
}

////////////////////////////////////////////////////////////////////////////////
/// Common AST elements
////////////////////////////////////////////////////////////////////////////////

type Ident = String;

#[derive(PartialEq, Debug)]
pub enum Type {
    Bool,
    Int,
    Real,
    Mod(i64),
    CustomType(Ident),
}

#[derive(PartialEq, Debug)]
pub enum Expr {
    Literal(Literal),
    Ident(Ident),
    Unary(UnaryExpr),
    Binary(BinaryExpr),
    Tuple(Vec<Expr>),
    Match(MatchExpr),
    Sample(SampleExpr),
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
pub struct SampleExpr {
    arms: Vec<SampleArm>,
}

#[derive(PartialEq, Debug)]
pub struct SampleArm {
    prob: Expr,
    expr: Expr,
}

#[derive(PartialEq, Debug)]
pub struct Param {
    name: Ident,
    param_type: Type,
}

////////////////////////////////////////////////////////////////////////////////
/// Entities
////////////////////////////////////////////////////////////////////////////////

#[derive(PartialEq, Debug)]
pub struct EntDecl {
    name: Ident,
    expr: EntExpr,
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
pub struct RelDecl {
    name: Ident,
    params: Vec<Param>,
    return_type: Type,
    body: RelBody,
}

#[derive(PartialEq, Debug)]
pub enum RelBody {
    Expr(Expr),
    Block(BlockExpr),
}

#[derive(PartialEq, Debug)]
pub struct BlockExpr {
    statements: Vec<LetStatement>,
    expr: Expr,
}

#[derive(PartialEq, Debug)]
pub struct LetStatement {
    name: Ident,
    expr: Expr,
}

////////////////////////////////////////////////////////////////////////////////
/// Networks
////////////////////////////////////////////////////////////////////////////////

#[derive(PartialEq, Debug)]
pub struct NetDecl {
    name: Ident,
    items: Vec<NetItem>,
}

#[derive(PartialEq, Debug)]
pub enum NetItem {
    Input(Param),
    Output(Param),
    Init(NetInit),
    RelInst(RelInst),
}

#[derive(PartialEq, Debug)]
pub struct NetInit {
    param: Param,
    val: Expr,
}

#[derive(PartialEq, Debug)]
pub struct RelInst {
    asignee: Ident,
    rel: Ident,
    args: Vec<Expr>, 
}