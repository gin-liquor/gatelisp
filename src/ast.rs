use crate::{Span, Spanned};

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub enums: Vec<Spanned<EnumDecl>>,
    pub roms: Vec<Spanned<RomDecl>>,
    pub modules: Vec<Spanned<ModuleDecl>>,
    pub testbenches: Vec<Spanned<TestbenchDecl>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RomDecl {
    pub name: Identifier,
    pub address_width: u32,
    pub data_width: u32,
    pub default_value: u64,
    pub entries: Vec<Spanned<RomEntryDecl>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RomEntryDecl {
    pub address: u64,
    pub value: u64,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub name: Identifier,
    pub width: u32,
    pub members: Vec<Spanned<EnumMemberDecl>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumMemberDecl {
    pub name: Identifier,
    pub value: u64,
    pub span: Span,
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
    Enum(Identifier),
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
    Subtract(Box<Spanned<ConstExprAst>>, Box<Spanned<ConstExprAst>>),
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
    RegisterArray(RegisterArrayDecl),
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
pub struct RegisterArrayDecl {
    pub name: Identifier,
    pub address_width: u32,
    pub data_width: u32,
    pub initial_value: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegisterArrayWrite {
    pub array: Identifier,
    pub address: Spanned<Expr>,
    pub value: Spanned<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssignStmt {
    pub target: Identifier,
    pub value: Spanned<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClockedDecl {
    pub clock: Identifier,
    pub edge: ClockEdgeSyntax,
    pub reset: Option<Spanned<ResetDecl>>,
    pub updates: Vec<Spanned<NextStmt>>,
    pub case_dos: Vec<Spanned<CaseDoStmt>>,
    pub writes: Vec<Spanned<RegisterArrayWrite>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaseDoStmt {
    pub selector: Spanned<Expr>,
    pub arms: Vec<Spanned<CaseDoArm>>,
    pub else_body: Option<Vec<Spanned<NextStmt>>>,
    pub else_writes: Vec<Spanned<RegisterArrayWrite>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaseDoArm {
    pub label: Spanned<Expr>,
    pub body: Vec<Spanned<NextStmt>>,
    pub writes: Vec<Spanned<RegisterArrayWrite>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockEdgeSyntax {
    Rising,
    Falling,
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
    Resize {
        target: Spanned<TypeExpr>,
        value: Box<Spanned<Expr>>,
    },
    Truncate {
        target: Spanned<TypeExpr>,
        value: Box<Spanned<Expr>>,
    },
    Slice {
        value: Box<Spanned<Expr>>,
        offset: Spanned<ConstExprAst>,
        width: Spanned<ConstExprAst>,
    },
    Concat {
        values: Vec<Spanned<Expr>>,
    },
    StaticBitMotion {
        kind: StaticBitMotionSyntaxKind,
        value: Box<Spanned<Expr>>,
        amount: Spanned<ConstExprAst>,
    },
    BitAt {
        source: Box<Spanned<Expr>>,
        index: Spanned<ConstExprAst>,
    },
    Case {
        selector: Box<Spanned<Expr>>,
        arms: Vec<Spanned<CaseExprArm>>,
        else_expr: Box<Spanned<Expr>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaseExprArm {
    pub label: Spanned<Expr>,
    pub result: Spanned<Expr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticBitMotionSyntaxKind {
    ShiftLeft,
    ShiftRightLogical,
    ShiftRightArithmetic,
    RotateLeft,
    RotateRight,
}
