use std::{
    collections::{HashMap, HashSet},
    fmt,
};

use crate::{
    BinaryOp, ClockEdge, ClockedBlockId, ClockedDecl, ConstExprAst, ConversionKind, Expr,
    GenericBinding, GenericId, GenericKind, GenericKindSyntax, HardwareType, InstanceId,
    ModuleDecl, ModuleId, ModuleItem, NextStmt, PortDirection, Program, SignalId, SignalKind,
    SimulationTime, Span, Spanned, TestbenchDecl, TestbenchId, TestbenchStmt, TypeExpr,
    TypedAssign, TypedClockedBlock, TypedExpr, TypedExprKind, TypedGeneric, TypedGenericBinding,
    TypedInstance, TypedModule, TypedNext, TypedPortConnection, TypedProgram, TypedReset,
    TypedSignal, TypedTestbench, TypedTestbenchClock, TypedTestbenchStmt, UnaryOp, WidthExpr,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticErrorKind {
    DuplicateModule,
    DuplicateSignal,
    UndeclaredSignal,
    UndeclaredAssignTarget,
    AssignToInput,
    AssignToRegister,
    MultipleDrivers,
    UnknownOperator,
    WrongArgumentCount,
    BitArithmetic,
    BitOrdering,
    TypeMismatch,
    AssignTypeMismatch,
    IfConditionType,
    IfBranchType,
    IntegerOutOfRange,
    CannotInferIntegerType,
    CannotInferType,
    UndeclaredClock,
    InvalidClockSource,
    InvalidClockType,
    UndeclaredReset,
    InvalidResetSource,
    InvalidResetType,
    UndeclaredNextTarget,
    NextTargetNotRegister,
    DuplicateNext,
    RegisterMultipleClockedDrivers,
    NextTypeMismatch,
    ResetTargetMissing,
    ResetTargetExtra,
    DuplicateTestbench,
    UnknownTestbenchTarget,
    InvalidTestbenchClock,
    DuplicateTestbenchClock,
    InvalidDriveTarget,
    DriveClock,
    InvalidTestbenchReference,
    InvalidAssertType,
    UnknownWaitClock,
    DuplicateInstance,
    UnknownInstanceModule,
    UnknownFormalPort,
    DuplicateFormalPort,
    MissingFormalPort,
    UnknownActualSignal,
    InstanceInputTypeMismatch,
    InstanceOutputTypeMismatch,
    InvalidInstanceOutputTarget,
    InstanceMultipleDriver,
    RecursiveModule,
    DuplicateGeneric,
    InvalidGenericDefault,
    UnknownGeneric,
    InvalidWidthExpression,
    ConstExpressionOverflow,
    UnknownInstanceGeneric,
    DuplicateGenericBinding,
    InvalidGenericActual,
    InvalidConversionTarget,
    ConversionSourceNotVector,
    ConversionSignednessMismatch,
    InvalidResizeDirection,
    InvalidTruncateDirection,
    WidthRelationUnknown,
    InvalidReinterpretation,
    SliceSourceNotVector,
    SliceOutOfBounds,
    SliceRangeUnknown,
    InvalidConcatOperand,
    ConcatWidthOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticError {
    pub kind: SemanticErrorKind,
    pub message: String,
    pub span: Span,
    pub related_span: Option<Box<Span>>,
}

impl SemanticError {
    fn new(kind: SemanticErrorKind, message: impl Into<String>, span: Span) -> Self {
        Self {
            kind,
            message: message.into(),
            span,
            related_span: None,
        }
    }
    fn related(mut self, span: Span) -> Self {
        self.related_span = Some(Box::new(span));
        self
    }
}
impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for SemanticError {}

#[derive(Clone)]
struct SignalInfo {
    id: SignalId,
    name: String,
    ty: HardwareType,
    class: SignalClass,
    declaration_span: Span,
    initial: Option<Spanned<Expr>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SignalClass {
    Input,
    Output,
    Wire,
    Register,
}

struct ModuleContext {
    signals: Vec<SignalInfo>,
    names: HashMap<String, usize>,
    generics: Vec<TypedGeneric>,
}

pub fn analyze_program(program: &Program) -> Result<TypedProgram, SemanticError> {
    let mut module_names = HashMap::<&str, Span>::new();
    let mut next_signal = 0_u32;
    let mut next_clocked = 0_u32;
    let mut next_generic = 0_u32;
    let mut modules = Vec::with_capacity(program.modules.len());
    for (index, module) in program.modules.iter().enumerate() {
        if let Some(previous) = module_names.insert(&module.value.name.name, module.value.name.span)
        {
            return Err(SemanticError::new(
                SemanticErrorKind::DuplicateModule,
                format!("duplicate module `{}`", module.value.name.name),
                module.value.name.span,
            )
            .related(previous));
        }
        let id = ModuleId(u32::try_from(index).map_err(|_| {
            SemanticError::new(
                SemanticErrorKind::CannotInferType,
                "too many modules",
                module.span,
            )
        })?);
        modules.push(analyze_module(
            module,
            id,
            &mut next_signal,
            &mut next_clocked,
            &mut next_generic,
        )?);
    }
    let mut next_instance = 0_u32;
    resolve_instances(program, &mut modules, &mut next_instance)?;
    let module_order = topological_order(&modules)?;
    let module_indexes = modules
        .iter()
        .enumerate()
        .map(|(index, module)| (module.name.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut testbench_names = HashMap::new();
    let mut testbenches = Vec::new();
    for (index, testbench) in program.testbenches.iter().enumerate() {
        if let Some(previous) = testbench_names.insert(
            testbench.value.name.name.as_str(),
            testbench.value.name.span,
        ) {
            return Err(SemanticError::new(
                SemanticErrorKind::DuplicateTestbench,
                format!("duplicate testbench `{}`", testbench.value.name.name),
                testbench.value.name.span,
            )
            .related(previous));
        }
        let module_index = *module_indexes
            .get(testbench.value.target.name.as_str())
            .ok_or_else(|| {
                SemanticError::new(
                    SemanticErrorKind::UnknownTestbenchTarget,
                    format!("unknown target module `{}`", testbench.value.target.name),
                    testbench.value.target.span,
                )
            })?;
        let module = modules.get(module_index).ok_or_else(|| {
            SemanticError::new(
                SemanticErrorKind::UnknownTestbenchTarget,
                "target module index is invalid",
                testbench.value.target.span,
            )
        })?;
        let id = TestbenchId(u32::try_from(index).map_err(|_| {
            SemanticError::new(
                SemanticErrorKind::CannotInferType,
                "too many testbenches",
                testbench.span,
            )
        })?);
        testbenches.push(analyze_testbench(testbench, id, module)?);
    }
    Ok(TypedProgram {
        modules,
        testbenches,
        module_order,
    })
}

fn resolve_instances(
    program: &Program,
    modules: &mut [TypedModule],
    next_id: &mut u32,
) -> Result<(), SemanticError> {
    let module_names = modules
        .iter()
        .enumerate()
        .map(|(index, module)| (module.name.clone(), index))
        .collect::<HashMap<_, _>>();
    let snapshot = modules.to_vec();
    for (parent_index, source_module) in program.modules.iter().enumerate() {
        let parent_snapshot = snapshot.get(parent_index).ok_or_else(|| {
            SemanticError::new(
                SemanticErrorKind::UnknownInstanceModule,
                "parent module is missing",
                source_module.span,
            )
        })?;
        let mut instance_names = HashMap::new();
        let mut drivers = parent_snapshot
            .assignments
            .iter()
            .map(|assignment| (assignment.target, assignment.span))
            .collect::<HashMap<_, _>>();
        let mut typed_instances = Vec::new();
        for item in &source_module.value.items {
            let ModuleItem::Instance(instance) = &item.value else {
                continue;
            };
            if let Some(previous) =
                instance_names.insert(instance.name.name.clone(), instance.name.span)
            {
                return Err(SemanticError::new(
                    SemanticErrorKind::DuplicateInstance,
                    format!("duplicate instance `{}`", instance.name.name),
                    instance.name.span,
                )
                .related(previous));
            }
            let target_index = *module_names.get(&instance.module.name).ok_or_else(|| {
                SemanticError::new(
                    SemanticErrorKind::UnknownInstanceModule,
                    format!("unknown instance module `{}`", instance.module.name),
                    instance.module.span,
                )
            })?;
            let target = snapshot.get(target_index).ok_or_else(|| {
                SemanticError::new(
                    SemanticErrorKind::UnknownInstanceModule,
                    "instance target index is invalid",
                    instance.module.span,
                )
            })?;
            let generic_bindings = resolve_generic_bindings(
                &instance.generics,
                &parent_snapshot.generics,
                &target.generics,
                instance.generics_span.unwrap_or(item.span),
            )?;
            let ports = target
                .signals
                .iter()
                .filter(|signal| matches!(signal.kind, SignalKind::Input | SignalKind::Output))
                .collect::<Vec<_>>();
            let mut supplied = HashMap::new();
            for connection in &instance.ports {
                if supplied
                    .insert(connection.value.formal.name.clone(), connection)
                    .is_some()
                {
                    return Err(SemanticError::new(
                        SemanticErrorKind::DuplicateFormalPort,
                        format!(
                            "formal port `{}` is connected more than once",
                            connection.value.formal.name
                        ),
                        connection.value.formal.span,
                    ));
                }
                if !ports
                    .iter()
                    .any(|port| port.name == connection.value.formal.name)
                {
                    return Err(SemanticError::new(
                        SemanticErrorKind::UnknownFormalPort,
                        format!("unknown formal port `{}`", connection.value.formal.name),
                        connection.value.formal.span,
                    ));
                }
            }
            let mut connections = Vec::new();
            for formal in ports {
                let connection = supplied.get(&formal.name).ok_or_else(|| {
                    SemanticError::new(
                        SemanticErrorKind::MissingFormalPort,
                        format!("missing connection for port `{}`", formal.name),
                        instance.ports_span,
                    )
                })?;
                let actual = parent_snapshot
                    .signals
                    .iter()
                    .find(|signal| signal.name == connection.value.actual.name)
                    .ok_or_else(|| {
                        SemanticError::new(
                            SemanticErrorKind::UnknownActualSignal,
                            format!("unknown actual signal `{}`", connection.value.actual.name),
                            connection.value.actual.span,
                        )
                    })?;
                let direction = match formal.kind {
                    SignalKind::Input => PortDirection::Input,
                    SignalKind::Output => PortDirection::Output,
                    _ => {
                        return Err(SemanticError::new(
                            SemanticErrorKind::UnknownFormalPort,
                            "formal signal is not a port",
                            connection.value.formal.span,
                        ));
                    }
                };
                let formal_ty = substitute_type(&formal.ty, &generic_bindings)?;
                if formal_ty != actual.ty {
                    let kind = if direction == PortDirection::Input {
                        SemanticErrorKind::InstanceInputTypeMismatch
                    } else {
                        SemanticErrorKind::InstanceOutputTypeMismatch
                    };
                    return Err(SemanticError::new(
                        kind,
                        "instance port and actual signal types differ",
                        connection.span,
                    ));
                }
                if direction == PortDirection::Output {
                    if !matches!(actual.kind, SignalKind::Output | SignalKind::Wire) {
                        return Err(SemanticError::new(
                            SemanticErrorKind::InvalidInstanceOutputTarget,
                            "child output must connect to parent output or wire",
                            connection.value.actual.span,
                        ));
                    }
                    if let Some(previous) = drivers.insert(actual.id, connection.span) {
                        return Err(SemanticError::new(
                            SemanticErrorKind::InstanceMultipleDriver,
                            format!("multiple drivers for `{}`", actual.name),
                            connection.value.actual.span,
                        )
                        .related(previous));
                    }
                }
                connections.push(TypedPortConnection {
                    formal: formal.id,
                    actual: actual.id,
                    direction,
                    ty: formal_ty,
                    span: connection.span,
                });
            }
            let id = InstanceId(*next_id);
            *next_id = next_id.checked_add(1).ok_or_else(|| {
                SemanticError::new(
                    SemanticErrorKind::CannotInferType,
                    "too many instances",
                    item.span,
                )
            })?;
            typed_instances.push(TypedInstance {
                id,
                name: instance.name.name.clone(),
                target_module: target.id,
                generic_bindings,
                connections,
                span: item.span,
            });
        }
        let parent = modules.get_mut(parent_index).ok_or_else(|| {
            SemanticError::new(
                SemanticErrorKind::UnknownInstanceModule,
                "parent module is missing",
                source_module.span,
            )
        })?;
        parent.instances = typed_instances;
    }
    Ok(())
}

fn topological_order(modules: &[TypedModule]) -> Result<Vec<ModuleId>, SemanticError> {
    let indexes = modules
        .iter()
        .enumerate()
        .map(|(index, module)| (module.id, index))
        .collect::<HashMap<_, _>>();
    let mut state = vec![0_u8; modules.len()];
    let mut order = Vec::new();
    fn visit(
        index: usize,
        modules: &[TypedModule],
        indexes: &HashMap<ModuleId, usize>,
        state: &mut [u8],
        order: &mut Vec<ModuleId>,
    ) -> Result<(), SemanticError> {
        if state.get(index) == Some(&2) {
            return Ok(());
        }
        if state.get(index) == Some(&1) {
            let module = modules.get(index).ok_or_else(|| {
                SemanticError::new(
                    SemanticErrorKind::RecursiveModule,
                    "invalid dependency index",
                    empty_span(),
                )
            })?;
            return Err(SemanticError::new(
                SemanticErrorKind::RecursiveModule,
                format!("recursive module instantiation involving `{}`", module.name),
                module.name_span,
            ));
        }
        if let Some(value) = state.get_mut(index) {
            *value = 1;
        } else {
            return Err(SemanticError::new(
                SemanticErrorKind::RecursiveModule,
                "invalid dependency state",
                empty_span(),
            ));
        }
        let module = modules.get(index).ok_or_else(|| {
            SemanticError::new(
                SemanticErrorKind::RecursiveModule,
                "invalid dependency index",
                empty_span(),
            )
        })?;
        for instance in &module.instances {
            let child = *indexes.get(&instance.target_module).ok_or_else(|| {
                SemanticError::new(
                    SemanticErrorKind::UnknownInstanceModule,
                    "instance target ModuleId is missing",
                    instance.span,
                )
            })?;
            if state.get(child) == Some(&1) {
                return Err(SemanticError::new(
                    SemanticErrorKind::RecursiveModule,
                    format!(
                        "recursive module instantiation: {} -> {}",
                        module.name,
                        modules.get(child).map_or("?", |value| value.name.as_str())
                    ),
                    instance.span,
                )
                .related(
                    modules
                        .get(child)
                        .map_or(instance.span, |value| value.name_span),
                ));
            }
            visit(child, modules, indexes, state, order)?;
        }
        if let Some(value) = state.get_mut(index) {
            *value = 2;
        }
        order.push(module.id);
        Ok(())
    }
    for index in 0..modules.len() {
        visit(index, modules, &indexes, &mut state, &mut order)?;
    }
    Ok(order)
}

fn empty_span() -> Span {
    let position = crate::Position {
        offset: 0,
        line: 1,
        column: 1,
    };
    Span {
        start: position,
        end: position,
    }
}

fn analyze_module(
    module: &Spanned<ModuleDecl>,
    id: ModuleId,
    next_id: &mut u32,
    next_clocked_id: &mut u32,
    next_generic_id: &mut u32,
) -> Result<TypedModule, SemanticError> {
    let generics = collect_generics(&module.value, next_generic_id)?;
    let context = collect_signals(&module.value, next_id, &generics)?;
    let mut signals = Vec::with_capacity(context.signals.len());
    for info in &context.signals {
        let kind = match info.class {
            SignalClass::Input => SignalKind::Input,
            SignalClass::Output => SignalKind::Output,
            SignalClass::Wire => SignalKind::Wire,
            SignalClass::Register => SignalKind::Register {
                initial: info
                    .initial
                    .as_ref()
                    .map(|expr| check_expr(expr, Some(&info.ty), &context))
                    .transpose()?,
            },
        };
        signals.push(TypedSignal {
            id: info.id,
            name: info.name.clone(),
            kind,
            ty: info.ty.clone(),
            declaration_span: info.declaration_span,
        });
    }
    let mut driven = HashSet::new();
    let mut assignments = Vec::new();
    for item in &module.value.items {
        if let ModuleItem::Assign(assign) = &item.value {
            let info = lookup(&context, &assign.target.name).ok_or_else(|| {
                SemanticError::new(
                    SemanticErrorKind::UndeclaredAssignTarget,
                    format!("undeclared assign target `{}`", assign.target.name),
                    assign.target.span,
                )
            })?;
            match info.class {
                SignalClass::Input => {
                    return Err(SemanticError::new(
                        SemanticErrorKind::AssignToInput,
                        "cannot continuously assign to an input",
                        assign.target.span,
                    ));
                }
                SignalClass::Register => {
                    return Err(SemanticError::new(
                        SemanticErrorKind::AssignToRegister,
                        "cannot continuously assign to a register",
                        assign.target.span,
                    ));
                }
                SignalClass::Output | SignalClass::Wire => {}
            }
            if !driven.insert(info.id) {
                return Err(SemanticError::new(
                    SemanticErrorKind::MultipleDrivers,
                    format!("multiple assignments drive `{}`", info.name),
                    assign.target.span,
                )
                .related(info.declaration_span));
            }
            let value =
                check_expr(&assign.value, Some(&info.ty), &context).map_err(|mut error| {
                    if error.kind == SemanticErrorKind::TypeMismatch
                        && error.span == assign.value.span
                    {
                        error.kind = SemanticErrorKind::AssignTypeMismatch;
                        error.message = "assignment value type does not match target".into();
                    }
                    error
                })?;
            assignments.push(TypedAssign {
                target: info.id,
                value,
                span: item.span,
            });
        }
    }
    let mut register_drivers = HashMap::new();
    let mut clocked_blocks = Vec::new();
    for item in &module.value.items {
        if let ModuleItem::Clocked(clocked) = &item.value {
            let block_id = ClockedBlockId(*next_clocked_id);
            *next_clocked_id = next_clocked_id.checked_add(1).ok_or_else(|| {
                SemanticError::new(
                    SemanticErrorKind::CannotInferType,
                    "too many clocked blocks",
                    item.span,
                )
            })?;
            clocked_blocks.push(analyze_clocked(
                clocked,
                item.span,
                block_id,
                &context,
                &mut register_drivers,
            )?);
        }
    }
    Ok(TypedModule {
        id,
        name: module.value.name.name.clone(),
        name_span: module.value.name.span,
        generics,
        signals,
        assignments,
        clocked_blocks,
        instances: Vec::new(),
        span: module.span,
    })
}

fn analyze_testbench(
    testbench: &Spanned<TestbenchDecl>,
    id: TestbenchId,
    module: &TypedModule,
) -> Result<TypedTestbench, SemanticError> {
    let target_generic_bindings = resolve_generic_bindings(
        &testbench.value.target_generics,
        &[],
        &module.generics,
        testbench.value.target.span,
    )?;
    let mut context = ModuleContext {
        signals: Vec::new(),
        names: HashMap::new(),
        generics: Vec::new(),
    };
    for signal in &module.signals {
        let class = match signal.kind {
            SignalKind::Input => SignalClass::Input,
            SignalKind::Output => SignalClass::Output,
            _ => continue,
        };
        context
            .names
            .insert(signal.name.clone(), context.signals.len());
        context.signals.push(SignalInfo {
            id: signal.id,
            name: signal.name.clone(),
            ty: substitute_type(&signal.ty, &target_generic_bindings)?,
            class,
            declaration_span: signal.declaration_span,
            initial: None,
        });
    }
    let mut clock_ids = HashSet::new();
    let mut clocks = Vec::new();
    for clock in &testbench.value.clocks {
        let info = lookup(&context, &clock.value.signal.name).ok_or_else(|| {
            SemanticError::new(
                SemanticErrorKind::InvalidTestbenchClock,
                format!("unknown testbench clock `{}`", clock.value.signal.name),
                clock.value.signal.span,
            )
        })?;
        if info.class != SignalClass::Input || info.ty != HardwareType::Bit {
            return Err(SemanticError::new(
                SemanticErrorKind::InvalidTestbenchClock,
                "testbench clock must be an input bit",
                clock.value.signal.span,
            ));
        }
        if !clock_ids.insert(info.id) {
            return Err(SemanticError::new(
                SemanticErrorKind::DuplicateTestbenchClock,
                "duplicate testbench clock",
                clock.value.signal.span,
            ));
        }
        clocks.push(TypedTestbenchClock {
            signal: info.id,
            period: SimulationTime {
                value: clock.value.period.value,
                unit: clock.value.period.unit,
            },
            span: clock.span,
        });
    }
    let mut statements = Vec::new();
    for statement in &testbench.value.stimulus {
        let typed = match &statement.value {
            TestbenchStmt::Drive { target, value } => {
                let info = lookup(&context, &target.name).ok_or_else(|| {
                    SemanticError::new(
                        SemanticErrorKind::InvalidDriveTarget,
                        format!("unknown drive target `{}`", target.name),
                        target.span,
                    )
                })?;
                if info.class != SignalClass::Input {
                    return Err(SemanticError::new(
                        SemanticErrorKind::InvalidDriveTarget,
                        "drive target must be an input port",
                        target.span,
                    ));
                }
                if clock_ids.contains(&info.id) {
                    return Err(SemanticError::new(
                        SemanticErrorKind::DriveClock,
                        "clock input cannot be driven by stimulus",
                        target.span,
                    ));
                }
                TypedTestbenchStmt::Drive {
                    target: info.id,
                    value: check_expr(value, Some(&info.ty), &context)?,
                    span: statement.span,
                }
            }
            TestbenchStmt::Wait { duration } => TypedTestbenchStmt::Wait {
                duration: SimulationTime {
                    value: duration.value,
                    unit: duration.unit,
                },
                span: statement.span,
            },
            TestbenchStmt::WaitRising { clock, count } => {
                let info = lookup(&context, &clock.name).ok_or_else(|| {
                    SemanticError::new(
                        SemanticErrorKind::UnknownWaitClock,
                        format!("unknown wait clock `{}`", clock.name),
                        clock.span,
                    )
                })?;
                if !clock_ids.contains(&info.id) {
                    return Err(SemanticError::new(
                        SemanticErrorKind::UnknownWaitClock,
                        "wait-rising requires a declared testbench clock",
                        clock.span,
                    ));
                }
                TypedTestbenchStmt::WaitRising {
                    clock: info.id,
                    count: *count,
                    span: statement.span,
                }
            }
            TestbenchStmt::Assert { condition, message } => {
                let condition = check_expr(condition, Some(&HardwareType::Bit), &context).map_err(
                    |mut error| {
                        if error.kind == SemanticErrorKind::TypeMismatch {
                            error.kind = SemanticErrorKind::InvalidAssertType;
                            error.message = "assert condition must have type bit".into();
                        }
                        error
                    },
                )?;
                TypedTestbenchStmt::Assert {
                    condition,
                    message: message.clone(),
                    span: statement.span,
                }
            }
        };
        statements.push(typed);
    }
    Ok(TypedTestbench {
        id,
        name: testbench.value.name.name.clone(),
        target: module.id,
        target_generic_bindings,
        clocks,
        statements,
        span: testbench.span,
    })
}

fn analyze_clocked(
    clocked: &ClockedDecl,
    span: Span,
    id: ClockedBlockId,
    context: &ModuleContext,
    register_drivers: &mut HashMap<SignalId, Span>,
) -> Result<TypedClockedBlock, SemanticError> {
    let clock = resolve_control_signal(
        context,
        &clocked.clock.name,
        clocked.clock.span,
        SemanticErrorKind::UndeclaredClock,
        SemanticErrorKind::InvalidClockSource,
        SemanticErrorKind::InvalidClockType,
        "clock",
    )?;
    let mut normal_seen = HashMap::new();
    let mut updates = Vec::new();
    for update in &clocked.updates {
        let typed = analyze_next(update, context, &mut normal_seen)?;
        if let Some(previous) = register_drivers.insert(typed.target, update.span) {
            return Err(SemanticError::new(
                SemanticErrorKind::RegisterMultipleClockedDrivers,
                "register is driven by multiple clocked blocks",
                update.value.target.span,
            )
            .related(previous));
        }
        updates.push(typed);
    }
    let reset = clocked
        .reset
        .as_ref()
        .map(|reset| {
            let signal = resolve_control_signal(
                context,
                &reset.value.signal.name,
                reset.value.signal.span,
                SemanticErrorKind::UndeclaredReset,
                SemanticErrorKind::InvalidResetSource,
                SemanticErrorKind::InvalidResetType,
                "reset",
            )?;
            let mut reset_seen = HashMap::new();
            let reset_updates = reset
                .value
                .updates
                .iter()
                .map(|update| analyze_next(update, context, &mut reset_seen))
                .collect::<Result<Vec<_>, _>>()?;
            for target in normal_seen.keys() {
                if !reset_seen.contains_key(target) {
                    return Err(SemanticError::new(
                        SemanticErrorKind::ResetTargetMissing,
                        "reset is missing a normally updated register",
                        reset.span,
                    ));
                }
            }
            for (target, target_span) in &reset_seen {
                if !normal_seen.contains_key(target) {
                    return Err(SemanticError::new(
                        SemanticErrorKind::ResetTargetExtra,
                        "reset updates a register absent from normal updates",
                        *target_span,
                    ));
                }
            }
            Ok(TypedReset {
                kind: reset.value.kind,
                signal,
                updates: reset_updates,
                span: reset.span,
            })
        })
        .transpose()?;
    Ok(TypedClockedBlock {
        id,
        clock,
        edge: ClockEdge::Rising,
        reset,
        updates,
        span,
    })
}

fn resolve_control_signal(
    context: &ModuleContext,
    name: &str,
    span: Span,
    undeclared: SemanticErrorKind,
    invalid_source: SemanticErrorKind,
    invalid_type: SemanticErrorKind,
    role: &str,
) -> Result<SignalId, SemanticError> {
    let info = lookup(context, name).ok_or_else(|| {
        SemanticError::new(
            undeclared,
            format!("undeclared {role} signal `{name}`"),
            span,
        )
    })?;
    if info.class != SignalClass::Input {
        return Err(SemanticError::new(
            invalid_source,
            format!("{role} signal must be an input port"),
            span,
        ));
    }
    if info.ty != HardwareType::Bit {
        return Err(SemanticError::new(
            invalid_type,
            format!("{role} signal must have type bit"),
            span,
        ));
    }
    Ok(info.id)
}

fn analyze_next(
    update: &Spanned<NextStmt>,
    context: &ModuleContext,
    seen: &mut HashMap<SignalId, Span>,
) -> Result<TypedNext, SemanticError> {
    let info = lookup(context, &update.value.target.name).ok_or_else(|| {
        SemanticError::new(
            SemanticErrorKind::UndeclaredNextTarget,
            format!("undeclared next target `{}`", update.value.target.name),
            update.value.target.span,
        )
    })?;
    if info.class != SignalClass::Register {
        return Err(SemanticError::new(
            SemanticErrorKind::NextTargetNotRegister,
            "next target must be a register",
            update.value.target.span,
        ));
    }
    if let Some(previous) = seen.insert(info.id, update.span) {
        return Err(SemanticError::new(
            SemanticErrorKind::DuplicateNext,
            "register is updated more than once in this block",
            update.value.target.span,
        )
        .related(previous));
    }
    let value = check_expr(&update.value.value, Some(&info.ty), context).map_err(|mut error| {
        if error.kind == SemanticErrorKind::TypeMismatch && error.span == update.value.value.span {
            error.kind = SemanticErrorKind::NextTypeMismatch;
            error.message = "next value type does not match register".into();
        }
        error
    })?;
    Ok(TypedNext {
        target: info.id,
        value,
        span: update.span,
    })
}

fn collect_signals(
    module: &ModuleDecl,
    next_id: &mut u32,
    generics: &[TypedGeneric],
) -> Result<ModuleContext, SemanticError> {
    let mut context = ModuleContext {
        signals: Vec::new(),
        names: HashMap::new(),
        generics: generics.to_vec(),
    };
    for port in &module.ports {
        let class = match port.value.direction {
            PortDirection::Input => SignalClass::Input,
            PortDirection::Output => SignalClass::Output,
        };
        add_signal(
            &mut context,
            next_id,
            &port.value.name.name,
            class,
            type_from_ast(&port.value.ty.value, generics)?,
            port.span,
            None,
        )?;
    }
    for item in &module.items {
        match &item.value {
            ModuleItem::Wire(wire) => add_signal(
                &mut context,
                next_id,
                &wire.name.name,
                SignalClass::Wire,
                type_from_ast(&wire.ty.value, generics)?,
                item.span,
                None,
            )?,
            ModuleItem::Register(reg) => add_signal(
                &mut context,
                next_id,
                &reg.name.name,
                SignalClass::Register,
                type_from_ast(&reg.ty.value, generics)?,
                item.span,
                reg.initial.clone(),
            )?,
            ModuleItem::Assign(_) | ModuleItem::Clocked(_) | ModuleItem::Instance(_) => {}
        }
    }
    Ok(context)
}

fn add_signal(
    context: &mut ModuleContext,
    next_id: &mut u32,
    name: &str,
    class: SignalClass,
    ty: HardwareType,
    span: Span,
    initial: Option<Spanned<Expr>>,
) -> Result<(), SemanticError> {
    if let Some(previous) = context
        .names
        .get(name)
        .and_then(|index| context.signals.get(*index))
    {
        return Err(SemanticError::new(
            SemanticErrorKind::DuplicateSignal,
            format!("duplicate signal `{name}`"),
            span,
        )
        .related(previous.declaration_span));
    }
    let id = SignalId(*next_id);
    *next_id = next_id.checked_add(1).ok_or_else(|| {
        SemanticError::new(SemanticErrorKind::CannotInferType, "too many signals", span)
    })?;
    context.names.insert(name.to_owned(), context.signals.len());
    context.signals.push(SignalInfo {
        id,
        name: name.to_owned(),
        ty,
        class,
        declaration_span: span,
        initial,
    });
    Ok(())
}

fn check_expr(
    expr: &Spanned<Expr>,
    expected: Option<&HardwareType>,
    context: &ModuleContext,
) -> Result<TypedExpr, SemanticError> {
    let typed = match &expr.value {
        Expr::Reference(identifier) => {
            let info = lookup(context, &identifier.name).ok_or_else(|| {
                SemanticError::new(
                    SemanticErrorKind::UndeclaredSignal,
                    format!("undeclared signal `{}`", identifier.name),
                    identifier.span,
                )
            })?;
            TypedExpr {
                kind: TypedExprKind::Signal(info.id),
                ty: info.ty.clone(),
                span: expr.span,
            }
        }
        Expr::Integer(value) => {
            let ty = expected.ok_or_else(|| {
                SemanticError::new(
                    SemanticErrorKind::CannotInferIntegerType,
                    "cannot infer integer literal type",
                    expr.span,
                )
            })?;
            check_integer(*value, ty, &context.generics, expr.span)?;
            TypedExpr {
                kind: TypedExprKind::Integer(*value),
                ty: ty.clone(),
                span: expr.span,
            }
        }
        Expr::Call { callee, arguments } => check_call(
            callee.name.as_str(),
            arguments,
            expected,
            context,
            expr.span,
        )?,
        Expr::Resize { target, value } => {
            check_sized_conversion(ConversionKind::Resize, target, value, context, expr.span)?
        }
        Expr::Truncate { target, value } => {
            check_sized_conversion(ConversionKind::Truncate, target, value, context, expr.span)?
        }
        Expr::Slice {
            value,
            offset,
            width,
        } => check_slice(value, offset, width, context, expr.span)?,
        Expr::Concat { values } => check_concat(values, context, expr.span)?,
    };
    if let Some(expected) = expected
        && &typed.ty != expected
    {
        return Err(SemanticError::new(
            SemanticErrorKind::TypeMismatch,
            format!("expected type {expected:?}, found {:?}", typed.ty),
            expr.span,
        ));
    }
    Ok(typed)
}

fn check_call(
    name: &str,
    args: &[Spanned<Expr>],
    expected: Option<&HardwareType>,
    context: &ModuleContext,
    span: Span,
) -> Result<TypedExpr, SemanticError> {
    match name {
        "as-signed" | "as-unsigned" => {
            require_arity(name, args, 1, span)?;
            let value = check_expr(&args[0], None, context)?;
            let ty = reinterpret_type(&value.ty, name == "as-signed").ok_or_else(|| {
                SemanticError::new(
                    SemanticErrorKind::InvalidReinterpretation,
                    if name == "as-signed" {
                        "as-signed requires an unsigned vector"
                    } else {
                        "as-unsigned requires a signed vector"
                    },
                    span,
                )
            })?;
            Ok(TypedExpr {
                kind: TypedExprKind::Convert {
                    kind: if name == "as-signed" {
                        ConversionKind::AsSigned
                    } else {
                        ConversionKind::AsUnsigned
                    },
                    value: Box::new(value),
                    target_type: ty.clone(),
                },
                ty,
                span,
            })
        }
        "not" => {
            require_arity(name, args, 1, span)?;
            let operand = check_expr(&args[0], expected, context)?;
            Ok(TypedExpr {
                ty: operand.ty.clone(),
                kind: TypedExprKind::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                },
                span,
            })
        }
        "and" | "or" | "xor" | "+" | "-" => {
            require_arity(name, args, 2, span)?;
            let op = match name {
                "and" => BinaryOp::And,
                "or" => BinaryOp::Or,
                "xor" => BinaryOp::Xor,
                "+" => BinaryOp::Add,
                _ => BinaryOp::Subtract,
            };
            let (left, right, ty) = check_pair(args, expected, context)?;
            if matches!(op, BinaryOp::Add | BinaryOp::Subtract) && ty == HardwareType::Bit {
                return Err(SemanticError::new(
                    SemanticErrorKind::BitArithmetic,
                    "arithmetic operators do not accept bit",
                    span,
                ));
            }
            Ok(binary(op, left, right, ty, span))
        }
        "=" | "/=" | "<" | "<=" | ">" | ">=" => {
            require_arity(name, args, 2, span)?;
            let op = match name {
                "=" => BinaryOp::Equal,
                "/=" => BinaryOp::NotEqual,
                "<" => BinaryOp::LessThan,
                "<=" => BinaryOp::LessEqual,
                ">" => BinaryOp::GreaterThan,
                _ => BinaryOp::GreaterEqual,
            };
            let (left, right, operand_ty) = check_pair(args, None, context)?;
            if matches!(
                op,
                BinaryOp::LessThan
                    | BinaryOp::LessEqual
                    | BinaryOp::GreaterThan
                    | BinaryOp::GreaterEqual
            ) && operand_ty == HardwareType::Bit
            {
                return Err(SemanticError::new(
                    SemanticErrorKind::BitOrdering,
                    "ordering operators do not accept bit",
                    span,
                ));
            }
            Ok(binary(op, left, right, HardwareType::Bit, span))
        }
        "if" => {
            require_arity(name, args, 3, span)?;
            let condition =
                check_expr(&args[0], Some(&HardwareType::Bit), context).map_err(|mut error| {
                    if error.kind == SemanticErrorKind::TypeMismatch {
                        error.kind = SemanticErrorKind::IfConditionType;
                        error.message = "if condition must have type bit".into();
                    }
                    error
                })?;
            let hint = type_hint(&args[1], context)
                .or_else(|| type_hint(&args[2], context))
                .or_else(|| expected.cloned());
            let when_true = check_expr(&args[1], hint.as_ref(), context)?;
            let when_false =
                check_expr(&args[2], Some(&when_true.ty), context).map_err(|mut error| {
                    if error.kind == SemanticErrorKind::TypeMismatch {
                        error.kind = SemanticErrorKind::IfBranchType;
                        error.message = "if branches must have identical types".into();
                    }
                    error
                })?;
            Ok(TypedExpr {
                ty: when_true.ty.clone(),
                kind: TypedExprKind::If {
                    condition: Box::new(condition),
                    when_true: Box::new(when_true),
                    when_false: Box::new(when_false),
                },
                span,
            })
        }
        _ => Err(SemanticError::new(
            SemanticErrorKind::UnknownOperator,
            format!("unknown operator `{name}`"),
            span,
        )),
    }
}

fn check_sized_conversion(
    kind: ConversionKind,
    target: &Spanned<TypeExpr>,
    value: &Spanned<Expr>,
    context: &ModuleContext,
    span: Span,
) -> Result<TypedExpr, SemanticError> {
    let target_type = type_from_ast(&target.value, &context.generics)?;
    let (target_signed, target_width) = vector_parts(&target_type).ok_or_else(|| {
        SemanticError::new(
            SemanticErrorKind::InvalidConversionTarget,
            "conversion target must be an unsigned or signed vector",
            target.span,
        )
    })?;
    let source = check_expr(value, None, context)?;
    let (source_signed, source_width) = vector_parts(&source.ty).ok_or_else(|| {
        SemanticError::new(
            SemanticErrorKind::ConversionSourceNotVector,
            "conversion source must have a known vector width",
            value.span,
        )
    })?;
    if target_signed != source_signed {
        return Err(SemanticError::new(
            SemanticErrorKind::ConversionSignednessMismatch,
            "resize and truncate cannot change signedness",
            span,
        ));
    }
    let valid = match kind {
        ConversionKind::Resize => prove_ge(&target_width, &source_width, &context.generics)?,
        ConversionKind::Truncate => prove_ge(&source_width, &target_width, &context.generics)?,
        _ => false,
    };
    if !valid {
        let definitely_wrong = match (&target_width, &source_width, kind) {
            (WidthExpr::Constant(t), WidthExpr::Constant(s), ConversionKind::Resize) => t < s,
            (WidthExpr::Constant(t), WidthExpr::Constant(s), ConversionKind::Truncate) => t > s,
            _ => false,
        };
        let error_kind = if definitely_wrong {
            if kind == ConversionKind::Resize {
                SemanticErrorKind::InvalidResizeDirection
            } else {
                SemanticErrorKind::InvalidTruncateDirection
            }
        } else {
            SemanticErrorKind::WidthRelationUnknown
        };
        return Err(SemanticError::new(
            error_kind,
            if kind == ConversionKind::Resize {
                "cannot prove that resize target width is at least source width"
            } else {
                "cannot prove that truncate target width is at most source width"
            },
            span,
        ));
    }
    Ok(TypedExpr {
        kind: TypedExprKind::Convert {
            kind,
            value: Box::new(source),
            target_type: target_type.clone(),
        },
        ty: target_type,
        span,
    })
}

fn check_slice(
    value: &Spanned<Expr>,
    offset: &Spanned<ConstExprAst>,
    width: &Spanned<ConstExprAst>,
    context: &ModuleContext,
    span: Span,
) -> Result<TypedExpr, SemanticError> {
    let value = check_expr(value, None, context)?;
    let (_, source_width) = vector_parts(&value.ty).ok_or_else(|| {
        SemanticError::new(
            SemanticErrorKind::SliceSourceNotVector,
            "slice source must be an unsigned or signed vector",
            value.span,
        )
    })?;
    let offset = normalize_width(resolve_const(offset, &context.generics)?)?;
    let width = resolve_width(width, &context.generics)?;
    let end = normalize_width(WidthExpr::Add(
        Box::new(offset.clone()),
        Box::new(width.clone()),
    ))?;
    if !prove_ge(&source_width, &end, &context.generics)? {
        let kind = if matches!((&source_width,&end),(WidthExpr::Constant(s),WidthExpr::Constant(e)) if e>s)
        {
            SemanticErrorKind::SliceOutOfBounds
        } else {
            SemanticErrorKind::SliceRangeUnknown
        };
        return Err(SemanticError::new(
            kind,
            "cannot prove that slice range fits within source width",
            span,
        ));
    }
    let ty = unsigned_width_type(width.clone()).ok_or_else(|| {
        SemanticError::new(
            SemanticErrorKind::ConcatWidthOverflow,
            "slice result width exceeds supported concrete width",
            span,
        )
    })?;
    Ok(TypedExpr {
        kind: TypedExprKind::Slice {
            value: Box::new(value),
            offset,
            width,
        },
        ty,
        span,
    })
}
fn check_concat(
    values: &[Spanned<Expr>],
    context: &ModuleContext,
    span: Span,
) -> Result<TypedExpr, SemanticError> {
    if values.len() < 2 {
        return Err(SemanticError::new(
            SemanticErrorKind::InvalidConcatOperand,
            "concat requires at least two operands",
            span,
        ));
    }
    let mut operands = Vec::new();
    for value in values {
        let typed = check_expr(value, None, context)?;
        if let TypedExprKind::Concat { values } = typed.kind {
            operands.extend(values);
        } else {
            operands.push(typed);
        }
    }
    let mut width = WidthExpr::Constant(0);
    for operand in &operands {
        let part = operand_width(&operand.ty).ok_or_else(|| {
            SemanticError::new(
                SemanticErrorKind::InvalidConcatOperand,
                "concat operand must be bit or vector",
                operand.span,
            )
        })?;
        width =
            normalize_width(WidthExpr::Add(Box::new(width), Box::new(part))).map_err(|mut e| {
                e.kind = SemanticErrorKind::ConcatWidthOverflow;
                e
            })?;
    }
    let ty = unsigned_width_type(width.clone()).ok_or_else(|| {
        SemanticError::new(
            SemanticErrorKind::ConcatWidthOverflow,
            "concat result width exceeds supported concrete width",
            span,
        )
    })?;
    Ok(TypedExpr {
        kind: TypedExprKind::Concat { values: operands },
        ty,
        span,
    })
}
fn operand_width(ty: &HardwareType) -> Option<WidthExpr> {
    if *ty == HardwareType::Bit {
        Some(WidthExpr::Constant(1))
    } else {
        vector_parts(ty).map(|(_, w)| w)
    }
}
fn unsigned_width_type(width: WidthExpr) -> Option<HardwareType> {
    match width {
        WidthExpr::Constant(v) => u32::try_from(v).ok().map(HardwareType::Unsigned),
        value => Some(HardwareType::SymbolicUnsigned(value)),
    }
}
fn concat_hint(values: &[Spanned<Expr>], context: &ModuleContext) -> Option<HardwareType> {
    let mut width = WidthExpr::Constant(0);
    for value in values {
        let ty = type_hint(value, context)?;
        width = normalize_width(WidthExpr::Add(
            Box::new(width),
            Box::new(operand_width(&ty)?),
        ))
        .ok()?;
    }
    unsigned_width_type(width)
}

fn vector_parts(ty: &HardwareType) -> Option<(bool, WidthExpr)> {
    match ty {
        HardwareType::Unsigned(w) => Some((false, WidthExpr::Constant(u64::from(*w)))),
        HardwareType::Signed(w) => Some((true, WidthExpr::Constant(u64::from(*w)))),
        HardwareType::SymbolicUnsigned(w) => Some((false, w.clone())),
        HardwareType::SymbolicSigned(w) => Some((true, w.clone())),
        HardwareType::Bit => None,
    }
}
fn reinterpret_type(ty: &HardwareType, to_signed: bool) -> Option<HardwareType> {
    match (ty, to_signed) {
        (HardwareType::Unsigned(w), true) => Some(HardwareType::Signed(*w)),
        (HardwareType::SymbolicUnsigned(w), true) => Some(HardwareType::SymbolicSigned(w.clone())),
        (HardwareType::Signed(w), false) => Some(HardwareType::Unsigned(*w)),
        (HardwareType::SymbolicSigned(w), false) => Some(HardwareType::SymbolicUnsigned(w.clone())),
        _ => None,
    }
}
fn prove_ge(
    big: &WidthExpr,
    small: &WidthExpr,
    generics: &[TypedGeneric],
) -> Result<bool, SemanticError> {
    if big == small {
        return Ok(true);
    }
    if let (WidthExpr::Constant(a), WidthExpr::Constant(b)) = (big, small) {
        return Ok(a >= b);
    }
    if let WidthExpr::Constant(s) = small
        && minimum_with_generics(big, generics)? >= *s
    {
        return Ok(true);
    }
    let mut big_terms = Vec::new();
    let mut small_terms = Vec::new();
    flatten_add(big, &mut big_terms);
    flatten_add(small, &mut small_terms);
    for term in small_terms {
        if let Some(index) = big_terms.iter().position(|candidate| **candidate == *term) {
            big_terms.remove(index);
        } else {
            return Ok(false);
        }
    }
    Ok(true)
}
fn flatten_add<'a>(expr: &'a WidthExpr, out: &mut Vec<&'a WidthExpr>) {
    if let WidthExpr::Add(a, b) = expr {
        flatten_add(a, out);
        flatten_add(b, out)
    } else {
        out.push(expr)
    }
}

fn check_pair(
    args: &[Spanned<Expr>],
    expected: Option<&HardwareType>,
    context: &ModuleContext,
) -> Result<(TypedExpr, TypedExpr, HardwareType), SemanticError> {
    let hint = type_hint(&args[0], context)
        .or_else(|| type_hint(&args[1], context))
        .or_else(|| expected.cloned());
    let left = check_expr(&args[0], hint.as_ref(), context)?;
    let right = check_expr(&args[1], Some(&left.ty), context)?;
    let ty = left.ty.clone();
    Ok((left, right, ty))
}

fn type_hint(expr: &Spanned<Expr>, context: &ModuleContext) -> Option<HardwareType> {
    match &expr.value {
        Expr::Reference(identifier) => {
            lookup(context, &identifier.name).map(|info| info.ty.clone())
        }
        Expr::Integer(_) => None,
        Expr::Resize { target, .. } | Expr::Truncate { target, .. } => {
            type_from_ast(&target.value, &context.generics).ok()
        }
        Expr::Slice { width, .. } => resolve_width(width, &context.generics)
            .ok()
            .and_then(unsigned_width_type),
        Expr::Concat { values } => concat_hint(values, context),
        Expr::Call { callee, arguments } => match callee.name.as_str() {
            "=" | "/=" | "<" | "<=" | ">" | ">=" => Some(HardwareType::Bit),
            "not" => arguments.first().and_then(|arg| type_hint(arg, context)),
            "and" | "or" | "xor" | "+" | "-" => {
                arguments.iter().find_map(|arg| type_hint(arg, context))
            }
            "if" => arguments
                .get(1)
                .and_then(|arg| type_hint(arg, context))
                .or_else(|| arguments.get(2).and_then(|arg| type_hint(arg, context))),
            "as-signed" => arguments
                .first()
                .and_then(|arg| type_hint(arg, context))
                .and_then(|ty| reinterpret_type(&ty, true)),
            "as-unsigned" => arguments
                .first()
                .and_then(|arg| type_hint(arg, context))
                .and_then(|ty| reinterpret_type(&ty, false)),
            _ => None,
        },
    }
}

fn check_integer(
    value: i64,
    ty: &HardwareType,
    generics: &[TypedGeneric],
    span: Span,
) -> Result<(), SemanticError> {
    let value = i128::from(value);
    let fits = match ty {
        HardwareType::Bit => matches!(value, 0 | 1),
        HardwareType::Unsigned(width) => {
            value >= 0 && u64::from(*width) >= unsigned_bits(value as u128)
        }
        HardwareType::Signed(width) => u64::from(*width) >= signed_bits(value),
        HardwareType::SymbolicUnsigned(width) => {
            value >= 0 && minimum_with_generics(width, generics)? >= unsigned_bits(value as u128)
        }
        HardwareType::SymbolicSigned(width) => {
            minimum_with_generics(width, generics)? >= signed_bits(value)
        }
    };
    if fits {
        Ok(())
    } else {
        Err(SemanticError::new(
            SemanticErrorKind::IntegerOutOfRange,
            format!("integer literal does not fit {ty:?}"),
            span,
        ))
    }
}

fn require_arity(
    name: &str,
    args: &[Spanned<Expr>],
    expected: usize,
    span: Span,
) -> Result<(), SemanticError> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(SemanticError::new(
            SemanticErrorKind::WrongArgumentCount,
            format!(
                "operator `{name}` expects {expected} arguments, found {}",
                args.len()
            ),
            span,
        ))
    }
}
fn binary(
    op: BinaryOp,
    left: TypedExpr,
    right: TypedExpr,
    ty: HardwareType,
    span: Span,
) -> TypedExpr {
    TypedExpr {
        kind: TypedExprKind::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        },
        ty,
        span,
    }
}
fn lookup<'a>(context: &'a ModuleContext, name: &str) -> Option<&'a SignalInfo> {
    context
        .names
        .get(name)
        .and_then(|index| context.signals.get(*index))
}
fn type_from_ast(ty: &TypeExpr, generics: &[TypedGeneric]) -> Result<HardwareType, SemanticError> {
    match ty {
        TypeExpr::Bit => Ok(HardwareType::Bit),
        TypeExpr::Unsigned(width) => Ok(HardwareType::Unsigned(*width)),
        TypeExpr::Signed(width) => Ok(HardwareType::Signed(*width)),
        TypeExpr::SymbolicUnsigned(width) => Ok(HardwareType::SymbolicUnsigned(resolve_width(
            width, generics,
        )?)),
        TypeExpr::SymbolicSigned(width) => Ok(HardwareType::SymbolicSigned(resolve_width(
            width, generics,
        )?)),
    }
}

fn collect_generics(
    module: &ModuleDecl,
    next: &mut u32,
) -> Result<Vec<TypedGeneric>, SemanticError> {
    let mut names = HashMap::new();
    let mut result = Vec::new();
    for generic in &module.generics {
        if let Some(previous) =
            names.insert(generic.value.name.name.clone(), generic.value.name.span)
        {
            return Err(SemanticError::new(
                SemanticErrorKind::DuplicateGeneric,
                format!("duplicate generic `{}`", generic.value.name.name),
                generic.value.name.span,
            )
            .related(previous));
        }
        let kind = match generic.value.kind {
            GenericKindSyntax::Natural => GenericKind::Natural,
            GenericKindSyntax::Positive => GenericKind::Positive,
        };
        if kind == GenericKind::Positive && generic.value.default == 0 {
            return Err(SemanticError::new(
                SemanticErrorKind::InvalidGenericDefault,
                "positive generic default must be at least one",
                generic.span,
            ));
        }
        let id = GenericId(*next);
        *next = next.checked_add(1).ok_or_else(|| {
            SemanticError::new(
                SemanticErrorKind::CannotInferType,
                "too many generics",
                generic.span,
            )
        })?;
        result.push(TypedGeneric {
            id,
            name: generic.value.name.name.clone(),
            kind,
            default: generic.value.default,
            declaration_span: generic.span,
        });
    }
    Ok(result)
}

fn resolve_width(
    expr: &Spanned<ConstExprAst>,
    generics: &[TypedGeneric],
) -> Result<WidthExpr, SemanticError> {
    let value = normalize_width(resolve_const(expr, generics)?)?;
    if minimum_with_generics(&value, generics)? == 0 {
        return Err(SemanticError::new(
            SemanticErrorKind::InvalidWidthExpression,
            "type width can be zero",
            expr.span,
        ));
    }
    Ok(value)
}

fn resolve_const(
    expr: &Spanned<ConstExprAst>,
    generics: &[TypedGeneric],
) -> Result<WidthExpr, SemanticError> {
    match &expr.value {
        ConstExprAst::Integer(v) => Ok(WidthExpr::Constant(*v)),
        ConstExprAst::Reference(id) => generics
            .iter()
            .find(|g| g.name == id.name)
            .map(|g| WidthExpr::Generic(g.id))
            .ok_or_else(|| {
                SemanticError::new(
                    SemanticErrorKind::UnknownGeneric,
                    format!("unknown generic `{}`", id.name),
                    id.span,
                )
            }),
        ConstExprAst::Add(a, b) => Ok(WidthExpr::Add(
            Box::new(resolve_const(a, generics)?),
            Box::new(resolve_const(b, generics)?),
        )),
        ConstExprAst::Multiply(a, b) => Ok(WidthExpr::Multiply(
            Box::new(resolve_const(a, generics)?),
            Box::new(resolve_const(b, generics)?),
        )),
    }
}

fn resolve_generic_bindings(
    source: &[Spanned<GenericBinding>],
    parent: &[TypedGeneric],
    target: &[TypedGeneric],
    span: Span,
) -> Result<Vec<TypedGenericBinding>, SemanticError> {
    let mut supplied = HashMap::new();
    for binding in source {
        if !target.iter().any(|g| g.name == binding.value.formal.name) {
            return Err(SemanticError::new(
                SemanticErrorKind::UnknownInstanceGeneric,
                format!("unknown target generic `{}`", binding.value.formal.name),
                binding.value.formal.span,
            ));
        }
        if supplied
            .insert(binding.value.formal.name.clone(), binding)
            .is_some()
        {
            return Err(SemanticError::new(
                SemanticErrorKind::DuplicateGenericBinding,
                "generic is bound more than once",
                binding.value.formal.span,
            ));
        }
    }
    let mut result = Vec::new();
    for formal in target {
        let (value, uses_default, binding_span) = if let Some(binding) = supplied.get(&formal.name)
        {
            (
                normalize_width(resolve_const(&binding.value.value, parent)?)?,
                false,
                binding.span,
            )
        } else {
            (WidthExpr::Constant(formal.default), true, span)
        };
        let min = minimum_with_generics(&value, parent)?;
        if formal.kind == GenericKind::Positive && min == 0 {
            return Err(SemanticError::new(
                SemanticErrorKind::InvalidGenericActual,
                "positive generic actual can be zero",
                binding_span,
            ));
        }
        result.push(TypedGenericBinding {
            formal: formal.id,
            value,
            uses_default,
            span: binding_span,
        });
    }
    Ok(result)
}

fn substitute_type(
    ty: &HardwareType,
    bindings: &[TypedGenericBinding],
) -> Result<HardwareType, SemanticError> {
    fn sub(expr: &WidthExpr, bindings: &[TypedGenericBinding]) -> Result<WidthExpr, SemanticError> {
        match expr {
            WidthExpr::Constant(v) => Ok(WidthExpr::Constant(*v)),
            WidthExpr::Generic(id) => bindings
                .iter()
                .find(|b| b.formal == *id)
                .map(|b| b.value.clone())
                .ok_or_else(|| {
                    SemanticError::new(
                        SemanticErrorKind::UnknownGeneric,
                        "generic substitution is incomplete",
                        empty_span(),
                    )
                }),
            WidthExpr::Add(a, b) => normalize_width(WidthExpr::Add(
                Box::new(sub(a, bindings)?),
                Box::new(sub(b, bindings)?),
            )),
            WidthExpr::Multiply(a, b) => normalize_width(WidthExpr::Multiply(
                Box::new(sub(a, bindings)?),
                Box::new(sub(b, bindings)?),
            )),
        }
    }
    match ty {
        HardwareType::Bit => Ok(HardwareType::Bit),
        HardwareType::Unsigned(w) => Ok(HardwareType::Unsigned(*w)),
        HardwareType::Signed(w) => Ok(HardwareType::Signed(*w)),
        HardwareType::SymbolicUnsigned(w) => match sub(w, bindings)? {
            WidthExpr::Constant(v) if v <= u64::from(u32::MAX) => {
                Ok(HardwareType::Unsigned(v as u32))
            }
            v => Ok(HardwareType::SymbolicUnsigned(v)),
        },
        HardwareType::SymbolicSigned(w) => match sub(w, bindings)? {
            WidthExpr::Constant(v) if v <= u64::from(u32::MAX) => {
                Ok(HardwareType::Signed(v as u32))
            }
            v => Ok(HardwareType::SymbolicSigned(v)),
        },
    }
}

fn normalize_width(expr: WidthExpr) -> Result<WidthExpr, SemanticError> {
    match expr {
        WidthExpr::Add(a, b) => {
            let a = normalize_width(*a)?;
            let b = normalize_width(*b)?;
            match (&a, &b) {
                (WidthExpr::Constant(x), WidthExpr::Constant(y)) => {
                    x.checked_add(*y).map(WidthExpr::Constant).ok_or_else(|| {
                        SemanticError::new(
                            SemanticErrorKind::ConstExpressionOverflow,
                            "constant addition overflow",
                            empty_span(),
                        )
                    })
                }
                (WidthExpr::Constant(0), _) => Ok(b),
                (_, WidthExpr::Constant(0)) => Ok(a),
                _ => Ok(WidthExpr::Add(Box::new(a), Box::new(b))),
            }
        }
        WidthExpr::Multiply(a, b) => {
            let a = normalize_width(*a)?;
            let b = normalize_width(*b)?;
            match (&a, &b) {
                (WidthExpr::Constant(x), WidthExpr::Constant(y)) => {
                    x.checked_mul(*y).map(WidthExpr::Constant).ok_or_else(|| {
                        SemanticError::new(
                            SemanticErrorKind::ConstExpressionOverflow,
                            "constant multiplication overflow",
                            empty_span(),
                        )
                    })
                }
                (WidthExpr::Constant(0), _) | (_, WidthExpr::Constant(0)) => {
                    Ok(WidthExpr::Constant(0))
                }
                (WidthExpr::Constant(1), _) => Ok(b),
                (_, WidthExpr::Constant(1)) => Ok(a),
                _ => Ok(WidthExpr::Multiply(Box::new(a), Box::new(b))),
            }
        }
        value => Ok(value),
    }
}

fn minimum_with_generics(
    expr: &WidthExpr,
    generics: &[TypedGeneric],
) -> Result<u64, SemanticError> {
    match expr {
        WidthExpr::Generic(id) => generics
            .iter()
            .find(|g| g.id == *id)
            .map(|g| {
                if g.kind == GenericKind::Positive {
                    1
                } else {
                    0
                }
            })
            .ok_or_else(|| {
                SemanticError::new(
                    SemanticErrorKind::UnknownGeneric,
                    "unknown GenericId",
                    empty_span(),
                )
            }),
        WidthExpr::Constant(v) => Ok(*v),
        WidthExpr::Add(a, b) => minimum_with_generics(a, generics)?
            .checked_add(minimum_with_generics(b, generics)?)
            .ok_or_else(|| {
                SemanticError::new(
                    SemanticErrorKind::ConstExpressionOverflow,
                    "minimum addition overflow",
                    empty_span(),
                )
            }),
        WidthExpr::Multiply(a, b) => minimum_with_generics(a, generics)?
            .checked_mul(minimum_with_generics(b, generics)?)
            .ok_or_else(|| {
                SemanticError::new(
                    SemanticErrorKind::ConstExpressionOverflow,
                    "minimum multiplication overflow",
                    empty_span(),
                )
            }),
    }
}
fn unsigned_bits(v: u128) -> u64 {
    if v == 0 {
        1
    } else {
        (128 - v.leading_zeros()) as u64
    }
}
fn signed_bits(v: i128) -> u64 {
    for bits in 1..128 {
        if let Some(bound) = 1_i128.checked_shl(bits - 1)
            && v >= -bound
            && v < bound
        {
            return bits.into();
        }
    }
    128
}
