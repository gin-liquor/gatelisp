#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VhdlDesign {
    pub units: Vec<VhdlDesignUnit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VhdlDesignUnit {
    Entity(VhdlEntity),
    Architecture(VhdlArchitecture),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VhdlIdentifier(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VhdlEntity {
    pub name: VhdlIdentifier,
    pub ports: Vec<VhdlPort>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VhdlPort {
    pub name: VhdlIdentifier,
    pub mode: VhdlPortMode,
    pub ty: VhdlType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VhdlPortMode {
    In,
    Out,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VhdlType {
    StdLogic,
    Unsigned(u32),
    Signed(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VhdlArchitecture {
    pub name: VhdlIdentifier,
    pub entity_name: VhdlIdentifier,
    pub declarations: Vec<VhdlDeclaration>,
    pub statements: Vec<VhdlConcurrentStatement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VhdlDeclaration {
    Signal {
        name: VhdlIdentifier,
        ty: VhdlType,
        initial: Option<VhdlExpression>,
    },
    BoolToStdLogicFunction,
    Constant {
        name: VhdlIdentifier,
        ty: String,
        value: VhdlExpression,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VhdlConcurrentStatement {
    Assignment {
        target: VhdlIdentifier,
        value: VhdlExpression,
    },
    Process(VhdlProcess),
    EntityInstance {
        label: VhdlIdentifier,
        entity: VhdlIdentifier,
        ports: Vec<(VhdlIdentifier, VhdlIdentifier)>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VhdlProcess {
    pub label: VhdlIdentifier,
    pub sensitivity: VhdlSensitivity,
    pub variables: Vec<VhdlVariable>,
    pub statements: Vec<VhdlSequentialStatement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VhdlSensitivity {
    None,
    All,
    Signals(Vec<VhdlIdentifier>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VhdlVariable {
    pub name: VhdlIdentifier,
    pub ty: VhdlType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VhdlSequentialStatement {
    SignalAssignment {
        target: VhdlIdentifier,
        value: VhdlExpression,
    },
    VariableAssignment {
        target: VhdlIdentifier,
        value: VhdlExpression,
    },
    If {
        condition: VhdlExpression,
        then_statements: Vec<VhdlSequentialStatement>,
        else_statements: Vec<VhdlSequentialStatement>,
    },
    WaitFor(VhdlExpression),
    WaitUntil(VhdlExpression),
    ForLoop {
        variable: VhdlIdentifier,
        from: u32,
        to: u32,
        statements: Vec<VhdlSequentialStatement>,
    },
    InfiniteLoop(Vec<VhdlSequentialStatement>),
    Assert {
        condition: VhdlExpression,
        message: String,
    },
    Report(String),
    Stop,
    Wait,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VhdlExpression {
    Name(VhdlIdentifier),
    Literal(String),
    Unary {
        op: String,
        operand: Box<VhdlExpression>,
    },
    Binary {
        op: String,
        left: Box<VhdlExpression>,
        right: Box<VhdlExpression>,
    },
    Call {
        function: VhdlIdentifier,
        arguments: Vec<VhdlExpression>,
    },
}
