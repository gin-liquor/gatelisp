use std::{
    collections::{HashMap, HashSet},
    fmt,
};

use crate::{
    BinaryOp, Expr, HardwareType, ModuleDecl, ModuleId, ModuleItem, PortDirection, Program,
    SignalId, SignalKind, Span, Spanned, TypeExpr, TypedAssign, TypedExpr, TypedExprKind,
    TypedModule, TypedProgram, TypedSignal, UnaryOp,
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
}

pub fn analyze_program(program: &Program) -> Result<TypedProgram, SemanticError> {
    let mut module_names = HashMap::<&str, Span>::new();
    let mut next_signal = 0_u32;
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
        modules.push(analyze_module(module, id, &mut next_signal)?);
    }
    Ok(TypedProgram { modules })
}

fn analyze_module(
    module: &Spanned<ModuleDecl>,
    id: ModuleId,
    next_id: &mut u32,
) -> Result<TypedModule, SemanticError> {
    let context = collect_signals(&module.value, next_id)?;
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
    Ok(TypedModule {
        id,
        name: module.value.name.name.clone(),
        name_span: module.value.name.span,
        signals,
        assignments,
        span: module.span,
    })
}

fn collect_signals(module: &ModuleDecl, next_id: &mut u32) -> Result<ModuleContext, SemanticError> {
    let mut context = ModuleContext {
        signals: Vec::new(),
        names: HashMap::new(),
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
            type_from_ast(&port.value.ty.value),
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
                type_from_ast(&wire.ty.value),
                item.span,
                None,
            )?,
            ModuleItem::Register(reg) => add_signal(
                &mut context,
                next_id,
                &reg.name.name,
                SignalClass::Register,
                type_from_ast(&reg.ty.value),
                item.span,
                reg.initial.clone(),
            )?,
            ModuleItem::Assign(_) => {}
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
            check_integer(*value, ty, expr.span)?;
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
            _ => None,
        },
    }
}

fn check_integer(value: i64, ty: &HardwareType, span: Span) -> Result<(), SemanticError> {
    let value = i128::from(value);
    let fits = match *ty {
        HardwareType::Bit => matches!(value, 0 | 1),
        HardwareType::Unsigned(width) if width >= 64 => value >= 0,
        HardwareType::Unsigned(width) => value >= 0 && value < (1_i128 << width),
        HardwareType::Signed(width) if width >= 64 => true,
        HardwareType::Signed(width) => {
            let bound = 1_i128 << (width - 1);
            value >= -bound && value < bound
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
fn type_from_ast(ty: &TypeExpr) -> HardwareType {
    match *ty {
        TypeExpr::Bit => HardwareType::Bit,
        TypeExpr::Unsigned(width) => HardwareType::Unsigned(width),
        TypeExpr::Signed(width) => HardwareType::Signed(width),
    }
}
