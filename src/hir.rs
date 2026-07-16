use crate::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClockedBlockId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TestbenchId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HardwareType {
    Bit,
    Unsigned(u32),
    Signed(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedProgram {
    pub modules: Vec<TypedModule>,
    pub testbenches: Vec<TypedTestbench>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedTestbench {
    pub id: TestbenchId,
    pub name: String,
    pub target: ModuleId,
    pub clocks: Vec<TypedTestbenchClock>,
    pub statements: Vec<TypedTestbenchStmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedTestbenchClock {
    pub signal: SignalId,
    pub period: SimulationTime,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimulationTime {
    pub value: u64,
    pub unit: crate::TimeUnit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedTestbenchStmt {
    Drive {
        target: SignalId,
        value: TypedExpr,
        span: Span,
    },
    Wait {
        duration: SimulationTime,
        span: Span,
    },
    WaitRising {
        clock: SignalId,
        count: u32,
        span: Span,
    },
    Assert {
        condition: TypedExpr,
        message: String,
        span: Span,
    },
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
