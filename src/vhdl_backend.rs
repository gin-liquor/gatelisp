use crate::{
    BinaryOp, ClockEdge, HardwareType, ModuleId, ResetKind, SignalId, SignalKind,
    TypedClockedBlock, TypedExpr, TypedExprKind, TypedModule, TypedNext, TypedProgram, UnaryOp,
    VhdlArchitecture, VhdlConcurrentStatement, VhdlDeclaration, VhdlDesign, VhdlDesignUnit,
    VhdlEntity, VhdlExpression, VhdlIdentifier, VhdlPort, VhdlPortMode, VhdlProcess,
    VhdlSensitivity, VhdlSequentialStatement, VhdlType, VhdlVariable,
};
use std::{collections::HashMap, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VhdlBackendErrorKind {
    InvalidTypeWidth,
    UnsupportedRegisterInitialValue,
    MissingSignal,
    InconsistentId,
    InvalidSignalKind,
    InvalidIdentifier,
    InvalidTypedExpression,
    IntegerRepresentation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VhdlBackendError {
    pub kind: VhdlBackendErrorKind,
    pub message: String,
}
impl VhdlBackendError {
    fn new(kind: VhdlBackendErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}
impl fmt::Display for VhdlBackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for VhdlBackendError {}

#[derive(Clone)]
struct SignalNames {
    read: VhdlIdentifier,
    port: Option<VhdlIdentifier>,
}

struct LowerContext {
    names: HashMap<SignalId, SignalNames>,
    temporary: u32,
    variables: Vec<VhdlVariable>,
}

pub fn lower_to_vhdl(program: &TypedProgram) -> Result<VhdlDesign, VhdlBackendError> {
    let mut units = Vec::new();
    for module in &program.modules {
        let (entity, architecture) = lower_module(module)?;
        units.push(VhdlDesignUnit::Entity(entity));
        units.push(VhdlDesignUnit::Architecture(architecture));
    }
    Ok(VhdlDesign { units })
}

fn lower_module(module: &TypedModule) -> Result<(VhdlEntity, VhdlArchitecture), VhdlBackendError> {
    let entity_name = module_name(module.id, &module.name);
    let mut ports = Vec::new();
    let mut declarations = Vec::new();
    let mut names = HashMap::new();
    for signal in &module.signals {
        let ty = lower_type(&signal.ty)?;
        let port_name = VhdlIdentifier(format!("gl_p{}_{}", signal.id.0, sanitize(&signal.name)));
        let internal = VhdlIdentifier(format!("gl_s{}_{}", signal.id.0, sanitize(&signal.name)));
        match &signal.kind {
            SignalKind::Input => {
                ports.push(VhdlPort {
                    name: port_name.clone(),
                    mode: VhdlPortMode::In,
                    ty: ty.clone(),
                });
                insert_name(
                    &mut names,
                    signal.id,
                    SignalNames {
                        read: port_name.clone(),
                        port: Some(port_name),
                    },
                )?;
            }
            SignalKind::Output => {
                ports.push(VhdlPort {
                    name: port_name.clone(),
                    mode: VhdlPortMode::Out,
                    ty: ty.clone(),
                });
                declarations.push(VhdlDeclaration::Signal {
                    name: internal.clone(),
                    ty: ty.clone(),
                    initial: None,
                });
                insert_name(
                    &mut names,
                    signal.id,
                    SignalNames {
                        read: internal,
                        port: Some(port_name),
                    },
                )?;
            }
            SignalKind::Wire => {
                declarations.push(VhdlDeclaration::Signal {
                    name: internal.clone(),
                    ty: ty.clone(),
                    initial: None,
                });
                insert_name(
                    &mut names,
                    signal.id,
                    SignalNames {
                        read: internal,
                        port: None,
                    },
                )?;
            }
            SignalKind::Register { initial } => {
                let initial = initial
                    .as_ref()
                    .map(|value| match value.kind {
                        TypedExprKind::Integer(integer) => integer_literal(integer, &value.ty),
                        _ => Err(VhdlBackendError::new(
                            VhdlBackendErrorKind::UnsupportedRegisterInitialValue,
                            "register initial value must be an integer literal",
                        )),
                    })
                    .transpose()?;
                declarations.push(VhdlDeclaration::Signal {
                    name: internal.clone(),
                    ty: ty.clone(),
                    initial,
                });
                insert_name(
                    &mut names,
                    signal.id,
                    SignalNames {
                        read: internal,
                        port: None,
                    },
                )?;
            }
        }
    }
    declarations.push(VhdlDeclaration::BoolToStdLogicFunction);
    let mut statements = Vec::new();
    for (index, assignment) in module.assignments.iter().enumerate() {
        let target = signal_name(&names, assignment.target)?.read.clone();
        let mut context = LowerContext {
            names: names.clone(),
            temporary: 0,
            variables: Vec::new(),
        };
        let mut body = Vec::new();
        let value = lower_expr(&assignment.value, &mut context, &mut body)?;
        body.push(VhdlSequentialStatement::SignalAssignment { target, value });
        statements.push(VhdlConcurrentStatement::Process(VhdlProcess {
            label: VhdlIdentifier(format!("gl_comb_{index}")),
            sensitivity: VhdlSensitivity::All,
            variables: context.variables,
            statements: body,
        }));
    }
    for block in &module.clocked_blocks {
        statements.push(VhdlConcurrentStatement::Process(lower_clocked(
            block, &names,
        )?));
    }
    for signal in &module.signals {
        if matches!(signal.kind, SignalKind::Output) {
            let mapping = signal_name(&names, signal.id)?;
            let port = mapping.port.clone().ok_or_else(|| {
                VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidSignalKind,
                    "output has no entity port",
                )
            })?;
            statements.push(VhdlConcurrentStatement::Assignment {
                target: port,
                value: VhdlExpression::Name(mapping.read.clone()),
            });
        }
    }
    Ok((
        VhdlEntity {
            name: entity_name.clone(),
            ports,
        },
        VhdlArchitecture {
            name: VhdlIdentifier("rtl".into()),
            entity_name,
            declarations,
            statements,
        },
    ))
}

fn lower_clocked(
    block: &TypedClockedBlock,
    names: &HashMap<SignalId, SignalNames>,
) -> Result<VhdlProcess, VhdlBackendError> {
    if block.edge != ClockEdge::Rising {
        return Err(VhdlBackendError::new(
            VhdlBackendErrorKind::InvalidTypedExpression,
            "unsupported clock edge",
        ));
    }
    let clock = signal_name(names, block.clock)?.read.clone();
    let mut context = LowerContext {
        names: names.clone(),
        temporary: 0,
        variables: Vec::new(),
    };
    let normal = lower_updates(&block.updates, &mut context)?;
    let rising = VhdlExpression::Call {
        function: VhdlIdentifier("rising_edge".into()),
        arguments: vec![VhdlExpression::Name(clock.clone())],
    };
    let (sensitivity, statements) = if let Some(reset) = &block.reset {
        let reset_name = signal_name(names, reset.signal)?.read.clone();
        let reset_condition = VhdlExpression::Binary {
            op: "=".into(),
            left: Box::new(VhdlExpression::Name(reset_name.clone())),
            right: Box::new(VhdlExpression::Literal("'1'".into())),
        };
        let reset_updates = lower_updates(&reset.updates, &mut context)?;
        match reset.kind {
            ResetKind::Synchronous => (
                VhdlSensitivity::Signals(vec![clock]),
                vec![VhdlSequentialStatement::If {
                    condition: rising,
                    then_statements: vec![VhdlSequentialStatement::If {
                        condition: reset_condition,
                        then_statements: reset_updates,
                        else_statements: normal,
                    }],
                    else_statements: vec![],
                }],
            ),
            ResetKind::Asynchronous => (
                VhdlSensitivity::Signals(vec![clock, reset_name]),
                vec![VhdlSequentialStatement::If {
                    condition: reset_condition,
                    then_statements: reset_updates,
                    else_statements: vec![VhdlSequentialStatement::If {
                        condition: rising,
                        then_statements: normal,
                        else_statements: vec![],
                    }],
                }],
            ),
        }
    } else {
        (
            VhdlSensitivity::Signals(vec![clock]),
            vec![VhdlSequentialStatement::If {
                condition: rising,
                then_statements: normal,
                else_statements: vec![],
            }],
        )
    };
    Ok(VhdlProcess {
        label: VhdlIdentifier(format!("gl_seq_{}", block.id.0)),
        sensitivity,
        variables: context.variables,
        statements,
    })
}

fn lower_updates(
    updates: &[TypedNext],
    context: &mut LowerContext,
) -> Result<Vec<VhdlSequentialStatement>, VhdlBackendError> {
    let mut statements = Vec::new();
    for update in updates {
        let target = signal_name(&context.names, update.target)?.read.clone();
        let value = lower_expr(&update.value, context, &mut statements)?;
        statements.push(VhdlSequentialStatement::SignalAssignment { target, value });
    }
    Ok(statements)
}

fn lower_expr(
    expr: &TypedExpr,
    context: &mut LowerContext,
    prelude: &mut Vec<VhdlSequentialStatement>,
) -> Result<VhdlExpression, VhdlBackendError> {
    match &expr.kind {
        TypedExprKind::Signal(id) => Ok(VhdlExpression::Name(
            signal_name(&context.names, *id)?.read.clone(),
        )),
        TypedExprKind::Integer(value) => integer_literal(*value, &expr.ty),
        TypedExprKind::Unary {
            op: UnaryOp::Not,
            operand,
        } => Ok(VhdlExpression::Unary {
            op: "not".into(),
            operand: Box::new(lower_expr(operand, context, prelude)?),
        }),
        TypedExprKind::Binary { op, left, right } => {
            let left = lower_expr(left, context, prelude)?;
            let right = lower_expr(right, context, prelude)?;
            let operator = binary_text(*op);
            let binary = VhdlExpression::Binary {
                op: operator.into(),
                left: Box::new(left),
                right: Box::new(right),
            };
            if matches!(
                op,
                BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::LessThan
                    | BinaryOp::LessEqual
                    | BinaryOp::GreaterThan
                    | BinaryOp::GreaterEqual
            ) {
                Ok(VhdlExpression::Call {
                    function: VhdlIdentifier("gl_bool_to_sl".into()),
                    arguments: vec![binary],
                })
            } else {
                Ok(binary)
            }
        }
        TypedExprKind::If {
            condition,
            when_true,
            when_false,
        } => {
            let condition = lower_expr(condition, context, prelude)?;
            let name = VhdlIdentifier(format!("gl_tmp_{}", context.temporary));
            context.temporary = context.temporary.checked_add(1).ok_or_else(|| {
                VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidIdentifier,
                    "too many temporary variables",
                )
            })?;
            context.variables.push(VhdlVariable {
                name: name.clone(),
                ty: lower_type(&expr.ty)?,
            });
            let mut then_statements = Vec::new();
            let when_true = lower_expr(when_true, context, &mut then_statements)?;
            then_statements.push(VhdlSequentialStatement::VariableAssignment {
                target: name.clone(),
                value: when_true,
            });
            let mut else_statements = Vec::new();
            let when_false = lower_expr(when_false, context, &mut else_statements)?;
            else_statements.push(VhdlSequentialStatement::VariableAssignment {
                target: name.clone(),
                value: when_false,
            });
            let condition = VhdlExpression::Binary {
                op: "=".into(),
                left: Box::new(condition),
                right: Box::new(VhdlExpression::Literal("'1'".into())),
            };
            prelude.push(VhdlSequentialStatement::If {
                condition,
                then_statements,
                else_statements,
            });
            Ok(VhdlExpression::Name(name))
        }
    }
}

fn lower_type(ty: &HardwareType) -> Result<VhdlType, VhdlBackendError> {
    match *ty {
        HardwareType::Bit => Ok(VhdlType::StdLogic),
        HardwareType::Unsigned(0) | HardwareType::Signed(0) => Err(VhdlBackendError::new(
            VhdlBackendErrorKind::InvalidTypeWidth,
            "VHDL vector width must be positive",
        )),
        HardwareType::Unsigned(width) => Ok(VhdlType::Unsigned(width)),
        HardwareType::Signed(width) => Ok(VhdlType::Signed(width)),
    }
}
fn integer_literal(value: i64, ty: &HardwareType) -> Result<VhdlExpression, VhdlBackendError> {
    let text = match ty {
        HardwareType::Bit if value == 0 => "'0'".into(),
        HardwareType::Bit if value == 1 => "'1'".into(),
        HardwareType::Bit => {
            return Err(VhdlBackendError::new(
                VhdlBackendErrorKind::IntegerRepresentation,
                "invalid bit literal",
            ));
        }
        HardwareType::Unsigned(width) if *width > 0 => {
            format!("resize(unsigned'(x\"{:016x}\"), {width})", value as u64)
        }
        HardwareType::Signed(width) if *width > 0 => {
            format!("resize(signed'(x\"{:016x}\"), {width})", value as u64)
        }
        _ => {
            return Err(VhdlBackendError::new(
                VhdlBackendErrorKind::InvalidTypeWidth,
                "integer has invalid target width",
            ));
        }
    };
    Ok(VhdlExpression::Literal(text))
}
fn binary_text(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::And => "and",
        BinaryOp::Or => "or",
        BinaryOp::Xor => "xor",
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "-",
        BinaryOp::Equal => "=",
        BinaryOp::NotEqual => "/=",
        BinaryOp::LessThan => "<",
        BinaryOp::LessEqual => "<=",
        BinaryOp::GreaterThan => ">",
        BinaryOp::GreaterEqual => ">=",
    }
}
fn signal_name(
    names: &HashMap<SignalId, SignalNames>,
    id: SignalId,
) -> Result<&SignalNames, VhdlBackendError> {
    names.get(&id).ok_or_else(|| {
        VhdlBackendError::new(
            VhdlBackendErrorKind::MissingSignal,
            format!("SignalId {} is missing", id.0),
        )
    })
}
fn insert_name(
    names: &mut HashMap<SignalId, SignalNames>,
    id: SignalId,
    value: SignalNames,
) -> Result<(), VhdlBackendError> {
    if names.insert(id, value).is_some() {
        Err(VhdlBackendError::new(
            VhdlBackendErrorKind::InconsistentId,
            format!("duplicate SignalId {}", id.0),
        ))
    } else {
        Ok(())
    }
}
fn module_name(id: ModuleId, name: &str) -> VhdlIdentifier {
    VhdlIdentifier(format!("gl_m{}_{}", id.0, sanitize(name)))
}
fn sanitize(name: &str) -> String {
    let value: String = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    if value.is_empty() { "x".into() } else { value }
}
