use crate::{Span, Spanned};

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub modules: Vec<Spanned<ModuleDecl>>,
    pub testbenches: Vec<Spanned<TestbenchDecl>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestbenchDecl {
    pub name: Identifier,
    pub target: Identifier,
    pub target_generics: Vec<Spanned<GenericBinding>>,
    pub clocks: Vec<Spanned<TestbenchClockDecl>>,
    pub stimulus: Vec<Spanned<TestbenchStmt>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestbenchClockDecl {
    pub signal: Identifier,
    pub period: TimeLiteral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeLiteral {
    pub value: u64,
    pub unit: TimeUnit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeUnit {
    Femtosecond,
    Picosecond,
    Nanosecond,
    Microsecond,
    Millisecond,
    Second,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TestbenchStmt {
    Drive {
        target: Identifier,
        value: Spanned<Expr>,
    },
    Wait {
        duration: TimeLiteral,
    },
    WaitRising {
        clock: Identifier,
        count: u32,
    },
    Assert {
        condition: Spanned<Expr>,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDecl {
    pub name: Identifier,
    pub generics: Vec<Spanned<GenericDecl>>,
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

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    Bit,
    Unsigned(u32),
    Signed(u32),
    SymbolicUnsigned(Spanned<ConstExprAst>),
    SymbolicSigned(Spanned<ConstExprAst>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericKindSyntax {
    Natural,
    Positive,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericDecl {
    pub name: Identifier,
    pub kind: GenericKindSyntax,
    pub default: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConstExprAst {
    Integer(u64),
    Reference(Identifier),
    Add(Box<Spanned<ConstExprAst>>, Box<Spanned<ConstExprAst>>),
    Multiply(Box<Spanned<ConstExprAst>>, Box<Spanned<ConstExprAst>>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericBinding {
    pub formal: Identifier,
    pub value: Spanned<ConstExprAst>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModuleItem {
    Wire(WireDecl),
    Register(RegisterDecl),
    Assign(AssignStmt),
    Clocked(ClockedDecl),
    Instance(InstanceDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstanceDecl {
    pub name: Identifier,
    pub module: Identifier,
    pub generics_span: Option<Span>,
    pub generics: Vec<Spanned<GenericBinding>>,
    pub ports_span: Span,
    pub ports: Vec<Spanned<PortConnection>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PortConnection {
    pub formal: Identifier,
    pub actual: Identifier,
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
