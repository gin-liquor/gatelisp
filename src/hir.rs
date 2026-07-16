use crate::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClockedBlockId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HardwareType {
    Bit,
    Unsigned(u32),
    Signed(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedProgram {
    pub modules: Vec<TypedModule>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedModule {
    pub id: ModuleId,
    pub name: String,
    pub name_span: Span,
    pub signals: Vec<TypedSignal>,
    pub assignments: Vec<TypedAssign>,
    pub clocked_blocks: Vec<TypedClockedBlock>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedSignal {
    pub id: SignalId,
    pub name: String,
    pub kind: SignalKind,
    pub ty: HardwareType,
    pub declaration_span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SignalKind {
    Input,
    Output,
    Wire,
    Register { initial: Option<TypedExpr> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedAssign {
    pub target: SignalId,
    pub value: TypedExpr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedClockedBlock {
    pub id: ClockedBlockId,
    pub clock: SignalId,
    pub edge: ClockEdge,
    pub reset: Option<TypedReset>,
    pub updates: Vec<TypedNext>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockEdge {
    Rising,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedReset {
    pub kind: crate::ResetKind,
    pub signal: SignalId,
    pub updates: Vec<TypedNext>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedNext {
    pub target: SignalId,
    pub value: TypedExpr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedExpr {
    pub kind: TypedExprKind,
    pub ty: HardwareType,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedExprKind {
    Signal(SignalId),
    Integer(i64),
    Unary {
        op: UnaryOp,
        operand: Box<TypedExpr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<TypedExpr>,
        right: Box<TypedExpr>,
    },
    If {
        condition: Box<TypedExpr>,
        when_true: Box<TypedExpr>,
        when_false: Box<TypedExpr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    And,
    Or,
    Xor,
    Add,
    Subtract,
    Equal,
    NotEqual,
    LessThan,
    LessEqual,
    GreaterThan,
    GreaterEqual,
}
