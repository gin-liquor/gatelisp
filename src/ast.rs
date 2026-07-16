use crate::{Span, Spanned};

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub modules: Vec<Spanned<ModuleDecl>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDecl {
    pub name: Identifier,
    pub ports: Vec<Spanned<PortDecl>>,
    pub items: Vec<Spanned<ModuleItem>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identifier {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PortDecl {
    pub name: Identifier,
    pub direction: PortDirection,
    pub ty: Spanned<TypeExpr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortDirection {
    Input,
    Output,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeExpr {
    Bit,
    Unsigned(u32),
    Signed(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModuleItem {
    Wire(WireDecl),
    Register(RegisterDecl),
    Assign(AssignStmt),
    Clocked(ClockedDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct WireDecl {
    pub name: Identifier,
    pub ty: Spanned<TypeExpr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegisterDecl {
    pub name: Identifier,
    pub ty: Spanned<TypeExpr>,
    pub initial: Option<Spanned<Expr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssignStmt {
    pub target: Identifier,
    pub value: Spanned<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClockedDecl {
    pub clock: Identifier,
    pub reset: Option<Spanned<ResetDecl>>,
    pub updates: Vec<Spanned<NextStmt>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResetDecl {
    pub kind: ResetKind,
    pub signal: Identifier,
    pub updates: Vec<Spanned<NextStmt>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetKind {
    Synchronous,
    Asynchronous,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NextStmt {
    pub target: Identifier,
    pub value: Spanned<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Reference(Identifier),
    Integer(i64),
    Call {
        callee: Identifier,
        arguments: Vec<Spanned<Expr>>,
    },
}
