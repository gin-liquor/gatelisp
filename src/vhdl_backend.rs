use crate::{
    BinaryOp, ClockEdge, ConversionKind, GenericKind, HardwareType, ModuleId, PortDirection,
    ResetKind, SignalId, SignalKind, TimeUnit, TypedClockedBlock, TypedExpr, TypedExprKind,
    TypedModule, TypedNext, TypedProgram, TypedTestbench, TypedTestbenchStmt, UnaryOp,
    VhdlArchitecture, VhdlConcurrentStatement, VhdlDeclaration, VhdlDesign, VhdlDesignUnit,
    VhdlEntity, VhdlExpression, VhdlGeneric, VhdlGenericKind, VhdlIdentifier, VhdlPort,
    VhdlPortMode, VhdlProcess, VhdlSensitivity, VhdlSequentialStatement, VhdlType, VhdlVariable,
    WidthExpr,
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
    for module_id in &program.module_order {
        let module = program
            .modules
            .iter()
            .find(|module| module.id == *module_id)
            .ok_or_else(|| {
                VhdlBackendError::new(
                    VhdlBackendErrorKind::InconsistentId,
                    "module_order contains unknown ModuleId",
                )
            })?;
        let (entity, architecture) = lower_module(module, &program.modules)?;
        units.push(VhdlDesignUnit::Entity(entity));
        units.push(VhdlDesignUnit::Architecture(architecture));
    }
    for testbench in &program.testbenches {
        let module = program
            .modules
            .iter()
            .find(|module| module.id == testbench.target)
            .ok_or_else(|| {
                VhdlBackendError::new(
                    VhdlBackendErrorKind::InconsistentId,
                    "testbench target ModuleId is missing",
                )
            })?;
        let (entity, architecture) = lower_testbench(testbench, module)?;
        units.push(VhdlDesignUnit::Entity(entity));
        units.push(VhdlDesignUnit::Architecture(architecture));
    }
    Ok(VhdlDesign { units })
}

fn lower_module(
    module: &TypedModule,
    modules: &[TypedModule],
) -> Result<(VhdlEntity, VhdlArchitecture), VhdlBackendError> {
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
    let (need_unsigned_truncate, need_signed_truncate) = module_truncate_helpers(module);
    if need_unsigned_truncate {
        declarations.push(VhdlDeclaration::TruncateUnsignedFunction);
    }
    if need_signed_truncate {
        declarations.push(VhdlDeclaration::TruncateSignedFunction);
    }
    if module_needs_bit_concat(module) {
        declarations.push(VhdlDeclaration::BitToVectorFunction);
    }
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
    for instance in &module.instances {
        let target = modules
            .iter()
            .find(|target| target.id == instance.target_module)
            .ok_or_else(|| {
                VhdlBackendError::new(
                    VhdlBackendErrorKind::InconsistentId,
                    "TypedInstance target ModuleId is missing",
                )
            })?;
        let mut ports = Vec::new();
        for connection in &instance.connections {
            let formal = target
                .signals
                .iter()
                .find(|signal| signal.id == connection.formal)
                .ok_or_else(|| {
                    VhdlBackendError::new(
                        VhdlBackendErrorKind::MissingSignal,
                        "formal SignalId is missing",
                    )
                })?;
            if !matches!(formal.kind, SignalKind::Input | SignalKind::Output) {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidSignalKind,
                    "formal SignalId is not a port",
                ));
            }
            let formal_direction = match formal.kind {
                SignalKind::Input => PortDirection::Input,
                SignalKind::Output => PortDirection::Output,
                _ => {
                    return Err(VhdlBackendError::new(
                        VhdlBackendErrorKind::InvalidSignalKind,
                        "formal SignalId is not a port",
                    ));
                }
            };
            if connection.direction != formal_direction {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "instance connection disagrees with its formal port",
                ));
            }
            let actual_signal = module
                .signals
                .iter()
                .find(|signal| signal.id == connection.actual)
                .ok_or_else(|| {
                    VhdlBackendError::new(
                        VhdlBackendErrorKind::MissingSignal,
                        "actual SignalId is missing",
                    )
                })?;
            if actual_signal.ty != connection.ty
                || (connection.direction == PortDirection::Output
                    && !matches!(actual_signal.kind, SignalKind::Output | SignalKind::Wire))
            {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "instance actual is incompatible with its formal port",
                ));
            }
            let actual = signal_name(&names, connection.actual)?.read.clone();
            ports.push((
                VhdlIdentifier(format!("gl_p{}_{}", formal.id.0, sanitize(&formal.name))),
                actual,
            ));
        }
        statements.push(VhdlConcurrentStatement::EntityInstance {
            label: VhdlIdentifier(format!(
                "gl_i{}_{}",
                instance.id.0,
                sanitize(&instance.name)
            )),
            entity: module_name(target.id, &target.name),
            generics: instance
                .generic_bindings
                .iter()
                .filter(|b| !b.uses_default)
                .map(|b| Ok((generic_name(b.formal), lower_width(&b.value)?)))
                .collect::<Result<Vec<_>, VhdlBackendError>>()?,
            ports,
        });
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
            generics: module
                .generics
                .iter()
                .map(|g| VhdlGeneric {
                    name: generic_name(g.id),
                    kind: if g.kind == GenericKind::Natural {
                        VhdlGenericKind::Natural
                    } else {
                        VhdlGenericKind::Positive
                    },
                    default: g.default,
                })
                .collect(),
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

fn lower_testbench(
    testbench: &TypedTestbench,
    module: &TypedModule,
) -> Result<(VhdlEntity, VhdlArchitecture), VhdlBackendError> {
    let entity_name = VhdlIdentifier(format!(
        "gl_tb{}_{}",
        testbench.id.0,
        sanitize(&testbench.name)
    ));
    let mut declarations = vec![VhdlDeclaration::BoolToStdLogicFunction];
    let (need_unsigned_truncate, need_signed_truncate) = testbench_truncate_helpers(testbench);
    if need_unsigned_truncate {
        declarations.push(VhdlDeclaration::TruncateUnsignedFunction);
    }
    if need_signed_truncate {
        declarations.push(VhdlDeclaration::TruncateSignedFunction);
    }
    if testbench_needs_bit_concat(testbench) {
        declarations.push(VhdlDeclaration::BitToVectorFunction);
    }
    let mut names = HashMap::new();
    let mut port_map = Vec::new();
    for signal in &module.signals {
        let is_input = matches!(signal.kind, SignalKind::Input);
        if !is_input && !matches!(signal.kind, SignalKind::Output) {
            continue;
        }
        let concrete_ty = instantiate_type(&signal.ty, &testbench.target_generic_bindings)?;
        let ty = lower_type(&concrete_ty)?;
        let tb_name = VhdlIdentifier(format!("gl_tb_s{}_{}", signal.id.0, sanitize(&signal.name)));
        let initial = if is_input {
            Some(zero_value(&ty))
        } else {
            None
        };
        declarations.push(VhdlDeclaration::Signal {
            name: tb_name.clone(),
            ty: ty.clone(),
            initial,
        });
        let formal = VhdlIdentifier(format!("gl_p{}_{}", signal.id.0, sanitize(&signal.name)));
        port_map.push((formal, tb_name.clone()));
        insert_name(
            &mut names,
            signal.id,
            SignalNames {
                read: tb_name,
                port: None,
            },
        )?;
    }
    let mut statements = vec![VhdlConcurrentStatement::EntityInstance {
        label: VhdlIdentifier("gl_dut".into()),
        entity: module_name(module.id, &module.name),
        generics: testbench
            .target_generic_bindings
            .iter()
            .filter(|b| !b.uses_default)
            .map(|b| Ok((generic_name(b.formal), lower_width(&b.value)?)))
            .collect::<Result<Vec<_>, VhdlBackendError>>()?,
        ports: port_map,
    }];
    for (index, clock) in testbench.clocks.iter().enumerate() {
        let signal = signal_name(&names, clock.signal)?.read.clone();
        let constant = VhdlIdentifier(format!("gl_clk_period_{index}"));
        declarations.push(VhdlDeclaration::Constant {
            name: constant.clone(),
            ty: "time".into(),
            value: time_expression(&clock.period),
        });
        statements.push(VhdlConcurrentStatement::Process(VhdlProcess {
            label: VhdlIdentifier(format!("gl_clock_{index}")),
            sensitivity: VhdlSensitivity::None,
            variables: vec![],
            statements: vec![VhdlSequentialStatement::InfiniteLoop(vec![
                VhdlSequentialStatement::WaitFor(VhdlExpression::Binary {
                    op: "/".into(),
                    left: Box::new(VhdlExpression::Name(constant)),
                    right: Box::new(VhdlExpression::Literal("2".into())),
                }),
                VhdlSequentialStatement::SignalAssignment {
                    target: signal.clone(),
                    value: VhdlExpression::Unary {
                        op: "not".into(),
                        operand: Box::new(VhdlExpression::Name(signal)),
                    },
                },
            ])],
        }));
    }
    let mut context = LowerContext {
        names,
        temporary: 0,
        variables: Vec::new(),
    };
    let mut stimulus = Vec::new();
    for statement in &testbench.statements {
        match statement {
            TypedTestbenchStmt::Drive { target, value, .. } => {
                let target = signal_name(&context.names, *target)?.read.clone();
                let value = lower_expr(value, &mut context, &mut stimulus)?;
                stimulus.push(VhdlSequentialStatement::SignalAssignment { target, value });
            }
            TypedTestbenchStmt::Wait { duration, .. } => {
                stimulus.push(VhdlSequentialStatement::WaitFor(time_expression(duration)))
            }
            TypedTestbenchStmt::WaitRising { clock, count, .. } => {
                let clock = signal_name(&context.names, *clock)?.read.clone();
                let wait = VhdlSequentialStatement::WaitUntil(VhdlExpression::Call {
                    function: VhdlIdentifier("rising_edge".into()),
                    arguments: vec![VhdlExpression::Name(clock)],
                });
                if *count == 1 {
                    stimulus.push(wait);
                } else {
                    stimulus.push(VhdlSequentialStatement::ForLoop {
                        variable: VhdlIdentifier("gl_wait_index".into()),
                        from: 1,
                        to: *count,
                        statements: vec![wait],
                    });
                }
                stimulus.push(VhdlSequentialStatement::WaitFor(VhdlExpression::Literal(
                    "1 fs".into(),
                )));
            }
            TypedTestbenchStmt::Assert {
                condition, message, ..
            } => {
                let condition = lower_expr(condition, &mut context, &mut stimulus)?;
                stimulus.push(VhdlSequentialStatement::Assert {
                    condition: VhdlExpression::Binary {
                        op: "=".into(),
                        left: Box::new(condition),
                        right: Box::new(VhdlExpression::Literal("'1'".into())),
                    },
                    message: message.clone(),
                });
            }
        }
    }
    stimulus.push(VhdlSequentialStatement::Report(format!(
        "GateLisp testbench passed: {}",
        testbench.name
    )));
    stimulus.push(VhdlSequentialStatement::Stop);
    stimulus.push(VhdlSequentialStatement::Wait);
    statements.push(VhdlConcurrentStatement::Process(VhdlProcess {
        label: VhdlIdentifier("gl_stimulus".into()),
        sensitivity: VhdlSensitivity::None,
        variables: context.variables,
        statements: stimulus,
    }));
    Ok((
        VhdlEntity {
            name: entity_name.clone(),
            generics: vec![],
            ports: vec![],
        },
        VhdlArchitecture {
            name: VhdlIdentifier("sim".into()),
            entity_name,
            declarations,
            statements,
        },
    ))
}

fn zero_value(ty: &VhdlType) -> VhdlExpression {
    match ty {
        VhdlType::StdLogic => VhdlExpression::Literal("'0'".into()),
        VhdlType::Unsigned(_) | VhdlType::Signed(_) => {
            VhdlExpression::Literal("(others => '0')".into())
        }
    }
}
fn time_expression(time: &crate::SimulationTime) -> VhdlExpression {
    let unit = match time.unit {
        TimeUnit::Femtosecond => "fs",
        TimeUnit::Picosecond => "ps",
        TimeUnit::Nanosecond => "ns",
        TimeUnit::Microsecond => "us",
        TimeUnit::Millisecond => "ms",
        TimeUnit::Second => "sec",
    };
    VhdlExpression::Literal(format!("{} {unit}", time.value))
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
        TypedExprKind::Convert {
            kind,
            value,
            target_type,
        } => {
            if &expr.ty != target_type {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "conversion result type is inconsistent",
                ));
            }
            validate_conversion(*kind, &value.ty, target_type)?;
            let source = lower_expr(value, context, prelude)?;
            let function = match kind {
                ConversionKind::Resize => "resize",
                ConversionKind::Truncate => {
                    if matches!(
                        target_type,
                        HardwareType::Unsigned(_) | HardwareType::SymbolicUnsigned(_)
                    ) {
                        "gl_truncate_unsigned"
                    } else if matches!(
                        target_type,
                        HardwareType::Signed(_) | HardwareType::SymbolicSigned(_)
                    ) {
                        "gl_truncate_signed"
                    } else {
                        return Err(VhdlBackendError::new(
                            VhdlBackendErrorKind::InvalidTypedExpression,
                            "truncate target must be a vector",
                        ));
                    }
                }
                ConversionKind::AsSigned => "signed",
                ConversionKind::AsUnsigned => "unsigned",
            };
            let mut arguments = vec![source];
            if matches!(kind, ConversionKind::Resize | ConversionKind::Truncate) {
                arguments.push(lower_width(&hardware_width(target_type).ok_or_else(
                    || {
                        VhdlBackendError::new(
                            VhdlBackendErrorKind::InvalidTypedExpression,
                            "conversion target must be a vector",
                        )
                    },
                )?)?);
            }
            Ok(VhdlExpression::Call {
                function: VhdlIdentifier(function.into()),
                arguments,
            })
        }
        TypedExprKind::Slice {
            value,
            offset,
            width,
        } => {
            let source_width = hardware_width(&value.ty).ok_or_else(|| {
                VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "slice source is not a vector",
                )
            })?;
            let result_width = hardware_width(&expr.ty).ok_or_else(|| {
                VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "slice result is not a vector",
                )
            })?;
            if !matches!(
                expr.ty,
                HardwareType::Unsigned(_) | HardwareType::SymbolicUnsigned(_)
            ) || result_width != *width
            {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "slice result type is inconsistent",
                ));
            }
            if let (
                WidthExpr::Constant(source),
                WidthExpr::Constant(offset),
                WidthExpr::Constant(width),
            ) = (&source_width, offset, width)
                && offset.checked_add(*width).is_none_or(|end| end > *source)
            {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "slice range is out of bounds",
                ));
            }
            let lowered = lower_expr(value, context, prelude)?;
            let source = if matches!(value.kind, TypedExprKind::Signal(_)) {
                lowered
            } else {
                let name = VhdlIdentifier(format!("gl_tmp_{}", context.temporary));
                context.temporary = context.temporary.checked_add(1).ok_or_else(|| {
                    VhdlBackendError::new(
                        VhdlBackendErrorKind::InvalidIdentifier,
                        "too many temporary variables",
                    )
                })?;
                context.variables.push(VhdlVariable {
                    name: name.clone(),
                    ty: lower_type(&value.ty)?,
                });
                prelude.push(VhdlSequentialStatement::VariableAssignment {
                    target: name.clone(),
                    value: lowered,
                });
                VhdlExpression::Name(name)
            };
            let low = lower_width(offset)?;
            let high = VhdlExpression::Binary {
                op: "-".into(),
                left: Box::new(VhdlExpression::Binary {
                    op: "+".into(),
                    left: Box::new(lower_width(offset)?),
                    right: Box::new(lower_width(width)?),
                }),
                right: Box::new(VhdlExpression::Literal("1".into())),
            };
            Ok(VhdlExpression::Call {
                function: VhdlIdentifier("unsigned".into()),
                arguments: vec![VhdlExpression::Slice {
                    value: Box::new(source),
                    high: Box::new(high),
                    low: Box::new(low),
                }],
            })
        }
        TypedExprKind::Concat { values } => {
            if values.len() < 2 {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "typed concat requires at least two operands",
                ));
            }
            if values
                .iter()
                .any(|value| matches!(value.kind, TypedExprKind::Integer(_)))
            {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "concat integer operand has no width",
                ));
            }
            let mut expected = WidthExpr::Constant(0);
            for value in values {
                expected = add_width_backend(
                    expected,
                    operand_width_backend(&value.ty).ok_or_else(|| {
                        VhdlBackendError::new(
                            VhdlBackendErrorKind::InvalidTypedExpression,
                            "invalid concat operand",
                        )
                    })?,
                )?;
            }
            if !matches!(
                expr.ty,
                HardwareType::Unsigned(_) | HardwareType::SymbolicUnsigned(_)
            ) || hardware_width(&expr.ty) != Some(expected)
            {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "concat result width is inconsistent",
                ));
            }
            let mut parts = Vec::new();
            for value in values {
                let lowered = lower_expr(value, context, prelude)?;
                let function = if value.ty == HardwareType::Bit {
                    "gl_bit_to_slv"
                } else {
                    "std_logic_vector"
                };
                parts.push(VhdlExpression::Call {
                    function: VhdlIdentifier(function.into()),
                    arguments: vec![lowered],
                });
            }
            Ok(VhdlExpression::Call {
                function: VhdlIdentifier("unsigned".into()),
                arguments: vec![VhdlExpression::Concatenate(parts)],
            })
        }
    }
}

fn hardware_width(ty: &HardwareType) -> Option<WidthExpr> {
    match ty {
        HardwareType::Unsigned(v) | HardwareType::Signed(v) => {
            Some(WidthExpr::Constant(u64::from(*v)))
        }
        HardwareType::SymbolicUnsigned(v) | HardwareType::SymbolicSigned(v) => Some(v.clone()),
        HardwareType::Bit => None,
    }
}
fn operand_width_backend(ty: &HardwareType) -> Option<WidthExpr> {
    if *ty == HardwareType::Bit {
        Some(WidthExpr::Constant(1))
    } else {
        hardware_width(ty)
    }
}
fn add_width_backend(left: WidthExpr, right: WidthExpr) -> Result<WidthExpr, VhdlBackendError> {
    match (left, right) {
        (WidthExpr::Constant(0), value) | (value, WidthExpr::Constant(0)) => Ok(value),
        (WidthExpr::Constant(a), WidthExpr::Constant(b)) => {
            a.checked_add(b).map(WidthExpr::Constant).ok_or_else(|| {
                VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypeWidth,
                    "concat width overflow",
                )
            })
        }
        (a, b) => Ok(WidthExpr::Add(Box::new(a), Box::new(b))),
    }
}
fn hardware_signedness(ty: &HardwareType) -> Option<bool> {
    match ty {
        HardwareType::Unsigned(_) | HardwareType::SymbolicUnsigned(_) => Some(false),
        HardwareType::Signed(_) | HardwareType::SymbolicSigned(_) => Some(true),
        HardwareType::Bit => None,
    }
}
fn validate_conversion(
    kind: ConversionKind,
    source: &HardwareType,
    target: &HardwareType,
) -> Result<(), VhdlBackendError> {
    let source_signed = hardware_signedness(source).ok_or_else(|| {
        VhdlBackendError::new(
            VhdlBackendErrorKind::InvalidTypedExpression,
            "conversion source is not a vector",
        )
    })?;
    let target_signed = hardware_signedness(target).ok_or_else(|| {
        VhdlBackendError::new(
            VhdlBackendErrorKind::InvalidTypedExpression,
            "conversion target is not a vector",
        )
    })?;
    let source_width = hardware_width(source).ok_or_else(|| {
        VhdlBackendError::new(
            VhdlBackendErrorKind::InvalidTypedExpression,
            "conversion source width is missing",
        )
    })?;
    let target_width = hardware_width(target).ok_or_else(|| {
        VhdlBackendError::new(
            VhdlBackendErrorKind::InvalidTypedExpression,
            "conversion target width is missing",
        )
    })?;
    let valid = match kind {
        ConversionKind::Resize => {
            source_signed == target_signed
                && !matches!((&source_width,&target_width),(WidthExpr::Constant(s),WidthExpr::Constant(t)) if t<s)
        }
        ConversionKind::Truncate => {
            source_signed == target_signed
                && !matches!((&source_width,&target_width),(WidthExpr::Constant(s),WidthExpr::Constant(t)) if t>s)
        }
        ConversionKind::AsSigned => !source_signed && target_signed && source_width == target_width,
        ConversionKind::AsUnsigned => {
            source_signed && !target_signed && source_width == target_width
        }
    };
    if valid {
        Ok(())
    } else {
        Err(VhdlBackendError::new(
            VhdlBackendErrorKind::InvalidTypedExpression,
            "invalid typed conversion",
        ))
    }
}
fn expr_truncate_helpers(expr: &TypedExpr, flags: &mut (bool, bool)) {
    match &expr.kind {
        TypedExprKind::Convert {
            kind: ConversionKind::Truncate,
            value,
            target_type,
        } => {
            if matches!(
                target_type,
                HardwareType::Unsigned(_) | HardwareType::SymbolicUnsigned(_)
            ) {
                flags.0 = true
            } else if matches!(
                target_type,
                HardwareType::Signed(_) | HardwareType::SymbolicSigned(_)
            ) {
                flags.1 = true
            }
            expr_truncate_helpers(value, flags)
        }
        TypedExprKind::Convert { value, .. } | TypedExprKind::Unary { operand: value, .. } => {
            expr_truncate_helpers(value, flags)
        }
        TypedExprKind::Binary { left, right, .. } => {
            expr_truncate_helpers(left, flags);
            expr_truncate_helpers(right, flags)
        }
        TypedExprKind::If {
            condition,
            when_true,
            when_false,
        } => {
            expr_truncate_helpers(condition, flags);
            expr_truncate_helpers(when_true, flags);
            expr_truncate_helpers(when_false, flags)
        }
        TypedExprKind::Signal(_) | TypedExprKind::Integer(_) => {}
        TypedExprKind::Slice { value, .. } => expr_truncate_helpers(value, flags),
        TypedExprKind::Concat { values } => {
            for value in values {
                expr_truncate_helpers(value, flags);
            }
        }
    }
}
fn module_truncate_helpers(module: &TypedModule) -> (bool, bool) {
    let mut flags = (false, false);
    for assign in &module.assignments {
        expr_truncate_helpers(&assign.value, &mut flags)
    }
    for block in &module.clocked_blocks {
        for update in &block.updates {
            expr_truncate_helpers(&update.value, &mut flags)
        }
        if let Some(reset) = &block.reset {
            for update in &reset.updates {
                expr_truncate_helpers(&update.value, &mut flags)
            }
        }
    }
    flags
}
fn testbench_truncate_helpers(testbench: &TypedTestbench) -> (bool, bool) {
    let mut flags = (false, false);
    for statement in &testbench.statements {
        match statement {
            TypedTestbenchStmt::Drive { value, .. } => expr_truncate_helpers(value, &mut flags),
            TypedTestbenchStmt::Assert { condition, .. } => {
                expr_truncate_helpers(condition, &mut flags)
            }
            _ => {}
        }
    }
    flags
}
fn expr_has_bit_concat(expr: &TypedExpr) -> bool {
    match &expr.kind {
        TypedExprKind::Concat { values } => values
            .iter()
            .any(|v| v.ty == HardwareType::Bit || expr_has_bit_concat(v)),
        TypedExprKind::Slice { value, .. }
        | TypedExprKind::Convert { value, .. }
        | TypedExprKind::Unary { operand: value, .. } => expr_has_bit_concat(value),
        TypedExprKind::Binary { left, right, .. } => {
            expr_has_bit_concat(left) || expr_has_bit_concat(right)
        }
        TypedExprKind::If {
            condition,
            when_true,
            when_false,
        } => {
            expr_has_bit_concat(condition)
                || expr_has_bit_concat(when_true)
                || expr_has_bit_concat(when_false)
        }
        TypedExprKind::Signal(_) | TypedExprKind::Integer(_) => false,
    }
}
fn module_needs_bit_concat(module: &TypedModule) -> bool {
    module
        .assignments
        .iter()
        .any(|a| expr_has_bit_concat(&a.value))
        || module.clocked_blocks.iter().any(|b| {
            b.updates.iter().any(|u| expr_has_bit_concat(&u.value))
                || b.reset
                    .as_ref()
                    .is_some_and(|r| r.updates.iter().any(|u| expr_has_bit_concat(&u.value)))
        })
}
fn testbench_needs_bit_concat(testbench: &TypedTestbench) -> bool {
    testbench.statements.iter().any(|s| match s {
        TypedTestbenchStmt::Drive { value, .. } => expr_has_bit_concat(value),
        TypedTestbenchStmt::Assert { condition, .. } => expr_has_bit_concat(condition),
        _ => false,
    })
}

fn lower_type(ty: &HardwareType) -> Result<VhdlType, VhdlBackendError> {
    match ty {
        HardwareType::Bit => Ok(VhdlType::StdLogic),
        HardwareType::Unsigned(0)
        | HardwareType::Signed(0)
        | HardwareType::SymbolicUnsigned(WidthExpr::Constant(0))
        | HardwareType::SymbolicSigned(WidthExpr::Constant(0)) => Err(VhdlBackendError::new(
            VhdlBackendErrorKind::InvalidTypeWidth,
            "VHDL vector width must be positive",
        )),
        HardwareType::Unsigned(width) => Ok(VhdlType::Unsigned(VhdlExpression::Literal(
            width.to_string(),
        ))),
        HardwareType::Signed(width) => {
            Ok(VhdlType::Signed(VhdlExpression::Literal(width.to_string())))
        }
        HardwareType::SymbolicUnsigned(width) => Ok(VhdlType::Unsigned(lower_width(width)?)),
        HardwareType::SymbolicSigned(width) => Ok(VhdlType::Signed(lower_width(width)?)),
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
        HardwareType::Unsigned(width) => {
            return Ok(VhdlExpression::Call {
                function: VhdlIdentifier("resize".into()),
                arguments: vec![
                    VhdlExpression::Literal(format!("unsigned'(x\"{:016x}\")", value as u64)),
                    VhdlExpression::Literal(width.to_string()),
                ],
            });
        }
        HardwareType::Signed(width) => {
            return Ok(VhdlExpression::Call {
                function: VhdlIdentifier("resize".into()),
                arguments: vec![
                    VhdlExpression::Literal(format!("signed'(x\"{:016x}\")", value as u64)),
                    VhdlExpression::Literal(width.to_string()),
                ],
            });
        }
        HardwareType::SymbolicUnsigned(width) => {
            return Ok(VhdlExpression::Call {
                function: VhdlIdentifier("resize".into()),
                arguments: vec![
                    VhdlExpression::Literal(format!("unsigned'(x\"{:016x}\")", value as u64)),
                    lower_width(width)?,
                ],
            });
        }
        HardwareType::SymbolicSigned(width) => {
            return Ok(VhdlExpression::Call {
                function: VhdlIdentifier("resize".into()),
                arguments: vec![
                    VhdlExpression::Literal(format!("signed'(x\"{:016x}\")", value as u64)),
                    lower_width(width)?,
                ],
            });
        }
    };
    Ok(VhdlExpression::Literal(text))
}
fn generic_name(id: crate::GenericId) -> VhdlIdentifier {
    VhdlIdentifier(format!("gl_g{}", id.0))
}
fn lower_width(width: &WidthExpr) -> Result<VhdlExpression, VhdlBackendError> {
    match width {
        WidthExpr::Constant(v) => Ok(VhdlExpression::Literal(v.to_string())),
        WidthExpr::Generic(id) => Ok(VhdlExpression::Name(generic_name(*id))),
        WidthExpr::Add(a, b) => Ok(VhdlExpression::Binary {
            op: "+".into(),
            left: Box::new(lower_width(a)?),
            right: Box::new(lower_width(b)?),
        }),
        WidthExpr::Multiply(a, b) => Ok(VhdlExpression::Binary {
            op: "*".into(),
            left: Box::new(lower_width(a)?),
            right: Box::new(lower_width(b)?),
        }),
    }
}
fn instantiate_type(
    ty: &HardwareType,
    bindings: &[crate::TypedGenericBinding],
) -> Result<HardwareType, VhdlBackendError> {
    fn sub(w: &WidthExpr, b: &[crate::TypedGenericBinding]) -> Result<WidthExpr, VhdlBackendError> {
        match w {
            WidthExpr::Constant(v) => Ok(WidthExpr::Constant(*v)),
            WidthExpr::Generic(id) => b
                .iter()
                .find(|x| x.formal == *id)
                .map(|x| x.value.clone())
                .ok_or_else(|| {
                    VhdlBackendError::new(
                        VhdlBackendErrorKind::InconsistentId,
                        "missing generic binding",
                    )
                }),
            WidthExpr::Add(a, c) => Ok(WidthExpr::Add(Box::new(sub(a, b)?), Box::new(sub(c, b)?))),
            WidthExpr::Multiply(a, c) => Ok(WidthExpr::Multiply(
                Box::new(sub(a, b)?),
                Box::new(sub(c, b)?),
            )),
        }
    }
    match ty {
        HardwareType::Bit => Ok(HardwareType::Bit),
        HardwareType::Unsigned(v) => Ok(HardwareType::Unsigned(*v)),
        HardwareType::Signed(v) => Ok(HardwareType::Signed(*v)),
        HardwareType::SymbolicUnsigned(w) => Ok(HardwareType::SymbolicUnsigned(sub(w, bindings)?)),
        HardwareType::SymbolicSigned(w) => Ok(HardwareType::SymbolicSigned(sub(w, bindings)?)),
    }
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
