use std::{
    collections::{HashMap, HashSet},
    fmt,
};

use crate::{
    BinaryOp, CaseKey, ClockEdge, ClockEdgeSyntax, ClockedBlockId, ClockedDecl, ConstExprAst,
    ConversionKind, EnumId, EnumMemberId, Expr, GenericBinding, GenericId, GenericKind,
    GenericKindSyntax, HardwareType, InstanceId, ModuleDecl, ModuleId, ModuleItem, NextStmt,
    PortDirection, Program, RomId, SignalId, SignalKind, SimulationTime, Span, Spanned,
    StaticBitMotionKind, StaticBitMotionSyntaxKind, TestbenchDecl, TestbenchId, TestbenchStmt,
    TypeExpr, TypedAssign, TypedCaseDo, TypedCaseDoArm, TypedCaseExprArm, TypedClockedBlock,
    TypedEnum, TypedEnumMember, TypedExpr, TypedExprKind, TypedGeneric, TypedGenericBinding,
    TypedInstance, TypedModule, TypedNext, TypedPortConnection, TypedProgram, TypedRegisterArray,
    TypedRegisterArrayWrite, TypedReset, TypedRom, TypedRomEntry, TypedSignal, TypedTestbench,
    TypedTestbenchClock, TypedTestbenchStmt, UnaryOp, WidthExpr,
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
    InvalidReverseBitsSource,
    InvalidStaticBitMotionSource,
    InvalidStaticBitMotionSignedness,
    StaticBitMotionAmountOutOfRange,
    StaticBitMotionAmountRangeUnknown,
    InvalidBitAtSource,
    BitAtIndexOutOfRange,
    BitAtIndexRangeUnknown,
    InvalidCaseSelector,
    InvalidCaseLabel,
    DuplicateCaseLabel,
    CaseBranchTypeMismatch,
    DuplicateEnum,
    UnknownEnum,
    DuplicateEnumMember,
    DuplicateEnumValue,
    InvalidEnumValue,
    InvalidEnumType,
    InvalidEnumConversion,
    EnumTypeMismatch,
    DuplicateRom,
    UnknownRom,
    InvalidRomWidth,
    InvalidRomDefault,
    InvalidRomEntry,
    DuplicateRomAddress,
    InvalidRomAddress,
    InvalidRomData,
    InvalidRomRead,
    DuplicateRegisterArray,
    UnknownRegisterArray,
    InvalidRegisterArrayWidth,
    InvalidRegisterArrayInitial,
    InvalidRegisterArrayRead,
    InvalidRegisterArrayWrite,
    RegisterArrayWriteOutsideClocked,
    MultipleRegisterArrayWrites,
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
    enums: Vec<TypedEnum>,
    roms: Vec<TypedRom>,
    register_arrays: Vec<TypedRegisterArray>,
}

pub fn analyze_program(program: &Program) -> Result<TypedProgram, SemanticError> {
    let enums = collect_enums(&program.enums)?;
    let roms = collect_roms(&program.roms)?;
    let mut top_level_names = HashMap::<&str, Span>::new();
    for declaration in &program.enums {
        top_level_names.insert(&declaration.value.name.name, declaration.value.name.span);
    }
    for declaration in &program.roms {
        if let Some(previous) =
            top_level_names.insert(&declaration.value.name.name, declaration.value.name.span)
        {
            return Err(SemanticError::new(
                SemanticErrorKind::DuplicateRom,
                format!("duplicate top-level name `{}`", declaration.value.name.name),
                declaration.value.name.span,
            )
            .related(previous));
        }
    }
    let mut module_names = HashMap::<&str, Span>::new();
    let mut next_signal = 0_u32;
    let mut next_clocked = 0_u32;
    let mut next_generic = 0_u32;
    let mut modules = Vec::with_capacity(program.modules.len());
    for (index, module) in program.modules.iter().enumerate() {
        if let Some(previous) = top_level_names.get(module.value.name.name.as_str()) {
            return Err(SemanticError::new(
                SemanticErrorKind::DuplicateModule,
                format!("duplicate top-level name `{}`", module.value.name.name),
                module.value.name.span,
            )
            .related(*previous));
        }
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
            &enums,
            &roms,
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
        enums: enums.clone(),
        roms: roms.clone(),
        modules,
        testbenches,
        module_order,
    })
}

fn collect_enums(source: &[Spanned<crate::EnumDecl>]) -> Result<Vec<TypedEnum>, SemanticError> {
    let mut names = HashMap::new();
    let mut result = Vec::new();
    for (enum_index, declaration) in source.iter().enumerate() {
        if let Some(previous) = names.insert(
            declaration.value.name.name.clone(),
            declaration.value.name.span,
        ) {
            return Err(SemanticError::new(
                SemanticErrorKind::DuplicateEnum,
                "duplicate enum name",
                declaration.value.name.span,
            )
            .related(previous));
        }
        let mut member_names = HashMap::new();
        let mut values = HashMap::new();
        let mut members = Vec::new();
        for (member_index, member) in declaration.value.members.iter().enumerate() {
            if let Some(previous) =
                member_names.insert(member.value.name.name.clone(), member.value.name.span)
            {
                return Err(SemanticError::new(
                    SemanticErrorKind::DuplicateEnumMember,
                    "duplicate enum member name",
                    member.value.name.span,
                )
                .related(previous));
            }
            if let Some(previous) = values.insert(member.value.value, member.value.name.span) {
                return Err(SemanticError::new(
                    SemanticErrorKind::DuplicateEnumValue,
                    "duplicate enum member value",
                    member.value.name.span,
                )
                .related(previous));
            }
            if declaration.value.width < 64
                && member.value.value >= (1_u64 << declaration.value.width)
            {
                return Err(SemanticError::new(
                    SemanticErrorKind::InvalidEnumValue,
                    "enum member value does not fit enum width",
                    member.span,
                ));
            }
            members.push(TypedEnumMember {
                id: EnumMemberId(member_index as u32),
                enum_id: EnumId(enum_index as u32),
                name: member.value.name.name.clone(),
                value: member.value.value,
                span: member.span,
            });
        }
        result.push(TypedEnum {
            id: EnumId(enum_index as u32),
            name: declaration.value.name.name.clone(),
            width: declaration.value.width,
            members,
            span: declaration.span,
        });
    }
    Ok(result)
}

fn collect_roms(source: &[Spanned<crate::RomDecl>]) -> Result<Vec<TypedRom>, SemanticError> {
    let mut names = HashMap::new();
    let mut result = Vec::new();
    for (index, declaration) in source.iter().enumerate() {
        if let Some(previous) = names.insert(
            declaration.value.name.name.clone(),
            declaration.value.name.span,
        ) {
            return Err(SemanticError::new(
                SemanticErrorKind::DuplicateRom,
                "duplicate ROM name",
                declaration.value.name.span,
            )
            .related(previous));
        }
        let address_width = declaration.value.address_width;
        let data_width = declaration.value.data_width;
        let depth = 1_u64.checked_shl(address_width).ok_or_else(|| {
            SemanticError::new(
                SemanticErrorKind::InvalidRomWidth,
                "ROM address width is too large",
                declaration.span,
            )
        })?;
        let fits = |value: u64| data_width >= 64 || value < (1_u64 << data_width);
        if !fits(declaration.value.default_value) {
            return Err(SemanticError::new(
                SemanticErrorKind::InvalidRomDefault,
                "ROM default value does not fit data width",
                declaration.span,
            ));
        }
        let mut addresses = HashMap::new();
        let mut entries = Vec::new();
        for entry in &declaration.value.entries {
            if entry.value.address >= depth {
                return Err(SemanticError::new(
                    SemanticErrorKind::InvalidRomAddress,
                    "ROM entry address is outside depth",
                    entry.span,
                ));
            }
            if !fits(entry.value.value) {
                return Err(SemanticError::new(
                    SemanticErrorKind::InvalidRomData,
                    "ROM entry data does not fit data width",
                    entry.span,
                ));
            }
            if let Some(previous) = addresses.insert(entry.value.address, entry.span) {
                return Err(SemanticError::new(
                    SemanticErrorKind::DuplicateRomAddress,
                    "duplicate ROM entry address",
                    entry.span,
                )
                .related(previous));
            }
            entries.push(TypedRomEntry {
                address: entry.value.address,
                value: entry.value.value,
                span: entry.span,
            });
        }
        entries.sort_by_key(|entry| entry.address);
        result.push(TypedRom {
            id: RomId(u32::try_from(index).map_err(|_| {
                SemanticError::new(
                    SemanticErrorKind::CannotInferType,
                    "too many ROMs",
                    declaration.span,
                )
            })?),
            name: declaration.value.name.name.clone(),
            address_width,
            data_width,
            depth,
            default_value: declaration.value.default_value,
            entries,
            span: declaration.span,
        });
    }
    Ok(result)
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
    enums: &[TypedEnum],
    roms: &[TypedRom],
) -> Result<TypedModule, SemanticError> {
    let generics = collect_generics(&module.value, next_generic_id)?;
    let context = collect_signals(&module.value, next_id, &generics, enums, roms)?;
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
    let mut array_drivers = HashMap::new();
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
                &mut array_drivers,
            )?);
        }
    }
    Ok(TypedModule {
        enums: enums.to_vec(),
        roms: roms.to_vec(),
        register_arrays: context.register_arrays.clone(),
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
        enums: module.enums.clone(),
        roms: module.roms.clone(),
        register_arrays: module.register_arrays.clone(),
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
    array_drivers: &mut HashMap<crate::RegisterArrayId, Span>,
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
    let mut writes = Vec::new();
    let mut region_arrays = HashMap::new();
    for write in &clocked.writes {
        let typed = analyze_array_write(write, context, &mut region_arrays)?;
        if let Some(previous) = array_drivers.insert(typed.array_id, write.span) {
            return Err(SemanticError::new(
                SemanticErrorKind::MultipleRegisterArrayWrites,
                "register-array is written by multiple clocked blocks",
                write.span,
            )
            .related(previous));
        }
        writes.push(typed);
    }
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
    let mut region_targets = normal_seen.clone();
    let mut case_dos = Vec::new();
    for case_do in &clocked.case_dos {
        let (typed, targets) = analyze_case_do(&case_do.value, case_do.span, context)?;
        for (target, target_span) in targets {
            if region_targets.insert(target, target_span).is_some() {
                return Err(SemanticError::new(
                    SemanticErrorKind::DuplicateNext,
                    "register is updated more than once in one clocked region",
                    target_span,
                ));
            }
            if let Some(previous) = register_drivers.insert(target, target_span) {
                return Err(SemanticError::new(
                    SemanticErrorKind::RegisterMultipleClockedDrivers,
                    "register is driven by multiple clocked blocks",
                    target_span,
                )
                .related(previous));
            }
        }
        for (array_id, write_span) in case_do_write_spans(&typed) {
            if let Some(previous) = region_arrays.insert(array_id, write_span) {
                return Err(SemanticError::new(
                    SemanticErrorKind::MultipleRegisterArrayWrites,
                    "register-array is written more than once in one clocked region",
                    write_span,
                )
                .related(previous));
            }
            if let Some(previous) = array_drivers.insert(array_id, write_span) {
                return Err(SemanticError::new(
                    SemanticErrorKind::MultipleRegisterArrayWrites,
                    "register-array is written by multiple clocked blocks",
                    write_span,
                )
                .related(previous));
            }
        }
        case_dos.push(typed);
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
            for target in region_targets.keys() {
                if !reset_seen.contains_key(target) {
                    return Err(SemanticError::new(
                        SemanticErrorKind::ResetTargetMissing,
                        "reset is missing a normally updated register",
                        reset.span,
                    ));
                }
            }
            for (target, target_span) in &reset_seen {
                if !region_targets.contains_key(target) {
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
        edge: match clocked.edge {
            ClockEdgeSyntax::Rising => ClockEdge::Rising,
            ClockEdgeSyntax::Falling => ClockEdge::Falling,
        },
        reset,
        updates,
        case_dos,
        writes,
        span,
    })
}

fn analyze_case_do(
    case_do: &crate::CaseDoStmt,
    span: Span,
    context: &ModuleContext,
) -> Result<(TypedCaseDo, HashMap<SignalId, Span>), SemanticError> {
    let selector = check_expr(&case_do.selector, None, context)?;
    if selector.ty != HardwareType::Bit
        && vector_parts(&selector.ty).is_none()
        && !matches!(selector.ty, HardwareType::Enum(_, _))
    {
        return Err(SemanticError::new(
            SemanticErrorKind::InvalidCaseSelector,
            "case-do selector must be bit or vector",
            selector.span,
        ));
    }
    let mut labels = HashMap::new();
    let mut all_targets = HashMap::new();
    let mut arms = Vec::new();
    for arm in &case_do.arms {
        let key = case_key(&arm.value.label, &selector.ty, context)?;
        if let Some(previous) = labels.insert(key.value, key.span) {
            return Err(SemanticError::new(
                SemanticErrorKind::DuplicateCaseLabel,
                "duplicate case label",
                key.span,
            )
            .related(previous));
        }
        let mut seen = HashMap::new();
        let body = arm
            .value
            .body
            .iter()
            .map(|next| analyze_next(next, context, &mut seen))
            .collect::<Result<Vec<_>, _>>()?;
        let mut writes = Vec::new();
        let mut array_seen = HashMap::new();
        for write in &arm.value.writes {
            writes.push(analyze_array_write(write, context, &mut array_seen)?);
        }
        let mut nested = Vec::new();
        for child in &arm.value.nested {
            let (typed, child_targets) = analyze_case_do(&child.value, child.span, context)?;
            for (target, target_span) in child_targets {
                if let Some(previous) = seen.insert(target, target_span) {
                    return Err(SemanticError::new(
                        SemanticErrorKind::DuplicateNext,
                        "register is updated more than once in one case-do arm",
                        target_span,
                    )
                    .related(previous));
                }
                all_targets.entry(target).or_insert(target_span);
            }
            for (array_id, write_span) in case_do_write_spans(&typed) {
                if let Some(previous) = array_seen.insert(array_id, write_span) {
                    return Err(SemanticError::new(
                        SemanticErrorKind::MultipleRegisterArrayWrites,
                        "register-array is written more than once in one case-do arm",
                        write_span,
                    )
                    .related(previous));
                }
            }
            nested.push(typed);
        }
        for (target, target_span) in seen {
            all_targets.entry(target).or_insert(target_span);
        }
        arms.push(TypedCaseDoArm {
            key,
            body,
            writes,
            nested,
            span: arm.span,
        });
    }
    let mut else_seen = HashMap::new();
    let else_body = case_do
        .else_body
        .as_ref()
        .map(|body| {
            body.iter()
                .map(|next| analyze_next(next, context, &mut else_seen))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;
    let mut else_writes = Vec::new();
    let mut else_array_seen = HashMap::new();
    for write in &case_do.else_writes {
        else_writes.push(analyze_array_write(write, context, &mut else_array_seen)?);
    }
    let mut else_nested = Vec::new();
    for child in &case_do.else_nested {
        let (typed, child_targets) = analyze_case_do(&child.value, child.span, context)?;
        for (target, target_span) in child_targets {
            if let Some(previous) = else_seen.insert(target, target_span) {
                return Err(SemanticError::new(
                    SemanticErrorKind::DuplicateNext,
                    "register is updated more than once in one case-do arm",
                    target_span,
                )
                .related(previous));
            }
            all_targets.entry(target).or_insert(target_span);
        }
        for (array_id, write_span) in case_do_write_spans(&typed) {
            if let Some(previous) = else_array_seen.insert(array_id, write_span) {
                return Err(SemanticError::new(
                    SemanticErrorKind::MultipleRegisterArrayWrites,
                    "register-array is written more than once in one case-do arm",
                    write_span,
                )
                .related(previous));
            }
        }
        else_nested.push(typed);
    }
    for (target, target_span) in else_seen {
        all_targets.entry(target).or_insert(target_span);
    }
    Ok((
        TypedCaseDo {
            selector,
            arms,
            else_body,
            else_writes,
            else_nested,
            span,
        },
        all_targets,
    ))
}

fn case_do_write_spans(case_do: &TypedCaseDo) -> HashMap<crate::RegisterArrayId, Span> {
    let mut writes = HashMap::new();
    for arm in &case_do.arms {
        for write in &arm.writes {
            writes.entry(write.array_id).or_insert(write.span);
        }
        for nested in &arm.nested {
            for (array_id, span) in case_do_write_spans(nested) {
                writes.entry(array_id).or_insert(span);
            }
        }
    }
    for write in &case_do.else_writes {
        writes.entry(write.array_id).or_insert(write.span);
    }
    for nested in &case_do.else_nested {
        for (array_id, span) in case_do_write_spans(nested) {
            writes.entry(array_id).or_insert(span);
        }
    }
    writes
}

fn analyze_array_write(
    write: &Spanned<crate::RegisterArrayWrite>,
    context: &ModuleContext,
    seen: &mut HashMap<crate::RegisterArrayId, Span>,
) -> Result<TypedRegisterArrayWrite, SemanticError> {
    let array = context
        .register_arrays
        .iter()
        .find(|array| array.name == write.value.array.name)
        .ok_or_else(|| {
            SemanticError::new(
                SemanticErrorKind::UnknownRegisterArray,
                "unknown register-array",
                write.value.array.span,
            )
        })?;
    if let Some(previous) = seen.insert(array.id, write.span) {
        return Err(SemanticError::new(
            SemanticErrorKind::MultipleRegisterArrayWrites,
            "register-array is written more than once in one clocked region",
            write.span,
        )
        .related(previous));
    }
    let address_type = HardwareType::Unsigned(array.address_width);
    let address = check_expr(&write.value.address, Some(&address_type), context).map_err(|_| {
        SemanticError::new(
            SemanticErrorKind::InvalidRegisterArrayWrite,
            "register-array write address must be exact-width unsigned",
            write.value.address.span,
        )
    })?;
    let value_type = HardwareType::Unsigned(array.data_width);
    let value = check_expr(&write.value.value, Some(&value_type), context).map_err(|_| {
        SemanticError::new(
            SemanticErrorKind::InvalidRegisterArrayWrite,
            "register-array write value must be exact-width unsigned",
            write.value.value.span,
        )
    })?;
    if address.ty != address_type || value.ty != value_type {
        return Err(SemanticError::new(
            SemanticErrorKind::InvalidRegisterArrayWrite,
            "register-array write type mismatch",
            write.span,
        ));
    }
    Ok(TypedRegisterArrayWrite {
        array_id: array.id,
        address,
        value,
        span: write.span,
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
    enums: &[TypedEnum],
    roms: &[TypedRom],
) -> Result<ModuleContext, SemanticError> {
    let mut context = ModuleContext {
        signals: Vec::new(),
        names: HashMap::new(),
        generics: generics.to_vec(),
        enums: enums.to_vec(),
        roms: roms.to_vec(),
        register_arrays: Vec::new(),
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
            type_from_ast(&port.value.ty.value, generics, enums)?,
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
                type_from_ast(&wire.ty.value, generics, enums)?,
                item.span,
                None,
            )?,
            ModuleItem::Register(reg) => add_signal(
                &mut context,
                next_id,
                &reg.name.name,
                SignalClass::Register,
                type_from_ast(&reg.ty.value, generics, enums)?,
                item.span,
                reg.initial.clone(),
            )?,
            ModuleItem::RegisterArray(array) => {
                if context.names.contains_key(&array.name.name)
                    || context
                        .register_arrays
                        .iter()
                        .any(|item| item.name == array.name.name)
                {
                    return Err(SemanticError::new(
                        SemanticErrorKind::DuplicateRegisterArray,
                        format!("duplicate register-array `{}`", array.name.name),
                        item.span,
                    ));
                }
                let depth = 1_u64.checked_shl(array.address_width).ok_or_else(|| {
                    SemanticError::new(
                        SemanticErrorKind::InvalidRegisterArrayWidth,
                        "register-array address width is too large",
                        item.span,
                    )
                })?;
                let fits =
                    |value: u64| array.data_width >= 64 || value < (1_u64 << array.data_width);
                if !fits(array.initial_value) {
                    return Err(SemanticError::new(
                        SemanticErrorKind::InvalidRegisterArrayInitial,
                        "register-array initial value does not fit data width",
                        item.span,
                    ));
                }
                let id = crate::RegisterArrayId(
                    u32::try_from(context.register_arrays.len()).map_err(|_| {
                        SemanticError::new(
                            SemanticErrorKind::CannotInferType,
                            "too many register arrays",
                            item.span,
                        )
                    })?,
                );
                context.register_arrays.push(TypedRegisterArray {
                    id,
                    name: array.name.name.clone(),
                    address_width: array.address_width,
                    data_width: array.data_width,
                    depth,
                    initial_value: array.initial_value,
                    span: item.span,
                });
            }
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
            if let Some(value) = resolve_enum_member(identifier, context)? {
                value
            } else {
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
        Expr::StaticBitMotion {
            kind,
            value,
            amount,
        } => {
            let kind = match kind {
                StaticBitMotionSyntaxKind::ShiftLeft => StaticBitMotionKind::ShiftLeft,
                StaticBitMotionSyntaxKind::ShiftRightLogical => {
                    StaticBitMotionKind::ShiftRightLogical
                }
                StaticBitMotionSyntaxKind::ShiftRightArithmetic => {
                    StaticBitMotionKind::ShiftRightArithmetic
                }
                StaticBitMotionSyntaxKind::RotateLeft => StaticBitMotionKind::RotateLeft,
                StaticBitMotionSyntaxKind::RotateRight => StaticBitMotionKind::RotateRight,
            };
            check_static_bit_motion(kind, value, amount, context, expr.span)?
        }
        Expr::BitAt { source, index } => check_bit_at(source, index, context, expr.span)?,
        Expr::Case {
            selector,
            arms,
            else_expr,
        } => check_case_expr(selector, arms, else_expr, expected, context, expr.span)?,
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

fn resolve_enum_member(
    identifier: &crate::Identifier,
    context: &ModuleContext,
) -> Result<Option<TypedExpr>, SemanticError> {
    let Some((enum_name, member_name)) = identifier.name.split_once('.') else {
        return Ok(None);
    };
    let Some(enumeration) = context.enums.iter().find(|item| item.name == enum_name) else {
        return Err(SemanticError::new(
            SemanticErrorKind::UnknownEnum,
            format!("unknown enum `{enum_name}`"),
            identifier.span,
        ));
    };
    let Some(member) = enumeration
        .members
        .iter()
        .find(|item| item.name == member_name)
    else {
        return Err(SemanticError::new(
            SemanticErrorKind::InvalidEnumType,
            format!("unknown enum member `{member_name}`"),
            identifier.span,
        ));
    };
    Ok(Some(TypedExpr {
        kind: TypedExprKind::EnumValue {
            enum_id: enumeration.id,
            member_id: member.id,
            value: member.value,
        },
        ty: HardwareType::Enum(enumeration.id, enumeration.width),
        span: identifier.span,
    }))
}

fn check_static_bit_motion(
    kind: StaticBitMotionKind,
    value: &Spanned<Expr>,
    amount: &Spanned<ConstExprAst>,
    context: &ModuleContext,
    span: Span,
) -> Result<TypedExpr, SemanticError> {
    let value = check_expr(value, None, context)?;
    let (signed, source_width) = vector_parts(&value.ty).ok_or_else(|| {
        SemanticError::new(
            SemanticErrorKind::InvalidStaticBitMotionSource,
            "shift and rotate source must be an unsigned or signed vector",
            value.span,
        )
    })?;
    if kind == StaticBitMotionKind::ShiftRightLogical && signed {
        return Err(SemanticError::new(
            SemanticErrorKind::InvalidStaticBitMotionSignedness,
            "shift-right-logical requires an unsigned vector",
            value.span,
        ));
    }
    if kind == StaticBitMotionKind::ShiftRightArithmetic && !signed {
        return Err(SemanticError::new(
            SemanticErrorKind::InvalidStaticBitMotionSignedness,
            "shift-right-arithmetic requires a signed vector",
            value.span,
        ));
    }
    let amount_span = amount.span;
    let amount = normalize_width(resolve_const(amount, &context.generics)?)?;
    let end = normalize_width(WidthExpr::Add(
        Box::new(amount.clone()),
        Box::new(WidthExpr::Constant(1)),
    ))?;
    if !prove_ge(&source_width, &end, &context.generics)? {
        let definitely_out = matches!(
            (&source_width, &amount),
            (WidthExpr::Constant(width), WidthExpr::Constant(amount)) if amount >= width
        );
        return Err(SemanticError::new(
            if definitely_out {
                SemanticErrorKind::StaticBitMotionAmountOutOfRange
            } else {
                SemanticErrorKind::StaticBitMotionAmountRangeUnknown
            },
            "cannot prove that shift or rotate amount is less than source width",
            amount_span,
        ));
    }
    Ok(TypedExpr {
        kind: TypedExprKind::StaticBitMotion {
            kind,
            value: Box::new(value.clone()),
            amount,
        },
        ty: value.ty,
        span,
    })
}

fn check_bit_at(
    source: &Spanned<Expr>,
    index: &Spanned<ConstExprAst>,
    context: &ModuleContext,
    span: Span,
) -> Result<TypedExpr, SemanticError> {
    let source = check_expr(source, None, context)?;
    let (_, source_width) = vector_parts(&source.ty).ok_or_else(|| {
        SemanticError::new(
            SemanticErrorKind::InvalidBitAtSource,
            "bit-at source must be an unsigned or signed vector",
            source.span,
        )
    })?;
    let index_span = index.span;
    let index = normalize_width(resolve_const(index, &context.generics)?)?;
    let end = normalize_width(WidthExpr::Add(
        Box::new(index.clone()),
        Box::new(WidthExpr::Constant(1)),
    ))?;
    if !prove_ge(&source_width, &end, &context.generics)? {
        let out = matches!(
            (&source_width, &index),
            (WidthExpr::Constant(width), WidthExpr::Constant(index)) if index >= width
        );
        return Err(SemanticError::new(
            if out {
                SemanticErrorKind::BitAtIndexOutOfRange
            } else {
                SemanticErrorKind::BitAtIndexRangeUnknown
            },
            "cannot prove that bit-at index is less than source width",
            index_span,
        ));
    }
    Ok(TypedExpr {
        kind: TypedExprKind::BitAt {
            source: Box::new(source),
            index,
        },
        ty: HardwareType::Bit,
        span,
    })
}

fn case_key(
    label: &Spanned<Expr>,
    ty: &HardwareType,
    context: &ModuleContext,
) -> Result<CaseKey, SemanticError> {
    if let HardwareType::Enum(enum_id, _) = ty {
        let crate::Expr::Reference(identifier) = &label.value else {
            return Err(SemanticError::new(
                SemanticErrorKind::InvalidCaseLabel,
                "enum case label must be an enum member",
                label.span,
            ));
        };
        let Some(value) = resolve_enum_member(identifier, context)? else {
            return Err(SemanticError::new(
                SemanticErrorKind::InvalidCaseLabel,
                "enum case label must be an enum member",
                label.span,
            ));
        };
        let TypedExprKind::EnumValue {
            enum_id: label_enum,
            member_id,
            value: encoded,
        } = value.kind
        else {
            return Err(SemanticError::new(
                SemanticErrorKind::InvalidCaseLabel,
                "invalid enum case label",
                label.span,
            ));
        };
        if label_enum != *enum_id {
            return Err(SemanticError::new(
                SemanticErrorKind::EnumTypeMismatch,
                "case label enum type does not match selector",
                label.span,
            ));
        }
        return Ok(CaseKey {
            value: encoded as i64,
            ty: ty.clone(),
            span: label.span,
            enum_member: Some(member_id),
        });
    }
    let Expr::Integer(value) = label.value else {
        return Err(SemanticError::new(
            SemanticErrorKind::InvalidCaseLabel,
            "case label must be a static integer",
            label.span,
        ));
    };
    check_integer(value, ty, &context.generics, label.span)?;
    Ok(CaseKey {
        value,
        ty: ty.clone(),
        span: label.span,
        enum_member: None,
    })
}

fn check_case_expr(
    selector: &Spanned<Expr>,
    arms: &[Spanned<crate::CaseExprArm>],
    else_expr: &Spanned<Expr>,
    expected: Option<&HardwareType>,
    context: &ModuleContext,
    span: Span,
) -> Result<TypedExpr, SemanticError> {
    let selector = check_expr(selector, None, context)?;
    if selector.ty != HardwareType::Bit
        && vector_parts(&selector.ty).is_none()
        && !matches!(selector.ty, HardwareType::Enum(_, _))
    {
        return Err(SemanticError::new(
            SemanticErrorKind::InvalidCaseSelector,
            "case selector must be bit or vector",
            selector.span,
        ));
    }
    let hint = arms
        .iter()
        .find_map(|arm| type_hint(&arm.value.result, context))
        .or_else(|| type_hint(else_expr, context))
        .or_else(|| expected.cloned());
    let mut labels = HashMap::new();
    let mut typed_arms = Vec::new();
    let mut result_type = hint;
    for arm in arms {
        let key = case_key(&arm.value.label, &selector.ty, context)?;
        if let Some(previous) = labels.insert(key.value, key.span) {
            return Err(SemanticError::new(
                SemanticErrorKind::DuplicateCaseLabel,
                "duplicate case label",
                key.span,
            )
            .related(previous));
        }
        let result =
            check_expr(&arm.value.result, result_type.as_ref(), context).map_err(|mut e| {
                if e.kind == SemanticErrorKind::TypeMismatch {
                    e.kind = SemanticErrorKind::CaseBranchTypeMismatch;
                    e.message = "case result branches must have identical types".into();
                }
                e
            })?;
        result_type.get_or_insert_with(|| result.ty.clone());
        typed_arms.push(TypedCaseExprArm {
            key,
            result,
            span: arm.span,
        });
    }
    let result_type = result_type.ok_or_else(|| {
        SemanticError::new(
            SemanticErrorKind::CannotInferType,
            "cannot infer case result type",
            span,
        )
    })?;
    let else_expr = check_expr(else_expr, Some(&result_type), context).map_err(|mut e| {
        if e.kind == SemanticErrorKind::TypeMismatch {
            e.kind = SemanticErrorKind::CaseBranchTypeMismatch;
            e.message = "case result branches must have identical types".into();
        }
        e
    })?;
    Ok(TypedExpr {
        kind: TypedExprKind::Case {
            selector: Box::new(selector),
            arms: typed_arms,
            else_expr: Box::new(else_expr),
        },
        ty: result_type,
        span,
    })
}

fn check_call(
    name: &str,
    args: &[Spanned<Expr>],
    expected: Option<&HardwareType>,
    context: &ModuleContext,
    span: Span,
) -> Result<TypedExpr, SemanticError> {
    match name {
        "rom-read" => {
            if args.len() != 2 {
                return Err(SemanticError::new(
                    SemanticErrorKind::WrongArgumentCount,
                    "rom-read expects ROM name and address",
                    span,
                ));
            }
            let Expr::Reference(identifier) = &args[0].value else {
                return Err(SemanticError::new(
                    SemanticErrorKind::UnknownRom,
                    "rom-read requires a ROM name",
                    args[0].span,
                ));
            };
            let rom = context
                .roms
                .iter()
                .find(|rom| rom.name == identifier.name)
                .ok_or_else(|| {
                    SemanticError::new(
                        SemanticErrorKind::UnknownRom,
                        "unknown ROM",
                        identifier.span,
                    )
                })?;
            let address_type = HardwareType::Unsigned(rom.address_width);
            let address = check_expr(&args[1], Some(&address_type), context).map_err(|_| {
                SemanticError::new(
                    SemanticErrorKind::InvalidRomRead,
                    "ROM address must be an exact-width unsigned vector or in-range integer",
                    args[1].span,
                )
            })?;
            if address.ty != address_type {
                return Err(SemanticError::new(
                    SemanticErrorKind::InvalidRomRead,
                    "ROM address width is inconsistent",
                    args[1].span,
                ));
            }
            Ok(TypedExpr {
                kind: TypedExprKind::RomRead {
                    rom_id: rom.id,
                    address: Box::new(address),
                    address_width: rom.address_width,
                    data_width: rom.data_width,
                },
                ty: HardwareType::Unsigned(rom.data_width),
                span,
            })
        }
        "register-array-read" => {
            if args.len() != 2 {
                return Err(SemanticError::new(
                    SemanticErrorKind::WrongArgumentCount,
                    "register-array-read expects array name and address",
                    span,
                ));
            }
            let Expr::Reference(identifier) = &args[0].value else {
                return Err(SemanticError::new(
                    SemanticErrorKind::UnknownRegisterArray,
                    "register-array-read requires an array name",
                    args[0].span,
                ));
            };
            let array = context
                .register_arrays
                .iter()
                .find(|array| array.name == identifier.name)
                .ok_or_else(|| {
                    SemanticError::new(
                        SemanticErrorKind::UnknownRegisterArray,
                        "unknown register-array",
                        identifier.span,
                    )
                })?;
            let address_type = HardwareType::Unsigned(array.address_width);
            let address = check_expr(&args[1], Some(&address_type), context).map_err(|_| {
                SemanticError::new(
                    SemanticErrorKind::InvalidRegisterArrayRead,
                    "register-array address must be exact-width unsigned",
                    args[1].span,
                )
            })?;
            if address.ty != address_type {
                return Err(SemanticError::new(
                    SemanticErrorKind::InvalidRegisterArrayRead,
                    "register-array address width is inconsistent",
                    args[1].span,
                ));
            }
            Ok(TypedExpr {
                kind: TypedExprKind::RegisterArrayRead {
                    array_id: array.id,
                    address: Box::new(address),
                    address_width: array.address_width,
                    data_width: array.data_width,
                },
                ty: HardwareType::Unsigned(array.data_width),
                span,
            })
        }
        "enum-from-bits" => {
            require_arity(name, args, 2, span)?;
            let Expr::Reference(identifier) = &args[0].value else {
                return Err(SemanticError::new(
                    SemanticErrorKind::InvalidEnumConversion,
                    "enum-from-bits requires an enum type name",
                    args[0].span,
                ));
            };
            let enumeration = context
                .enums
                .iter()
                .find(|item| item.name == identifier.name)
                .ok_or_else(|| {
                    SemanticError::new(
                        SemanticErrorKind::UnknownEnum,
                        "unknown enum type",
                        identifier.span,
                    )
                })?;
            let value = check_expr(&args[1], None, context)?;
            if vector_parts(&value.ty)
                != Some((false, WidthExpr::Constant(u64::from(enumeration.width))))
            {
                return Err(SemanticError::new(
                    SemanticErrorKind::InvalidEnumConversion,
                    "enum-from-bits requires an exact-width unsigned vector",
                    args[1].span,
                ));
            }
            Ok(TypedExpr {
                kind: TypedExprKind::EnumFromBits {
                    enum_id: enumeration.id,
                    value: Box::new(value),
                },
                ty: HardwareType::Enum(enumeration.id, enumeration.width),
                span,
            })
        }
        "enum-to-bits" => {
            require_arity(name, args, 1, span)?;
            let value = check_expr(&args[0], None, context)?;
            let HardwareType::Enum(enum_id, width) = value.ty else {
                return Err(SemanticError::new(
                    SemanticErrorKind::InvalidEnumConversion,
                    "enum-to-bits requires an enum value",
                    args[0].span,
                ));
            };
            Ok(TypedExpr {
                kind: TypedExprKind::EnumToBits {
                    enum_id,
                    value: Box::new(value),
                },
                ty: HardwareType::Unsigned(width),
                span,
            })
        }
        "reverse-bits" => {
            require_arity(name, args, 1, span)?;
            let value = check_expr(&args[0], None, context)?;
            let (_, width) = vector_parts(&value.ty).ok_or_else(|| {
                SemanticError::new(
                    SemanticErrorKind::InvalidReverseBitsSource,
                    "reverse-bits requires an unsigned or signed vector",
                    args[0].span,
                )
            })?;
            let ty = unsigned_width_type(width).ok_or_else(|| {
                SemanticError::new(
                    SemanticErrorKind::InvalidReverseBitsSource,
                    "reverse-bits width is unsupported",
                    span,
                )
            })?;
            Ok(TypedExpr {
                kind: TypedExprKind::ReverseBits {
                    value: Box::new(value),
                },
                ty,
                span,
            })
        }
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
            if matches!(operand.ty, HardwareType::Enum(_, _)) {
                return Err(SemanticError::new(
                    SemanticErrorKind::EnumTypeMismatch,
                    "bitwise not does not accept enum values",
                    span,
                ));
            }
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
            if matches!(ty, HardwareType::Enum(_, _))
                && matches!(op, BinaryOp::And | BinaryOp::Or | BinaryOp::Xor)
            {
                return Err(SemanticError::new(
                    SemanticErrorKind::EnumTypeMismatch,
                    "bitwise operators do not accept enum values",
                    span,
                ));
            }
            if matches!(op, BinaryOp::Add | BinaryOp::Subtract)
                && (ty == HardwareType::Bit || matches!(ty, HardwareType::Enum(_, _)))
            {
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
            if matches!(operand_ty, HardwareType::Enum(_, _))
                && matches!(
                    op,
                    BinaryOp::LessThan
                        | BinaryOp::LessEqual
                        | BinaryOp::GreaterThan
                        | BinaryOp::GreaterEqual
                )
            {
                return Err(SemanticError::new(
                    SemanticErrorKind::EnumTypeMismatch,
                    "enum ordering comparisons are not supported",
                    span,
                ));
            }
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
    let target_type = type_from_ast(&target.value, &context.generics, &context.enums)?;
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
        HardwareType::Enum(_, _) => None,
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
    let mut unmatched_small = Vec::new();
    for term in small_terms {
        if let Some(index) = big_terms.iter().position(|candidate| **candidate == *term) {
            big_terms.remove(index);
        } else {
            unmatched_small.push(term);
        }
    }
    if unmatched_small.is_empty() {
        return Ok(true);
    }
    let constant_sum = |terms: &[&WidthExpr]| {
        terms.iter().try_fold(0_u64, |sum, term| match term {
            WidthExpr::Constant(value) => sum.checked_add(*value),
            _ => None,
        })
    };
    Ok(matches!(
        (constant_sum(&big_terms), constant_sum(&unmatched_small)),
        (Some(big), Some(small)) if big >= small
    ))
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
            type_from_ast(&target.value, &context.generics, &context.enums).ok()
        }
        Expr::Slice { width, .. } => resolve_width(width, &context.generics)
            .ok()
            .and_then(unsigned_width_type),
        Expr::Concat { values } => concat_hint(values, context),
        Expr::StaticBitMotion { value, .. } => type_hint(value, context),
        Expr::BitAt { .. } => Some(HardwareType::Bit),
        Expr::Case {
            arms, else_expr, ..
        } => arms
            .iter()
            .find_map(|arm| type_hint(&arm.value.result, context))
            .or_else(|| type_hint(else_expr, context)),
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
            "reverse-bits" => arguments
                .first()
                .and_then(|arg| type_hint(arg, context))
                .and_then(|ty| operand_width(&ty))
                .and_then(unsigned_width_type),
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
        HardwareType::Enum(_, _) => false,
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
fn type_from_ast(
    ty: &TypeExpr,
    generics: &[TypedGeneric],
    enums: &[TypedEnum],
) -> Result<HardwareType, SemanticError> {
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
        TypeExpr::Enum(name) => enums
            .iter()
            .find(|item| item.name == name.name)
            .map(|item| HardwareType::Enum(item.id, item.width))
            .ok_or_else(|| {
                SemanticError::new(
                    SemanticErrorKind::UnknownEnum,
                    format!("unknown enum `{}`", name.name),
                    name.span,
                )
            }),
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
        ConstExprAst::Subtract(a, b) => Ok(WidthExpr::Subtract(
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
            WidthExpr::Subtract(a, b) => normalize_width(WidthExpr::Subtract(
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
        HardwareType::Enum(id, width) => Ok(HardwareType::Enum(*id, *width)),
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
                (WidthExpr::Subtract(left, right), _) if **right == b => Ok((**left).clone()),
                (_, WidthExpr::Subtract(left, right)) if **right == a => Ok((**left).clone()),
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
        WidthExpr::Subtract(a, b) => {
            let a = normalize_width(*a)?;
            let b = normalize_width(*b)?;
            match (&a, &b) {
                _ if a == b => Ok(WidthExpr::Constant(0)),
                (_, WidthExpr::Constant(0)) => Ok(a),
                (WidthExpr::Constant(x), WidthExpr::Constant(y)) => {
                    x.checked_sub(*y).map(WidthExpr::Constant).ok_or_else(|| {
                        SemanticError::new(
                            SemanticErrorKind::InvalidWidthExpression,
                            "constant subtraction would be negative",
                            empty_span(),
                        )
                    })
                }
                _ => Ok(WidthExpr::Subtract(Box::new(a), Box::new(b))),
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
        WidthExpr::Subtract(a, b) => {
            let left = minimum_with_generics(a, generics)?;
            let WidthExpr::Constant(right) = &**b else {
                return Err(SemanticError::new(
                    SemanticErrorKind::InvalidWidthExpression,
                    "cannot prove symbolic subtraction is non-negative",
                    empty_span(),
                ));
            };
            left.checked_sub(*right).ok_or_else(|| {
                SemanticError::new(
                    SemanticErrorKind::InvalidWidthExpression,
                    "width subtraction can be negative",
                    empty_span(),
                )
            })
        }
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
