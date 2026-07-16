use crate::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClockedBlockId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TestbenchId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InstanceId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenericId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnumId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnumMemberId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RomId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegisterArrayId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GenericKind {
    Natural,
    Positive,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WidthExpr {
    Constant(u64),
    Generic(GenericId),
    Add(Box<WidthExpr>, Box<WidthExpr>),
    Multiply(Box<WidthExpr>, Box<WidthExpr>),
    Subtract(Box<WidthExpr>, Box<WidthExpr>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HardwareType {
    Bit,
    Unsigned(u32),
    Signed(u32),
    SymbolicUnsigned(WidthExpr),
    SymbolicSigned(WidthExpr),
    Enum(EnumId, u32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedProgram {
    pub enums: Vec<TypedEnum>,
    pub roms: Vec<TypedRom>,
    pub modules: Vec<TypedModule>,
    pub testbenches: Vec<TypedTestbench>,
    pub module_order: Vec<ModuleId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedRom {
    pub id: RomId,
    pub name: String,
    pub address_width: u32,
    pub data_width: u32,
    pub depth: u64,
    pub default_value: u64,
    pub entries: Vec<TypedRomEntry>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedRomEntry {
    pub address: u64,
    pub value: u64,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedEnum {
    pub id: EnumId,
    pub name: String,
    pub width: u32,
    pub members: Vec<TypedEnumMember>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedEnumMember {
    pub id: EnumMemberId,
    pub enum_id: EnumId,
    pub name: String,
    pub value: u64,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedTestbench {
    pub id: TestbenchId,
    pub name: String,
    pub target: ModuleId,
    pub target_generic_bindings: Vec<TypedGenericBinding>,
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
    pub enums: Vec<TypedEnum>,
    pub roms: Vec<TypedRom>,
    pub register_arrays: Vec<TypedRegisterArray>,
    pub id: ModuleId,
    pub name: String,
    pub name_span: Span,
    pub generics: Vec<TypedGeneric>,
    pub signals: Vec<TypedSignal>,
    pub assignments: Vec<TypedAssign>,
    pub clocked_blocks: Vec<TypedClockedBlock>,
    pub instances: Vec<TypedInstance>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedRegisterArray {
    pub id: RegisterArrayId,
    pub name: String,
    pub address_width: u32,
    pub data_width: u32,
    pub depth: u64,
    pub initial_value: u64,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedInstance {
    pub id: InstanceId,
    pub name: String,
    pub target_module: ModuleId,
    pub generic_bindings: Vec<TypedGenericBinding>,
    pub connections: Vec<TypedPortConnection>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedGeneric {
    pub id: GenericId,
    pub name: String,
    pub kind: GenericKind,
    pub default: u64,
    pub declaration_span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedGenericBinding {
    pub formal: GenericId,
    pub value: WidthExpr,
    pub uses_default: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedPortConnection {
    pub formal: SignalId,
    pub actual: SignalId,
    pub direction: crate::PortDirection,
    pub ty: HardwareType,
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
    pub case_dos: Vec<TypedCaseDo>,
    pub writes: Vec<TypedRegisterArrayWrite>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedCaseDo {
    pub selector: TypedExpr,
    pub arms: Vec<TypedCaseDoArm>,
    pub else_body: Option<Vec<TypedNext>>,
    pub span: Span,
    pub else_writes: Vec<TypedRegisterArrayWrite>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedCaseDoArm {
    pub key: CaseKey,
    pub body: Vec<TypedNext>,
    pub span: Span,
    pub writes: Vec<TypedRegisterArrayWrite>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedRegisterArrayWrite {
    pub array_id: RegisterArrayId,
    pub address: TypedExpr,
    pub value: TypedExpr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseKey {
    pub value: i64,
    pub ty: HardwareType,
    pub span: Span,
    pub enum_member: Option<EnumMemberId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockEdge {
    Rising,
    Falling,
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
    Convert {
        kind: ConversionKind,
        value: Box<TypedExpr>,
        target_type: HardwareType,
    },
    Slice {
        value: Box<TypedExpr>,
        offset: WidthExpr,
        width: WidthExpr,
    },
    Concat {
        values: Vec<TypedExpr>,
    },
    ReverseBits {
        value: Box<TypedExpr>,
    },
    StaticBitMotion {
        kind: StaticBitMotionKind,
        value: Box<TypedExpr>,
        amount: WidthExpr,
    },
    BitAt {
        source: Box<TypedExpr>,
        index: WidthExpr,
    },
    EnumValue {
        enum_id: EnumId,
        member_id: EnumMemberId,
        value: u64,
    },
    EnumFromBits {
        enum_id: EnumId,
        value: Box<TypedExpr>,
    },
    EnumToBits {
        enum_id: EnumId,
        value: Box<TypedExpr>,
    },
    RomRead {
        rom_id: RomId,
        address: Box<TypedExpr>,
        address_width: u32,
        data_width: u32,
    },
    RegisterArrayRead {
        array_id: RegisterArrayId,
        address: Box<TypedExpr>,
        address_width: u32,
        data_width: u32,
    },
    Case {
        selector: Box<TypedExpr>,
        arms: Vec<TypedCaseExprArm>,
        else_expr: Box<TypedExpr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedCaseExprArm {
    pub key: CaseKey,
    pub result: TypedExpr,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticBitMotionKind {
    ShiftLeft,
    ShiftRightLogical,
    ShiftRightArithmetic,
    RotateLeft,
    RotateRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionKind {
    Resize,
    Truncate,
    AsSigned,
    AsUnsigned,
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
