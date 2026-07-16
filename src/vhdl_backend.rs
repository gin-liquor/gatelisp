use crate::{
    BinaryOp, ClockEdge, ConversionKind, GenericKind, HardwareType, ModuleId, PortDirection,
    ResetKind, SignalId, SignalKind, StaticBitMotionKind, TimeUnit, TypedClockedBlock, TypedExpr,
    TypedExprKind, TypedModule, TypedNext, TypedProgram, TypedTestbench, TypedTestbenchStmt,
    UnaryOp, VhdlArchitecture, VhdlConcurrentStatement, VhdlDeclaration, VhdlDesign,
    VhdlDesignUnit, VhdlEntity, VhdlExpression, VhdlGeneric, VhdlGenericKind, VhdlIdentifier,
    VhdlPort, VhdlPortMode, VhdlProcess, VhdlSensitivity, VhdlSequentialStatement, VhdlType,
    VhdlVariable, WidthExpr,
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
    generics: Vec<crate::GenericId>,
    generic_minimums: HashMap<crate::GenericId, u64>,
    enums: Vec<crate::TypedEnum>,
    roms: Vec<crate::TypedRom>,
    register_arrays: Vec<crate::TypedRegisterArray>,
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
    let mut enum_ids = std::collections::HashSet::new();
    for enumeration in &module.enums {
        if !enum_ids.insert(enumeration.id) {
            return Err(VhdlBackendError::new(
                VhdlBackendErrorKind::InconsistentId,
                "duplicate EnumId in module enum table",
            ));
        }
        validate_enum_definition(enumeration)?;
    }
    for enumeration in &module.enums {
        let used_members = module_enum_members(module, enumeration.id);
        if !used_members.is_empty() {
            for member in &enumeration.members {
                if used_members.contains(&member.id) {
                    declarations.push(VhdlDeclaration::EnumConstant {
                        name: enum_member_name(enumeration, member),
                        width: enumeration.width,
                        value: member.value,
                    });
                }
            }
        }
    }
    for rom in &module.roms {
        if module_uses_rom(module, rom.id) {
            declarations.push(VhdlDeclaration::Rom {
                name: rom_name(rom),
                address_width: rom.address_width,
                data_width: rom.data_width,
                depth: rom.depth,
                default_value: rom.default_value,
                entries: rom
                    .entries
                    .iter()
                    .map(|entry| (entry.address, entry.value))
                    .collect(),
            });
        }
    }
    for array in &module.register_arrays {
        declarations.push(VhdlDeclaration::RegisterArray {
            name: register_array_name(array),
            address_width: array.address_width,
            data_width: array.data_width,
            depth: array.depth,
            initial_value: array.initial_value,
        });
    }
    let mut names = HashMap::new();
    for signal in &module.signals {
        validate_hardware_type(&signal.ty, &module.enums)?;
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
                        TypedExprKind::EnumValue {
                            enum_id,
                            member_id,
                            value: encoded,
                        } => lower_enum_constant(
                            &module.enums,
                            enum_id,
                            member_id,
                            encoded,
                            &value.ty,
                        ),
                        _ => Err(VhdlBackendError::new(
                            VhdlBackendErrorKind::UnsupportedRegisterInitialValue,
                            "register initial value must be an integer or enum member literal",
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
    if module_needs_reverse_bits(module) {
        declarations.push(VhdlDeclaration::ReverseBitsFunction);
    }
    if module_needs_bit_at(module) {
        declarations.push(VhdlDeclaration::BitAtFunction);
    }
    let mut statements = Vec::new();
    for (index, assignment) in module.assignments.iter().enumerate() {
        let target = signal_name(&names, assignment.target)?.read.clone();
        let mut context = LowerContext {
            names: names.clone(),
            generics: module.generics.iter().map(|generic| generic.id).collect(),
            generic_minimums: module
                .generics
                .iter()
                .map(|generic| {
                    (
                        generic.id,
                        if generic.kind == GenericKind::Positive {
                            1
                        } else {
                            0
                        },
                    )
                })
                .collect(),
            enums: module.enums.clone(),
            roms: module.roms.clone(),
            register_arrays: module.register_arrays.clone(),
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
        let clock = module
            .signals
            .iter()
            .find(|signal| signal.id == block.clock)
            .ok_or_else(|| {
                VhdlBackendError::new(
                    VhdlBackendErrorKind::InconsistentId,
                    "clocked block references an unknown clock SignalId",
                )
            })?;
        if clock.ty != HardwareType::Bit {
            return Err(VhdlBackendError::new(
                VhdlBackendErrorKind::InvalidTypedExpression,
                "clocked block clock must have type bit",
            ));
        }
        statements.push(VhdlConcurrentStatement::Process(lower_clocked(
            block,
            &names,
            &module
                .generics
                .iter()
                .map(|generic| generic.id)
                .collect::<Vec<_>>(),
            &module
                .generics
                .iter()
                .map(|generic| {
                    (
                        generic.id,
                        if generic.kind == GenericKind::Positive {
                            1
                        } else {
                            0
                        },
                    )
                })
                .collect(),
            &module.enums,
            &module.roms,
            &module.register_arrays,
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
    generics: &[crate::GenericId],
    generic_minimums: &HashMap<crate::GenericId, u64>,
    enums: &[crate::TypedEnum],
    roms: &[crate::TypedRom],
    register_arrays: &[crate::TypedRegisterArray],
) -> Result<VhdlProcess, VhdlBackendError> {
    let clock = signal_name(names, block.clock)?.read.clone();
    let mut context = LowerContext {
        names: names.clone(),
        generics: generics.to_vec(),
        generic_minimums: generic_minimums.clone(),
        enums: enums.to_vec(),
        roms: roms.to_vec(),
        register_arrays: register_arrays.to_vec(),
        temporary: 0,
        variables: Vec::new(),
    };
    let mut normal = lower_updates(&block.updates, &mut context)?;
    normal.extend(lower_array_writes(&block.writes, &mut context)?);
    for case_do in &block.case_dos {
        normal.extend(lower_case_do(case_do, &mut context)?);
    }
    let edge = VhdlExpression::Call {
        function: VhdlIdentifier(match block.edge {
            ClockEdge::Rising => "rising_edge".into(),
            ClockEdge::Falling => "falling_edge".into(),
        }),
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
                    condition: edge,
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
                        condition: edge,
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
                condition: edge,
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

fn lower_array_writes(
    writes: &[crate::TypedRegisterArrayWrite],
    context: &mut LowerContext,
) -> Result<Vec<VhdlSequentialStatement>, VhdlBackendError> {
    let mut statements = Vec::new();
    for write in writes {
        let array = context
            .register_arrays
            .iter()
            .find(|array| array.id == write.array_id)
            .cloned()
            .ok_or_else(|| {
                VhdlBackendError::new(
                    VhdlBackendErrorKind::InconsistentId,
                    "unknown register-array id",
                )
            })?;
        let address = lower_expr(&write.address, context, &mut statements)?;
        let value = lower_expr(&write.value, context, &mut statements)?;
        statements.push(VhdlSequentialStatement::IndexedSignalAssignment {
            array: register_array_name(&array),
            index: address,
            value,
        });
    }
    Ok(statements)
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

fn lower_case_do(
    case_do: &crate::TypedCaseDo,
    context: &mut LowerContext,
) -> Result<Vec<VhdlSequentialStatement>, VhdlBackendError> {
    if case_do.arms.is_empty() {
        return Err(VhdlBackendError::new(
            VhdlBackendErrorKind::InvalidTypedExpression,
            "typed case-do requires an arm",
        ));
    }
    validate_case_selector(&case_do.selector.ty, context)?;
    let mut prelude = Vec::new();
    let selector = lower_expr(&case_do.selector, context, &mut prelude)?;
    let selector_name = VhdlIdentifier(format!("gl_tmp_{}", context.temporary));
    context.temporary = context.temporary.checked_add(1).ok_or_else(|| {
        VhdlBackendError::new(
            VhdlBackendErrorKind::InvalidIdentifier,
            "too many temporary variables",
        )
    })?;
    context.variables.push(VhdlVariable {
        name: selector_name.clone(),
        ty: lower_type(&case_do.selector.ty)?,
    });
    prelude.push(VhdlSequentialStatement::VariableAssignment {
        target: selector_name.clone(),
        value: selector,
    });
    let mut otherwise = case_do
        .else_body
        .as_ref()
        .map(|body| lower_updates(body, context))
        .transpose()?
        .unwrap_or_default();
    otherwise.extend(lower_array_writes(&case_do.else_writes, context)?);
    let mut labels = std::collections::HashSet::new();
    for arm in case_do.arms.iter().rev() {
        validate_case_key(
            &arm.key,
            &case_do.selector.ty,
            &context.generic_minimums,
            &context.enums,
            &mut labels,
        )?;
        let mut body = lower_updates(&arm.body, context)?;
        body.extend(lower_array_writes(&arm.writes, context)?);
        otherwise = vec![VhdlSequentialStatement::If {
            condition: VhdlExpression::Binary {
                op: "=".into(),
                left: Box::new(VhdlExpression::Name(selector_name.clone())),
                right: Box::new(lower_case_key(&arm.key, context)?),
            },
            then_statements: body,
            else_statements: otherwise,
        }];
    }
    prelude.extend(otherwise);
    Ok(prelude)
}

fn validate_case_key(
    key: &crate::CaseKey,
    selector_type: &HardwareType,
    generic_minimums: &HashMap<crate::GenericId, u64>,
    enums: &[crate::TypedEnum],
    labels: &mut std::collections::HashSet<i64>,
) -> Result<(), VhdlBackendError> {
    if &key.ty != selector_type
        || !case_value_fits(key.value, selector_type, generic_minimums)
        || !labels.insert(key.value)
    {
        return Err(VhdlBackendError::new(
            VhdlBackendErrorKind::InvalidTypedExpression,
            "case label type is inconsistent or duplicated",
        ));
    }
    if let HardwareType::Enum(enum_id, width) = selector_type {
        let Some(member_id) = key.enum_member else {
            return Err(VhdlBackendError::new(
                VhdlBackendErrorKind::InvalidTypedExpression,
                "enum case label is missing its member id",
            ));
        };
        let Some(enumeration) = enums.iter().find(|item| item.id == *enum_id) else {
            return Err(VhdlBackendError::new(
                VhdlBackendErrorKind::InconsistentId,
                "enum case selector references an unknown EnumId",
            ));
        };
        if enumeration.width != *width
            || !enumeration
                .members
                .iter()
                .any(|member| member.id == member_id && member.value == key.value as u64)
        {
            return Err(VhdlBackendError::new(
                VhdlBackendErrorKind::InvalidTypedExpression,
                "enum case label member is inconsistent",
            ));
        }
    } else if key.enum_member.is_some() {
        return Err(VhdlBackendError::new(
            VhdlBackendErrorKind::InvalidTypedExpression,
            "non-enum case label contains an enum member id",
        ));
    }
    if matches!(selector_type, HardwareType::Enum(_, _)) {
        return Ok(());
    }
    integer_literal(key.value, selector_type).map(|_| ())
}

fn lower_case_key(
    key: &crate::CaseKey,
    context: &LowerContext,
) -> Result<VhdlExpression, VhdlBackendError> {
    if let HardwareType::Enum(enum_id, width) = key.ty {
        let Some(member_id) = key.enum_member else {
            return Err(VhdlBackendError::new(
                VhdlBackendErrorKind::InvalidTypedExpression,
                "enum case label is missing its member id",
            ));
        };
        let Some(enumeration) = context.enums.iter().find(|item| item.id == enum_id) else {
            return Err(VhdlBackendError::new(
                VhdlBackendErrorKind::InconsistentId,
                "enum case label references an unknown EnumId",
            ));
        };
        let Some(member) = enumeration
            .members
            .iter()
            .find(|member| member.id == member_id && member.value == key.value as u64)
        else {
            return Err(VhdlBackendError::new(
                VhdlBackendErrorKind::InvalidTypedExpression,
                "enum case label member is inconsistent",
            ));
        };
        if enumeration.width != width {
            return Err(VhdlBackendError::new(
                VhdlBackendErrorKind::InvalidTypedExpression,
                "enum case label width is inconsistent",
            ));
        }
        return Ok(VhdlExpression::Name(enum_member_name(enumeration, member)));
    }
    integer_literal(key.value, &key.ty)
}

fn validate_case_selector(
    ty: &HardwareType,
    context: &LowerContext,
) -> Result<(), VhdlBackendError> {
    if let Some(width) = hardware_width(ty)
        && !width_generics_are_known(&width, &context.generics)
    {
        return Err(VhdlBackendError::new(
            VhdlBackendErrorKind::InconsistentId,
            "case selector type contains an unknown GenericId",
        ));
    }
    Ok(())
}

fn case_value_fits(
    value: i64,
    ty: &HardwareType,
    generic_minimums: &HashMap<crate::GenericId, u64>,
) -> bool {
    let width = match ty {
        HardwareType::Bit => return matches!(value, 0 | 1),
        HardwareType::Unsigned(width) | HardwareType::Signed(width) => u64::from(*width),
        HardwareType::SymbolicUnsigned(width) | HardwareType::SymbolicSigned(width) => {
            minimum_width_backend(width, generic_minimums)
        }
        HardwareType::Enum(_, width) => u64::from(*width),
    };
    match ty {
        HardwareType::Unsigned(_)
        | HardwareType::SymbolicUnsigned(_)
        | HardwareType::Enum(_, _) => {
            value >= 0 && (width >= 64 || (value as u64) < (1_u64 << width))
        }
        HardwareType::Signed(_) | HardwareType::SymbolicSigned(_) => {
            width >= 64
                || (width > 0 && {
                    let bound = 1_i64 << (width - 1);
                    value >= -bound && value < bound
                })
        }
        HardwareType::Bit => false,
    }
}

fn minimum_width_backend(
    width: &WidthExpr,
    generic_minimums: &HashMap<crate::GenericId, u64>,
) -> u64 {
    match width {
        WidthExpr::Constant(value) => *value,
        WidthExpr::Generic(id) => generic_minimums.get(id).copied().unwrap_or(0),
        WidthExpr::Add(left, right) => minimum_width_backend(left, generic_minimums)
            .saturating_add(minimum_width_backend(right, generic_minimums)),
        WidthExpr::Multiply(left, right) => minimum_width_backend(left, generic_minimums)
            .saturating_mul(minimum_width_backend(right, generic_minimums)),
        WidthExpr::Subtract(left, right) => minimum_width_backend(left, generic_minimums)
            .saturating_sub(minimum_width_backend(right, generic_minimums)),
    }
}

fn lower_testbench(
    testbench: &TypedTestbench,
    module: &TypedModule,
) -> Result<(VhdlEntity, VhdlArchitecture), VhdlBackendError> {
    let mut enum_ids = std::collections::HashSet::new();
    for enumeration in &module.enums {
        if !enum_ids.insert(enumeration.id) {
            return Err(VhdlBackendError::new(
                VhdlBackendErrorKind::InconsistentId,
                "duplicate EnumId in module enum table",
            ));
        }
        validate_enum_definition(enumeration)?;
    }
    let entity_name = VhdlIdentifier(format!(
        "gl_tb{}_{}",
        testbench.id.0,
        sanitize(&testbench.name)
    ));
    let mut declarations = vec![VhdlDeclaration::BoolToStdLogicFunction];
    for rom in &module.roms {
        if testbench_uses_rom(testbench, rom.id) {
            declarations.push(VhdlDeclaration::Rom {
                name: rom_name(rom),
                address_width: rom.address_width,
                data_width: rom.data_width,
                depth: rom.depth,
                default_value: rom.default_value,
                entries: rom
                    .entries
                    .iter()
                    .map(|entry| (entry.address, entry.value))
                    .collect(),
            });
        }
    }
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
    if testbench_needs_reverse_bits(testbench) {
        declarations.push(VhdlDeclaration::ReverseBitsFunction);
    }
    if testbench_needs_bit_at(testbench) {
        declarations.push(VhdlDeclaration::BitAtFunction);
    }
    for enumeration in &module.enums {
        let used_members = testbench_enum_members(testbench, enumeration.id);
        if !used_members.is_empty() {
            for member in &enumeration.members {
                if used_members.contains(&member.id) {
                    declarations.push(VhdlDeclaration::EnumConstant {
                        name: enum_member_name(enumeration, member),
                        width: enumeration.width,
                        value: member.value,
                    });
                }
            }
        }
    }
    let mut names = HashMap::new();
    let mut port_map = Vec::new();
    for signal in &module.signals {
        validate_hardware_type(&signal.ty, &module.enums)?;
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
        generics: Vec::new(),
        generic_minimums: HashMap::new(),
        enums: module.enums.clone(),
        roms: module.roms.clone(),
        register_arrays: module.register_arrays.clone(),
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
        TypedExprKind::ReverseBits { value } => {
            let source_width = hardware_width(&value.ty).ok_or_else(|| {
                VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "reverse-bits source is not a vector",
                )
            })?;
            if !matches!(
                expr.ty,
                HardwareType::Unsigned(_) | HardwareType::SymbolicUnsigned(_)
            ) || hardware_width(&expr.ty) != Some(source_width)
            {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "reverse-bits result type is inconsistent",
                ));
            }
            let mut source = lower_expr(value, context, prelude)?;
            if matches!(
                value.ty,
                HardwareType::Signed(_) | HardwareType::SymbolicSigned(_)
            ) {
                source = VhdlExpression::Call {
                    function: VhdlIdentifier("unsigned".into()),
                    arguments: vec![source],
                };
            }
            Ok(VhdlExpression::Call {
                function: VhdlIdentifier("gl_reverse_bits".into()),
                arguments: vec![source],
            })
        }
        TypedExprKind::StaticBitMotion {
            kind,
            value,
            amount,
        } => {
            if !width_generics_are_known(amount, &context.generics) {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InconsistentId,
                    "static shift or rotate amount contains an unknown GenericId",
                ));
            }
            let source_width = hardware_width(&value.ty).ok_or_else(|| {
                VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "static shift or rotate source is not a vector",
                )
            })?;
            if expr.ty != value.ty || hardware_width(&expr.ty) != Some(source_width.clone()) {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "static shift or rotate result type is inconsistent",
                ));
            }
            let signed = matches!(
                value.ty,
                HardwareType::Signed(_) | HardwareType::SymbolicSigned(_)
            );
            if (*kind == StaticBitMotionKind::ShiftRightLogical && signed)
                || (*kind == StaticBitMotionKind::ShiftRightArithmetic && !signed)
            {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "static shift source signedness is inconsistent",
                ));
            }
            let end = add_width_backend(amount.clone(), WidthExpr::Constant(1))?;
            if !prove_ge_backend(&source_width, &end) {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "static shift or rotate amount is not proven less than source width",
                ));
            }
            let function = match kind {
                StaticBitMotionKind::ShiftLeft => "shift_left",
                StaticBitMotionKind::ShiftRightLogical
                | StaticBitMotionKind::ShiftRightArithmetic => "shift_right",
                StaticBitMotionKind::RotateLeft => "rotate_left",
                StaticBitMotionKind::RotateRight => "rotate_right",
            };
            Ok(VhdlExpression::Call {
                function: VhdlIdentifier(function.into()),
                arguments: vec![lower_expr(value, context, prelude)?, lower_width(amount)?],
            })
        }
        TypedExprKind::BitAt { source, index } => {
            if expr.ty != HardwareType::Bit {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "bit-at result must have type bit",
                ));
            }
            let source_width = hardware_width(&source.ty).ok_or_else(|| {
                VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "bit-at source must be a vector",
                )
            })?;
            if !width_generics_are_known(index, &context.generics) {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InconsistentId,
                    "bit-at index contains an unknown GenericId",
                ));
            }
            let end = add_width_backend(index.clone(), WidthExpr::Constant(1))?;
            if !prove_ge_backend(&source_width, &end) {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "bit-at index is not proven less than source width",
                ));
            }
            let mut value = lower_expr(source, context, prelude)?;
            if matches!(
                source.ty,
                HardwareType::Signed(_) | HardwareType::SymbolicSigned(_)
            ) {
                value = VhdlExpression::Call {
                    function: VhdlIdentifier("unsigned".into()),
                    arguments: vec![value],
                };
            }
            Ok(VhdlExpression::Call {
                function: VhdlIdentifier("gl_bit_at".into()),
                arguments: vec![value, lower_width(index)?],
            })
        }
        TypedExprKind::EnumValue {
            enum_id,
            member_id,
            value,
        } => lower_enum_constant(&context.enums, *enum_id, *member_id, *value, &expr.ty),
        TypedExprKind::EnumFromBits { enum_id, value } => {
            let HardwareType::Enum(result_enum_id, width) = expr.ty else {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "enum-from-bits result is not the destination enum type",
                ));
            };
            if result_enum_id != *enum_id
                || !context
                    .enums
                    .iter()
                    .any(|item| item.id == *enum_id && item.width == width)
                || value.ty != HardwareType::Unsigned(width)
            {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "enum-from-bits source or result type is inconsistent",
                ));
            }
            lower_expr(value, context, prelude)
        }
        TypedExprKind::EnumToBits { enum_id, value } => {
            let HardwareType::Unsigned(width) = expr.ty else {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "enum-to-bits result is not an unsigned vector",
                ));
            };
            if value.ty != HardwareType::Enum(*enum_id, width)
                || !context
                    .enums
                    .iter()
                    .any(|item| item.id == *enum_id && item.width == width)
            {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "enum-to-bits source or result type is inconsistent",
                ));
            }
            lower_expr(value, context, prelude)
        }
        TypedExprKind::RomRead {
            rom_id,
            address,
            address_width,
            data_width,
        } => {
            let rom = context
                .roms
                .iter()
                .find(|rom| rom.id == *rom_id)
                .cloned()
                .ok_or_else(|| {
                    VhdlBackendError::new(
                        VhdlBackendErrorKind::InconsistentId,
                        "ROM read references an unknown RomId",
                    )
                })?;
            if rom.address_width != *address_width
                || rom.data_width != *data_width
                || expr.ty != HardwareType::Unsigned(*data_width)
                || address.ty != HardwareType::Unsigned(*address_width)
            {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "ROM read type information is inconsistent",
                ));
            }
            let address = lower_expr(address, context, prelude)?;
            Ok(VhdlExpression::Call {
                function: rom_name(&rom),
                arguments: vec![VhdlExpression::Call {
                    function: VhdlIdentifier("to_integer".into()),
                    arguments: vec![address],
                }],
            })
        }
        TypedExprKind::RegisterArrayRead {
            array_id,
            address,
            address_width,
            data_width,
        } => {
            let array = context
                .register_arrays
                .iter()
                .find(|array| array.id == *array_id)
                .ok_or_else(|| {
                    VhdlBackendError::new(
                        VhdlBackendErrorKind::InconsistentId,
                        "register-array read references an unknown RegisterArrayId",
                    )
                })?;
            if array.address_width != *address_width
                || array.data_width != *data_width
                || expr.ty != HardwareType::Unsigned(*data_width)
                || address.ty != HardwareType::Unsigned(*address_width)
            {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "register-array read type information is inconsistent",
                ));
            }
            Ok(VhdlExpression::Call {
                function: register_array_name(array),
                arguments: vec![VhdlExpression::Call {
                    function: VhdlIdentifier("to_integer".into()),
                    arguments: vec![lower_expr(address, context, prelude)?],
                }],
            })
        }
        TypedExprKind::Case {
            selector,
            arms,
            else_expr,
        } => {
            validate_case_selector(&selector.ty, context)?;
            if arms.is_empty() || else_expr.ty != expr.ty {
                return Err(VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidTypedExpression,
                    "typed case expression is incomplete or has inconsistent result type",
                ));
            }
            let selector_value = lower_expr(selector, context, prelude)?;
            let selector_name = VhdlIdentifier(format!("gl_tmp_{}", context.temporary));
            context.temporary = context.temporary.checked_add(1).ok_or_else(|| {
                VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidIdentifier,
                    "too many temporary variables",
                )
            })?;
            context.variables.push(VhdlVariable {
                name: selector_name.clone(),
                ty: lower_type(&selector.ty)?,
            });
            prelude.push(VhdlSequentialStatement::VariableAssignment {
                target: selector_name.clone(),
                value: selector_value,
            });
            let result_name = VhdlIdentifier(format!("gl_tmp_{}", context.temporary));
            context.temporary = context.temporary.checked_add(1).ok_or_else(|| {
                VhdlBackendError::new(
                    VhdlBackendErrorKind::InvalidIdentifier,
                    "too many temporary variables",
                )
            })?;
            context.variables.push(VhdlVariable {
                name: result_name.clone(),
                ty: lower_type(&expr.ty)?,
            });
            let mut else_statements = Vec::new();
            let default = lower_expr(else_expr, context, &mut else_statements)?;
            else_statements.push(VhdlSequentialStatement::VariableAssignment {
                target: result_name.clone(),
                value: default,
            });
            let mut labels = std::collections::HashSet::new();
            for arm in arms.iter().rev() {
                if arm.result.ty != expr.ty {
                    return Err(VhdlBackendError::new(
                        VhdlBackendErrorKind::InvalidTypedExpression,
                        "case result type is inconsistent",
                    ));
                }
                validate_case_key(
                    &arm.key,
                    &selector.ty,
                    &context.generic_minimums,
                    &context.enums,
                    &mut labels,
                )?;
                let mut then_statements = Vec::new();
                let result = lower_expr(&arm.result, context, &mut then_statements)?;
                then_statements.push(VhdlSequentialStatement::VariableAssignment {
                    target: result_name.clone(),
                    value: result,
                });
                else_statements = vec![VhdlSequentialStatement::If {
                    condition: VhdlExpression::Binary {
                        op: "=".into(),
                        left: Box::new(VhdlExpression::Name(selector_name.clone())),
                        right: Box::new(lower_case_key(&arm.key, context)?),
                    },
                    then_statements,
                    else_statements,
                }];
            }
            prelude.extend(else_statements);
            Ok(VhdlExpression::Name(result_name))
        }
    }
}

fn width_generics_are_known(width: &WidthExpr, known: &[crate::GenericId]) -> bool {
    match width {
        WidthExpr::Constant(_) => true,
        WidthExpr::Generic(id) => known.contains(id),
        WidthExpr::Add(left, right)
        | WidthExpr::Multiply(left, right)
        | WidthExpr::Subtract(left, right) => {
            width_generics_are_known(left, known) && width_generics_are_known(right, known)
        }
    }
}

fn prove_ge_backend(big: &WidthExpr, small: &WidthExpr) -> bool {
    if big == small {
        return true;
    }
    if let (WidthExpr::Constant(a), WidthExpr::Constant(b)) = (big, small) {
        return a >= b;
    }
    let mut big_terms = Vec::new();
    let mut small_terms = Vec::new();
    flatten_width_terms(big, &mut big_terms);
    flatten_width_terms(small, &mut small_terms);
    let mut unmatched_small = Vec::new();
    for term in small_terms {
        if let Some(index) = big_terms.iter().position(|candidate| *candidate == term) {
            big_terms.remove(index);
        } else {
            unmatched_small.push(term);
        }
    }
    if unmatched_small.is_empty() {
        return true;
    }
    let constant_sum = |terms: &[&WidthExpr]| {
        terms.iter().try_fold(0_u64, |sum, term| match term {
            WidthExpr::Constant(value) => sum.checked_add(*value),
            _ => None,
        })
    };
    matches!(
        (constant_sum(&big_terms), constant_sum(&unmatched_small)),
        (Some(big), Some(small)) if big >= small
    )
}

fn flatten_width_terms<'a>(width: &'a WidthExpr, terms: &mut Vec<&'a WidthExpr>) {
    if let WidthExpr::Add(left, right) = width {
        flatten_width_terms(left, terms);
        flatten_width_terms(right, terms);
    } else {
        terms.push(width);
    }
}

fn hardware_width(ty: &HardwareType) -> Option<WidthExpr> {
    match ty {
        HardwareType::Unsigned(v) | HardwareType::Signed(v) => {
            Some(WidthExpr::Constant(u64::from(*v)))
        }
        HardwareType::SymbolicUnsigned(v) | HardwareType::SymbolicSigned(v) => Some(v.clone()),
        HardwareType::Bit => None,
        HardwareType::Enum(_, width) => Some(WidthExpr::Constant(u64::from(*width))),
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
    if let WidthExpr::Subtract(value, subtracted) = &left
        && **subtracted == right
    {
        return Ok((**value).clone());
    }
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
        HardwareType::Enum(_, _) => None,
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
        TypedExprKind::ReverseBits { value } | TypedExprKind::StaticBitMotion { value, .. } => {
            expr_truncate_helpers(value, flags)
        }
        TypedExprKind::BitAt { source, .. } => expr_truncate_helpers(source, flags),
        TypedExprKind::EnumFromBits { value, .. } | TypedExprKind::EnumToBits { value, .. } => {
            expr_truncate_helpers(value, flags)
        }
        TypedExprKind::EnumValue { .. } => {}
        TypedExprKind::RomRead { address, .. } => expr_truncate_helpers(address, flags),
        TypedExprKind::RegisterArrayRead { address, .. } => expr_truncate_helpers(address, flags),
        TypedExprKind::Case {
            selector,
            arms,
            else_expr,
        } => {
            expr_truncate_helpers(selector, flags);
            for arm in arms {
                expr_truncate_helpers(&arm.result, flags);
            }
            expr_truncate_helpers(else_expr, flags);
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
        for case_do in &block.case_dos {
            expr_truncate_helpers(&case_do.selector, &mut flags);
            for arm in &case_do.arms {
                for update in &arm.body {
                    expr_truncate_helpers(&update.value, &mut flags);
                }
            }
            if let Some(body) = &case_do.else_body {
                for update in body {
                    expr_truncate_helpers(&update.value, &mut flags);
                }
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
        | TypedExprKind::Unary { operand: value, .. }
        | TypedExprKind::RegisterArrayRead { address: value, .. } => expr_has_bit_concat(value),
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
        TypedExprKind::ReverseBits { value } | TypedExprKind::StaticBitMotion { value, .. } => {
            expr_has_bit_concat(value)
        }
        TypedExprKind::BitAt { source, .. } => expr_has_bit_concat(source),
        TypedExprKind::EnumFromBits { value, .. } | TypedExprKind::EnumToBits { value, .. } => {
            expr_has_bit_concat(value)
        }
        TypedExprKind::EnumValue { .. } => false,
        TypedExprKind::RomRead { address, .. } => expr_has_bit_concat(address),
        TypedExprKind::Case {
            selector,
            arms,
            else_expr,
        } => {
            expr_has_bit_concat(selector)
                || arms.iter().any(|arm| expr_has_bit_concat(&arm.result))
                || expr_has_bit_concat(else_expr)
        }
    }
}
fn module_needs_bit_concat(module: &TypedModule) -> bool {
    module
        .assignments
        .iter()
        .any(|a| expr_has_bit_concat(&a.value))
        || module.clocked_blocks.iter().any(|b| {
            b.updates.iter().any(|u| expr_has_bit_concat(&u.value))
                || b.case_dos
                    .iter()
                    .any(|case_do| case_do_any(case_do, expr_has_bit_concat))
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
fn expr_has_reverse_bits(expr: &TypedExpr) -> bool {
    match &expr.kind {
        TypedExprKind::ReverseBits { .. } => true,
        TypedExprKind::Slice { value, .. }
        | TypedExprKind::Convert { value, .. }
        | TypedExprKind::Unary { operand: value, .. }
        | TypedExprKind::RegisterArrayRead { address: value, .. } => expr_has_reverse_bits(value),
        TypedExprKind::Binary { left, right, .. } => {
            expr_has_reverse_bits(left) || expr_has_reverse_bits(right)
        }
        TypedExprKind::If {
            condition,
            when_true,
            when_false,
        } => {
            expr_has_reverse_bits(condition)
                || expr_has_reverse_bits(when_true)
                || expr_has_reverse_bits(when_false)
        }
        TypedExprKind::Concat { values } => values.iter().any(expr_has_reverse_bits),
        TypedExprKind::StaticBitMotion { value, .. } => expr_has_reverse_bits(value),
        TypedExprKind::BitAt { source, .. } => expr_has_reverse_bits(source),
        TypedExprKind::EnumFromBits { value, .. } | TypedExprKind::EnumToBits { value, .. } => {
            expr_has_reverse_bits(value)
        }
        TypedExprKind::EnumValue { .. } => false,
        TypedExprKind::RomRead { address, .. } => expr_has_reverse_bits(address),
        TypedExprKind::Case {
            selector,
            arms,
            else_expr,
        } => {
            expr_has_reverse_bits(selector)
                || arms.iter().any(|arm| expr_has_reverse_bits(&arm.result))
                || expr_has_reverse_bits(else_expr)
        }
        TypedExprKind::Signal(_) | TypedExprKind::Integer(_) => false,
    }
}
fn module_needs_reverse_bits(module: &TypedModule) -> bool {
    module
        .assignments
        .iter()
        .any(|a| expr_has_reverse_bits(&a.value))
        || module.clocked_blocks.iter().any(|b| {
            b.updates.iter().any(|u| expr_has_reverse_bits(&u.value))
                || b.case_dos
                    .iter()
                    .any(|case_do| case_do_any(case_do, expr_has_reverse_bits))
                || b.reset
                    .as_ref()
                    .is_some_and(|r| r.updates.iter().any(|u| expr_has_reverse_bits(&u.value)))
        })
}
fn testbench_needs_reverse_bits(testbench: &TypedTestbench) -> bool {
    testbench.statements.iter().any(|s| match s {
        TypedTestbenchStmt::Drive { value, .. } => expr_has_reverse_bits(value),
        TypedTestbenchStmt::Assert { condition, .. } => expr_has_reverse_bits(condition),
        _ => false,
    })
}

fn expr_has_bit_at(expr: &TypedExpr) -> bool {
    match &expr.kind {
        TypedExprKind::BitAt { .. } => true,
        TypedExprKind::Slice { value, .. }
        | TypedExprKind::Convert { value, .. }
        | TypedExprKind::Unary { operand: value, .. }
        | TypedExprKind::ReverseBits { value }
        | TypedExprKind::StaticBitMotion { value, .. }
        | TypedExprKind::EnumFromBits { value, .. }
        | TypedExprKind::EnumToBits { value, .. } => expr_has_bit_at(value),
        TypedExprKind::Binary { left, right, .. } => {
            expr_has_bit_at(left) || expr_has_bit_at(right)
        }
        TypedExprKind::If {
            condition,
            when_true,
            when_false,
        } => {
            expr_has_bit_at(condition) || expr_has_bit_at(when_true) || expr_has_bit_at(when_false)
        }
        TypedExprKind::Concat { values } => values.iter().any(expr_has_bit_at),
        TypedExprKind::Signal(_) | TypedExprKind::Integer(_) | TypedExprKind::EnumValue { .. } => {
            false
        }
        TypedExprKind::RomRead { address, .. } => expr_has_bit_at(address),
        TypedExprKind::RegisterArrayRead { address, .. } => expr_has_bit_at(address),
        TypedExprKind::Case {
            selector,
            arms,
            else_expr,
        } => {
            expr_has_bit_at(selector)
                || arms.iter().any(|arm| expr_has_bit_at(&arm.result))
                || expr_has_bit_at(else_expr)
        }
    }
}

fn module_needs_bit_at(module: &TypedModule) -> bool {
    module.assignments.iter().any(|a| expr_has_bit_at(&a.value))
        || module.clocked_blocks.iter().any(|block| {
            block.updates.iter().any(|u| expr_has_bit_at(&u.value))
                || block
                    .case_dos
                    .iter()
                    .any(|case_do| case_do_any(case_do, expr_has_bit_at))
                || block
                    .reset
                    .as_ref()
                    .is_some_and(|reset| reset.updates.iter().any(|u| expr_has_bit_at(&u.value)))
        })
}

fn case_do_any<F>(case_do: &crate::TypedCaseDo, predicate: F) -> bool
where
    F: Fn(&TypedExpr) -> bool,
{
    predicate(&case_do.selector)
        || case_do
            .arms
            .iter()
            .any(|arm| arm.body.iter().any(|u| predicate(&u.value)))
        || case_do
            .else_body
            .as_ref()
            .is_some_and(|body| body.iter().any(|u| predicate(&u.value)))
}

fn testbench_needs_bit_at(testbench: &TypedTestbench) -> bool {
    testbench
        .statements
        .iter()
        .any(|statement| match statement {
            TypedTestbenchStmt::Drive { value, .. } => expr_has_bit_at(value),
            TypedTestbenchStmt::Assert { condition, .. } => expr_has_bit_at(condition),
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
        HardwareType::Enum(_, 0) => Err(VhdlBackendError::new(
            VhdlBackendErrorKind::InvalidTypeWidth,
            "VHDL enum width must be positive",
        )),
        HardwareType::Enum(_, width) => Ok(VhdlType::Unsigned(VhdlExpression::Literal(
            width.to_string(),
        ))),
    }
}

fn validate_enum_definition(enumeration: &crate::TypedEnum) -> Result<(), VhdlBackendError> {
    if enumeration.width == 0 {
        return Err(VhdlBackendError::new(
            VhdlBackendErrorKind::InvalidTypeWidth,
            "enum width must be positive",
        ));
    }
    let mut ids = std::collections::HashSet::new();
    let mut values = std::collections::HashSet::new();
    for member in &enumeration.members {
        if member.enum_id != enumeration.id
            || !ids.insert(member.id)
            || !values.insert(member.value)
            || (enumeration.width < 64 && member.value >= (1_u64 << enumeration.width))
        {
            return Err(VhdlBackendError::new(
                VhdlBackendErrorKind::InvalidTypedExpression,
                "enum declaration is inconsistent",
            ));
        }
    }
    if enumeration.members.is_empty() {
        return Err(VhdlBackendError::new(
            VhdlBackendErrorKind::InvalidTypedExpression,
            "enum declaration has no members",
        ));
    }
    Ok(())
}

fn validate_hardware_type(
    ty: &HardwareType,
    enums: &[crate::TypedEnum],
) -> Result<(), VhdlBackendError> {
    if let HardwareType::Enum(enum_id, width) = ty {
        let Some(enumeration) = enums.iter().find(|item| item.id == *enum_id) else {
            return Err(VhdlBackendError::new(
                VhdlBackendErrorKind::InconsistentId,
                "hardware type references an unknown EnumId",
            ));
        };
        if enumeration.width != *width {
            return Err(VhdlBackendError::new(
                VhdlBackendErrorKind::InvalidTypedExpression,
                "hardware enum type width is inconsistent",
            ));
        }
    }
    Ok(())
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
        HardwareType::Enum(_, _) => {
            return Err(VhdlBackendError::new(
                VhdlBackendErrorKind::InvalidTypedExpression,
                "integer literals cannot represent enum values",
            ));
        }
    };
    Ok(VhdlExpression::Literal(text))
}
fn generic_name(id: crate::GenericId) -> VhdlIdentifier {
    VhdlIdentifier(format!("gl_g{}", id.0))
}
fn enum_member_name(
    enumeration: &crate::TypedEnum,
    member: &crate::TypedEnumMember,
) -> VhdlIdentifier {
    VhdlIdentifier(format!(
        "gl_enum_{}_{}",
        sanitize(&enumeration.name),
        sanitize(&member.name)
    ))
}

fn rom_name(rom: &crate::TypedRom) -> VhdlIdentifier {
    VhdlIdentifier(format!("gl_rom_{}", sanitize(&rom.name)))
}

fn register_array_name(array: &crate::TypedRegisterArray) -> VhdlIdentifier {
    VhdlIdentifier(format!("gl_register_array_{}", sanitize(&array.name)))
}

fn lower_enum_constant(
    enums: &[crate::TypedEnum],
    enum_id: crate::EnumId,
    member_id: crate::EnumMemberId,
    value: u64,
    ty: &HardwareType,
) -> Result<VhdlExpression, VhdlBackendError> {
    let HardwareType::Enum(typed_enum_id, width) = ty else {
        return Err(VhdlBackendError::new(
            VhdlBackendErrorKind::InvalidTypedExpression,
            "enum member has a non-enum type",
        ));
    };
    if *typed_enum_id != enum_id {
        return Err(VhdlBackendError::new(
            VhdlBackendErrorKind::InvalidTypedExpression,
            "enum member type does not match its EnumId",
        ));
    }
    let Some(enumeration) = enums.iter().find(|item| item.id == enum_id) else {
        return Err(VhdlBackendError::new(
            VhdlBackendErrorKind::InconsistentId,
            "enum member references an unknown EnumId",
        ));
    };
    if enumeration.width != *width {
        return Err(VhdlBackendError::new(
            VhdlBackendErrorKind::InvalidTypedExpression,
            "enum member width is inconsistent",
        ));
    }
    let Some(member) = enumeration
        .members
        .iter()
        .find(|member| member.id == member_id && member.value == value)
    else {
        return Err(VhdlBackendError::new(
            VhdlBackendErrorKind::InvalidTypedExpression,
            "enum member id or value is inconsistent",
        ));
    };
    Ok(VhdlExpression::Name(enum_member_name(enumeration, member)))
}

fn module_enum_members(
    module: &TypedModule,
    id: crate::EnumId,
) -> std::collections::HashSet<crate::EnumMemberId> {
    let mut members = std::collections::HashSet::new();
    for signal in &module.signals {
        if let SignalKind::Register {
            initial: Some(initial),
        } = &signal.kind
        {
            collect_enum_members_expr(initial, id, &mut members);
        }
    }
    for assignment in &module.assignments {
        collect_enum_members_expr(&assignment.value, id, &mut members);
    }
    for block in &module.clocked_blocks {
        for update in &block.updates {
            collect_enum_members_expr(&update.value, id, &mut members);
        }
        if let Some(reset) = &block.reset {
            for update in &reset.updates {
                collect_enum_members_expr(&update.value, id, &mut members);
            }
        }
        for case_do in &block.case_dos {
            if let HardwareType::Enum(enum_id, _) = case_do.selector.ty
                && enum_id == id
            {
                for arm in &case_do.arms {
                    if let Some(member_id) = arm.key.enum_member {
                        members.insert(member_id);
                    }
                }
            }
            for arm in &case_do.arms {
                for update in &arm.body {
                    collect_enum_members_expr(&update.value, id, &mut members);
                }
            }
            if let Some(body) = &case_do.else_body {
                for update in body {
                    collect_enum_members_expr(&update.value, id, &mut members);
                }
            }
        }
    }
    members
}

fn testbench_enum_members(
    testbench: &TypedTestbench,
    id: crate::EnumId,
) -> std::collections::HashSet<crate::EnumMemberId> {
    let mut members = std::collections::HashSet::new();
    for statement in &testbench.statements {
        match statement {
            TypedTestbenchStmt::Drive { value, .. } => {
                collect_enum_members_expr(value, id, &mut members)
            }
            TypedTestbenchStmt::Assert { condition, .. } => {
                collect_enum_members_expr(condition, id, &mut members)
            }
            TypedTestbenchStmt::Wait { .. } | TypedTestbenchStmt::WaitRising { .. } => {}
        }
    }
    members
}

fn module_uses_rom(module: &TypedModule, id: crate::RomId) -> bool {
    module
        .assignments
        .iter()
        .any(|assignment| expr_uses_rom(&assignment.value, id))
        || module.clocked_blocks.iter().any(|block| {
            block
                .updates
                .iter()
                .any(|update| expr_uses_rom(&update.value, id))
                || block.reset.as_ref().is_some_and(|reset| {
                    reset
                        .updates
                        .iter()
                        .any(|update| expr_uses_rom(&update.value, id))
                })
                || block.case_dos.iter().any(|case_do| {
                    expr_uses_rom(&case_do.selector, id)
                        || case_do.arms.iter().any(|arm| {
                            arm.body
                                .iter()
                                .any(|update| expr_uses_rom(&update.value, id))
                        })
                        || case_do.else_body.as_ref().is_some_and(|body| {
                            body.iter().any(|update| expr_uses_rom(&update.value, id))
                        })
                })
        })
}

fn testbench_uses_rom(testbench: &TypedTestbench, id: crate::RomId) -> bool {
    testbench
        .statements
        .iter()
        .any(|statement| match statement {
            TypedTestbenchStmt::Drive { value, .. } => expr_uses_rom(value, id),
            TypedTestbenchStmt::Assert { condition, .. } => expr_uses_rom(condition, id),
            TypedTestbenchStmt::Wait { .. } | TypedTestbenchStmt::WaitRising { .. } => false,
        })
}

fn expr_uses_rom(expr: &TypedExpr, id: crate::RomId) -> bool {
    match &expr.kind {
        TypedExprKind::RomRead {
            rom_id, address, ..
        } => *rom_id == id || expr_uses_rom(address, id),
        TypedExprKind::RegisterArrayRead { address, .. } => expr_uses_rom(address, id),
        TypedExprKind::Unary { operand, .. } => expr_uses_rom(operand, id),
        TypedExprKind::Binary { left, right, .. } => {
            expr_uses_rom(left, id) || expr_uses_rom(right, id)
        }
        TypedExprKind::If {
            condition,
            when_true,
            when_false,
        } => {
            expr_uses_rom(condition, id)
                || expr_uses_rom(when_true, id)
                || expr_uses_rom(when_false, id)
        }
        TypedExprKind::Convert { value, .. }
        | TypedExprKind::Slice { value, .. }
        | TypedExprKind::ReverseBits { value }
        | TypedExprKind::StaticBitMotion { value, .. }
        | TypedExprKind::BitAt { source: value, .. } => expr_uses_rom(value, id),
        TypedExprKind::Concat { values } => values.iter().any(|value| expr_uses_rom(value, id)),
        TypedExprKind::Case {
            selector,
            arms,
            else_expr,
        } => {
            expr_uses_rom(selector, id)
                || arms.iter().any(|arm| expr_uses_rom(&arm.result, id))
                || expr_uses_rom(else_expr, id)
        }
        TypedExprKind::EnumFromBits { value, .. } | TypedExprKind::EnumToBits { value, .. } => {
            expr_uses_rom(value, id)
        }
        TypedExprKind::Signal(_) | TypedExprKind::Integer(_) | TypedExprKind::EnumValue { .. } => {
            false
        }
    }
}

fn collect_enum_members_expr(
    expr: &TypedExpr,
    id: crate::EnumId,
    members: &mut std::collections::HashSet<crate::EnumMemberId>,
) {
    match &expr.kind {
        TypedExprKind::EnumValue {
            enum_id, member_id, ..
        } => {
            if *enum_id == id {
                members.insert(*member_id);
            }
        }
        TypedExprKind::EnumFromBits { value, .. } | TypedExprKind::EnumToBits { value, .. } => {
            collect_enum_members_expr(value, id, members)
        }
        TypedExprKind::Unary { operand, .. } => collect_enum_members_expr(operand, id, members),
        TypedExprKind::Binary { left, right, .. } => {
            collect_enum_members_expr(left, id, members);
            collect_enum_members_expr(right, id, members);
        }
        TypedExprKind::If {
            condition,
            when_true,
            when_false,
        } => {
            collect_enum_members_expr(condition, id, members);
            collect_enum_members_expr(when_true, id, members);
            collect_enum_members_expr(when_false, id, members);
        }
        TypedExprKind::Convert { value, .. }
        | TypedExprKind::Slice { value, .. }
        | TypedExprKind::ReverseBits { value }
        | TypedExprKind::StaticBitMotion { value, .. }
        | TypedExprKind::BitAt { source: value, .. } => {
            collect_enum_members_expr(value, id, members)
        }
        TypedExprKind::Concat { values } => {
            for value in values {
                collect_enum_members_expr(value, id, members);
            }
        }
        TypedExprKind::Case {
            selector,
            arms,
            else_expr,
        } => {
            collect_enum_members_expr(selector, id, members);
            if let HardwareType::Enum(enum_id, _) = selector.ty
                && enum_id == id
            {
                for arm in arms {
                    if let Some(member_id) = arm.key.enum_member {
                        members.insert(member_id);
                    }
                }
            }
            for arm in arms {
                collect_enum_members_expr(&arm.result, id, members);
            }
            collect_enum_members_expr(else_expr, id, members);
        }
        TypedExprKind::Signal(_) | TypedExprKind::Integer(_) => {}
        TypedExprKind::RomRead { address, .. } => collect_enum_members_expr(address, id, members),
        TypedExprKind::RegisterArrayRead { address, .. } => {
            collect_enum_members_expr(address, id, members)
        }
    }
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
        WidthExpr::Subtract(a, b) => Ok(VhdlExpression::Binary {
            op: "-".into(),
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
            WidthExpr::Subtract(a, c) => Ok(WidthExpr::Subtract(
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
        HardwareType::Enum(id, width) => Ok(HardwareType::Enum(*id, *width)),
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
