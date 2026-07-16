use std::fmt;

use crate::{
    AssignStmt, ClockedDecl, ConstExprAst, Expr, GenericBinding, GenericDecl, GenericKindSyntax,
    Identifier, InstanceDecl, ModuleDecl, ModuleItem, NextStmt, PortConnection, PortDecl,
    PortDirection, Program, RegisterDecl, ResetDecl, ResetKind, SExpr, Span, Spanned,
    StaticBitMotionSyntaxKind, TestbenchClockDecl, TestbenchDecl, TestbenchStmt, TimeLiteral,
    TimeUnit, TypeExpr, WireDecl,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateParseErrorKind {
    TopLevelNotModule,
    EmptyTopLevelList,
    MissingModuleName,
    InvalidModuleName,
    MissingPorts,
    InvalidPortsPosition,
    InvalidPort,
    InvalidPortName,
    InvalidPortDirection,
    InvalidType,
    ZeroWidth,
    NegativeWidth,
    WidthOutOfRange,
    InvalidWire,
    InvalidRegister,
    InvalidAssign,
    InvalidAssignTarget,
    UnknownModuleItem,
    EmptyExpression,
    InvalidCallTarget,
    InvalidExpression,
    InvalidClocked,
    InvalidClockName,
    EmptyClocked,
    UnknownClockedItem,
    MultipleResets,
    ResetAfterNext,
    InvalidResetKind,
    UnknownResetKind,
    InvalidResetSignal,
    EmptyReset,
    InvalidResetItem,
    InvalidNext,
    InvalidNextTarget,
    InvalidTestbench,
    InvalidTestbenchName,
    MissingTarget,
    DuplicateTarget,
    InvalidTargetPosition,
    InvalidTargetModule,
    InvalidTestbenchClock,
    InvalidTime,
    UnknownTimeUnit,
    MissingStimulus,
    DuplicateStimulus,
    InvalidStimulusPosition,
    UnknownTestbenchItem,
    InvalidDrive,
    InvalidDriveTarget,
    InvalidWait,
    InvalidWaitRising,
    InvalidAssert,
    InvalidAssertMessage,
    InvalidInstance,
    InvalidInstanceName,
    InvalidInstanceModule,
    InvalidInstancePorts,
    InvalidPortConnection,
    InvalidFormalPort,
    InvalidActualSignal,
    InvalidGenerics,
    InvalidGenericDeclaration,
    InvalidGenericName,
    InvalidGenericKind,
    InvalidGenericDefault,
    InvalidGenericBinding,
    InvalidGenericFormal,
    InvalidConstExpression,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateParseError {
    pub kind: GateParseErrorKind,
    pub message: String,
    pub span: Span,
}

impl GateParseError {
    fn new(kind: GateParseErrorKind, message: impl Into<String>, span: Span) -> Self {
        Self {
            kind,
            message: message.into(),
            span,
        }
    }
}

impl fmt::Display for GateParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for GateParseError {}

pub fn build_program(expressions: &[Spanned<SExpr>]) -> Result<Program, GateParseError> {
    let mut modules = Vec::new();
    let mut testbenches = Vec::new();
    for expression in expressions {
        match &expression.value {
            SExpr::List(list) if is_symbol(list.first(), "module") => {
                modules.push(parse_module(expression)?)
            }
            SExpr::List(list) if is_symbol(list.first(), "testbench") => {
                testbenches.push(parse_testbench(expression)?)
            }
            SExpr::List(list) if list.is_empty() => {
                return Err(error(
                    GateParseErrorKind::EmptyTopLevelList,
                    "empty list is not a top-level declaration",
                    expression,
                ));
            }
            _ => {
                return Err(error(
                    GateParseErrorKind::TopLevelNotModule,
                    "top level must contain module or testbench forms",
                    expression,
                ));
            }
        }
    }
    Ok(Program {
        modules,
        testbenches,
    })
}

fn parse_module(expression: &Spanned<SExpr>) -> Result<Spanned<ModuleDecl>, GateParseError> {
    let list = match &expression.value {
        SExpr::List(list) if list.is_empty() => {
            return Err(error(
                GateParseErrorKind::EmptyTopLevelList,
                "empty list is not a module",
                expression,
            ));
        }
        SExpr::List(list) => list,
        _ => {
            return Err(error(
                GateParseErrorKind::TopLevelNotModule,
                "top level must contain only module forms",
                expression,
            ));
        }
    };
    if !is_symbol(list.first(), "module") {
        return Err(error(
            GateParseErrorKind::TopLevelNotModule,
            "top-level form must start with module",
            expression,
        ));
    }
    let name_expr = list.get(1).ok_or_else(|| {
        error(
            GateParseErrorKind::MissingModuleName,
            "module name is required",
            expression,
        )
    })?;
    let name = identifier(
        name_expr,
        GateParseErrorKind::InvalidModuleName,
        "module name must be a symbol",
    )?;
    let mut index = 2;
    let generics = if let Some(value) = list.get(index) {
        if matches!(&value.value, SExpr::List(values) if is_symbol(values.first(), "generics")) {
            index += 1;
            parse_generics(value)?
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    let ports_expr = list.get(index).ok_or_else(|| {
        error(
            GateParseErrorKind::MissingPorts,
            "ports form is required after module name",
            expression,
        )
    })?;
    let ports = parse_ports(ports_expr)?;
    let mut items = Vec::new();
    for item in &list[index + 1..] {
        items.push(parse_item(item)?);
    }
    Ok(Spanned {
        value: ModuleDecl {
            name,
            generics,
            ports,
            items,
        },
        span: expression.span,
    })
}

fn parse_ports(expression: &Spanned<SExpr>) -> Result<Vec<Spanned<PortDecl>>, GateParseError> {
    let list = match &expression.value {
        SExpr::List(list) if is_symbol(list.first(), "ports") => list,
        _ => {
            return Err(error(
                GateParseErrorKind::InvalidPortsPosition,
                "ports form must immediately follow module name",
                expression,
            ));
        }
    };
    list[1..].iter().map(parse_port).collect()
}

fn parse_port(expression: &Spanned<SExpr>) -> Result<Spanned<PortDecl>, GateParseError> {
    let list = exact_list(
        expression,
        3,
        GateParseErrorKind::InvalidPort,
        "port declaration must have three elements",
    )?;
    let name = identifier(
        &list[0],
        GateParseErrorKind::InvalidPortName,
        "port name must be a symbol",
    )?;
    let direction = match &list[1].value {
        SExpr::Keyword(value) if value == "in" => PortDirection::Input,
        SExpr::Keyword(value) if value == "out" => PortDirection::Output,
        _ => {
            return Err(error(
                GateParseErrorKind::InvalidPortDirection,
                "port direction must be :in or :out",
                &list[1],
            ));
        }
    };
    Ok(Spanned {
        value: PortDecl {
            name,
            direction,
            ty: parse_type(&list[2])?,
        },
        span: expression.span,
    })
}

fn parse_type(expression: &Spanned<SExpr>) -> Result<Spanned<TypeExpr>, GateParseError> {
    let value = match &expression.value {
        SExpr::Symbol(name) if name == "bit" => TypeExpr::Bit,
        SExpr::List(list)
            if list.len() == 2
                && (is_symbol(list.first(), "unsigned") || is_symbol(list.first(), "signed")) =>
        {
            if let SExpr::Integer(value) = list[1].value {
                if value == 0 {
                    return Err(error(
                        GateParseErrorKind::ZeroWidth,
                        "type width must be greater than zero",
                        &list[1],
                    ));
                }
                if value < 0 {
                    return Err(error(
                        GateParseErrorKind::NegativeWidth,
                        "type width cannot be negative",
                        &list[1],
                    ));
                }
                let width = u32::try_from(value).map_err(|_| {
                    error(
                        GateParseErrorKind::WidthOutOfRange,
                        "type width exceeds u32",
                        &list[1],
                    )
                })?;
                if is_symbol(list.first(), "unsigned") {
                    TypeExpr::Unsigned(width)
                } else {
                    TypeExpr::Signed(width)
                }
            } else {
                let width = parse_width(&list[1])?;
                if is_symbol(list.first(), "unsigned") {
                    TypeExpr::SymbolicUnsigned(width)
                } else {
                    TypeExpr::SymbolicSigned(width)
                }
            }
        }
        _ => {
            return Err(error(
                GateParseErrorKind::InvalidType,
                "expected bit, (unsigned width), or (signed width)",
                expression,
            ));
        }
    };
    Ok(Spanned {
        value,
        span: expression.span,
    })
}

fn parse_width(expression: &Spanned<SExpr>) -> Result<Spanned<ConstExprAst>, GateParseError> {
    parse_const_expr(expression)
}

fn parse_generics(
    expression: &Spanned<SExpr>,
) -> Result<Vec<Spanned<GenericDecl>>, GateParseError> {
    let values = match &expression.value {
        SExpr::List(values) if is_symbol(values.first(), "generics") => values,
        _ => {
            return Err(error(
                GateParseErrorKind::InvalidGenerics,
                "invalid generics form",
                expression,
            ));
        }
    };
    values[1..]
        .iter()
        .map(|decl| {
            let parts = exact_list(
                decl,
                3,
                GateParseErrorKind::InvalidGenericDeclaration,
                "generic declaration must have three elements",
            )?;
            let name = identifier(
                &parts[0],
                GateParseErrorKind::InvalidGenericName,
                "generic name must be a symbol",
            )?;
            let kind = match &parts[1].value {
                SExpr::Keyword(v) if v == "natural" => GenericKindSyntax::Natural,
                SExpr::Keyword(v) if v == "positive" => GenericKindSyntax::Positive,
                _ => {
                    return Err(error(
                        GateParseErrorKind::InvalidGenericKind,
                        "generic kind must be :natural or :positive",
                        &parts[1],
                    ));
                }
            };
            let default = match parts[2].value {
                SExpr::Integer(v) if v >= 0 => u64::try_from(v).map_err(|_| {
                    error(
                        GateParseErrorKind::InvalidGenericDefault,
                        "generic default is out of range",
                        &parts[2],
                    )
                })?,
                _ => {
                    return Err(error(
                        GateParseErrorKind::InvalidGenericDefault,
                        "generic default must be a non-negative integer",
                        &parts[2],
                    ));
                }
            };
            Ok(Spanned {
                value: GenericDecl {
                    name,
                    kind,
                    default,
                },
                span: decl.span,
            })
        })
        .collect()
}

fn parse_const_expr(expression: &Spanned<SExpr>) -> Result<Spanned<ConstExprAst>, GateParseError> {
    let value = match &expression.value {
        SExpr::Integer(v) if *v >= 0 => ConstExprAst::Integer(u64::try_from(*v).map_err(|_| {
            error(
                GateParseErrorKind::InvalidConstExpression,
                "constant is out of range",
                expression,
            )
        })?),
        SExpr::Symbol(name) => ConstExprAst::Reference(Identifier {
            name: name.clone(),
            span: expression.span,
        }),
        SExpr::List(values)
            if values.len() == 3
                && (is_symbol(values.first(), "+") || is_symbol(values.first(), "*")) =>
        {
            let left = Box::new(parse_const_expr(&values[1])?);
            let right = Box::new(parse_const_expr(&values[2])?);
            if is_symbol(values.first(), "+") {
                ConstExprAst::Add(left, right)
            } else {
                ConstExprAst::Multiply(left, right)
            }
        }
        _ => {
            return Err(error(
                GateParseErrorKind::InvalidConstExpression,
                "expected non-negative integer, generic, +, or *",
                expression,
            ));
        }
    };
    Ok(Spanned {
        value,
        span: expression.span,
    })
}

fn parse_generic_bindings(
    expression: &Spanned<SExpr>,
) -> Result<Vec<Spanned<GenericBinding>>, GateParseError> {
    let values = match &expression.value {
        SExpr::List(values) if is_symbol(values.first(), "generics") => values,
        _ => {
            return Err(error(
                GateParseErrorKind::InvalidGenerics,
                "invalid generic bindings",
                expression,
            ));
        }
    };
    values[1..]
        .iter()
        .map(|binding| {
            let parts = exact_list(
                binding,
                2,
                GateParseErrorKind::InvalidGenericBinding,
                "generic binding must have two elements",
            )?;
            Ok(Spanned {
                value: GenericBinding {
                    formal: identifier(
                        &parts[0],
                        GateParseErrorKind::InvalidGenericFormal,
                        "generic formal must be a symbol",
                    )?,
                    value: parse_const_expr(&parts[1])?,
                },
                span: binding.span,
            })
        })
        .collect()
}

fn parse_item(expression: &Spanned<SExpr>) -> Result<Spanned<ModuleItem>, GateParseError> {
    let list = match &expression.value {
        SExpr::List(list) => list,
        _ => {
            return Err(error(
                GateParseErrorKind::UnknownModuleItem,
                "module item must be a list",
                expression,
            ));
        }
    };
    let head = match list.first().map(|value| &value.value) {
        Some(SExpr::Symbol(head)) => head.as_str(),
        _ => {
            return Err(error(
                GateParseErrorKind::UnknownModuleItem,
                "unknown module item",
                expression,
            ));
        }
    };
    let value = match head {
        "wire" => ModuleItem::Wire(parse_wire(expression)?),
        "reg" => ModuleItem::Register(parse_register(expression)?),
        "assign" => ModuleItem::Assign(parse_assign(expression)?),
        "clocked" => ModuleItem::Clocked(parse_clocked(expression)?),
        "instance" => ModuleItem::Instance(parse_instance(expression)?),
        "ports" => {
            return Err(error(
                GateParseErrorKind::InvalidPortsPosition,
                "ports form must appear exactly once after module name",
                expression,
            ));
        }
        _ => {
            return Err(error(
                GateParseErrorKind::UnknownModuleItem,
                "unknown module item",
                expression,
            ));
        }
    };
    Ok(Spanned {
        value,
        span: expression.span,
    })
}

fn parse_instance(expression: &Spanned<SExpr>) -> Result<InstanceDecl, GateParseError> {
    let list = match &expression.value {
        SExpr::List(values) if matches!(values.len(), 4 | 5) => values,
        _ => {
            return Err(error(
                GateParseErrorKind::InvalidInstance,
                "instance form must have four or five elements",
                expression,
            ));
        }
    };
    let name = identifier(
        &list[1],
        GateParseErrorKind::InvalidInstanceName,
        "instance name must be a symbol",
    )?;
    let module = identifier(
        &list[2],
        GateParseErrorKind::InvalidInstanceModule,
        "instance module must be a symbol",
    )?;
    let (generics_span, generics, ports_index) = if list.len() == 5 {
        (Some(list[3].span), parse_generic_bindings(&list[3])?, 4)
    } else {
        (None, Vec::new(), 3)
    };
    let ports = match &list[ports_index].value {
        SExpr::List(values) if is_symbol(values.first(), "ports") => values,
        _ => {
            return Err(error(
                GateParseErrorKind::InvalidInstancePorts,
                "instance requires one ports form",
                &list[ports_index],
            ));
        }
    };
    let connections = ports[1..]
        .iter()
        .map(|connection| {
            let values = exact_list(
                connection,
                2,
                GateParseErrorKind::InvalidPortConnection,
                "port connection must have two elements",
            )?;
            Ok(Spanned {
                value: PortConnection {
                    formal: identifier(
                        &values[0],
                        GateParseErrorKind::InvalidFormalPort,
                        "formal port must be a symbol",
                    )?,
                    actual: identifier(
                        &values[1],
                        GateParseErrorKind::InvalidActualSignal,
                        "actual connection must be a signal symbol",
                    )?,
                },
                span: connection.span,
            })
        })
        .collect::<Result<Vec<_>, GateParseError>>()?;
    Ok(InstanceDecl {
        name,
        module,
        generics_span,
        generics,
        ports_span: list[ports_index].span,
        ports: connections,
    })
}

fn parse_clocked(expression: &Spanned<SExpr>) -> Result<ClockedDecl, GateParseError> {
    let list = match &expression.value {
        SExpr::List(list) if list.len() < 2 => {
            return Err(error(
                GateParseErrorKind::InvalidClocked,
                "clocked form requires a clock signal",
                expression,
            ));
        }
        SExpr::List(list) => list,
        _ => {
            return Err(error(
                GateParseErrorKind::InvalidClocked,
                "invalid clocked form",
                expression,
            ));
        }
    };
    let clock = identifier(
        &list[1],
        GateParseErrorKind::InvalidClockName,
        "clock name must be a symbol",
    )?;
    if list.len() == 2 {
        return Err(error(
            GateParseErrorKind::EmptyClocked,
            "clocked body must contain at least one next",
            expression,
        ));
    }
    let mut reset = None;
    let mut updates = Vec::new();
    let mut saw_next = false;
    for item in &list[2..] {
        let item_list = match &item.value {
            SExpr::List(values) => values,
            _ => {
                return Err(error(
                    GateParseErrorKind::UnknownClockedItem,
                    "clocked body accepts only reset and next forms",
                    item,
                ));
            }
        };
        if is_symbol(item_list.first(), "reset") {
            if saw_next {
                return Err(error(
                    GateParseErrorKind::ResetAfterNext,
                    "reset must appear before normal next forms",
                    item,
                ));
            }
            if reset.is_some() {
                return Err(error(
                    GateParseErrorKind::MultipleResets,
                    "clocked block may contain only one reset",
                    item,
                ));
            }
            reset = Some(Spanned {
                value: parse_reset(item)?,
                span: item.span,
            });
        } else if is_symbol(item_list.first(), "next") {
            saw_next = true;
            updates.push(Spanned {
                value: parse_next(item)?,
                span: item.span,
            });
        } else {
            return Err(error(
                GateParseErrorKind::UnknownClockedItem,
                "unknown clocked item",
                item,
            ));
        }
    }
    if updates.is_empty() {
        return Err(error(
            GateParseErrorKind::EmptyClocked,
            "clocked block requires at least one normal next",
            expression,
        ));
    }
    Ok(ClockedDecl {
        clock,
        reset,
        updates,
    })
}

fn parse_reset(expression: &Spanned<SExpr>) -> Result<ResetDecl, GateParseError> {
    let list = match &expression.value {
        SExpr::List(list) => list,
        _ => {
            return Err(error(
                GateParseErrorKind::InvalidResetItem,
                "invalid reset form",
                expression,
            ));
        }
    };
    let kind_expr = list.get(1).ok_or_else(|| {
        error(
            GateParseErrorKind::InvalidResetKind,
            "reset kind is required",
            expression,
        )
    })?;
    let kind = match &kind_expr.value {
        SExpr::Keyword(value) if value == "sync" => ResetKind::Synchronous,
        SExpr::Keyword(value) if value == "async" => ResetKind::Asynchronous,
        SExpr::Keyword(_) => {
            return Err(error(
                GateParseErrorKind::UnknownResetKind,
                "reset kind must be :sync or :async",
                kind_expr,
            ));
        }
        _ => {
            return Err(error(
                GateParseErrorKind::InvalidResetKind,
                "reset kind must be a keyword",
                kind_expr,
            ));
        }
    };
    let signal_expr = list.get(2).ok_or_else(|| {
        error(
            GateParseErrorKind::InvalidResetSignal,
            "reset signal is required",
            expression,
        )
    })?;
    let signal = identifier(
        signal_expr,
        GateParseErrorKind::InvalidResetSignal,
        "reset signal must be a symbol",
    )?;
    if list.len() == 3 {
        return Err(error(
            GateParseErrorKind::EmptyReset,
            "reset body must contain at least one next",
            expression,
        ));
    }
    let mut updates = Vec::new();
    for item in &list[3..] {
        let values = match &item.value {
            SExpr::List(values) => values,
            _ => {
                return Err(error(
                    GateParseErrorKind::InvalidResetItem,
                    "reset body accepts only next forms",
                    item,
                ));
            }
        };
        if !is_symbol(values.first(), "next") {
            return Err(error(
                GateParseErrorKind::InvalidResetItem,
                "reset body accepts only next forms",
                item,
            ));
        }
        updates.push(Spanned {
            value: parse_next(item)?,
            span: item.span,
        });
    }
    Ok(ResetDecl {
        kind,
        signal,
        updates,
    })
}

fn parse_next(expression: &Spanned<SExpr>) -> Result<NextStmt, GateParseError> {
    let list = exact_list(
        expression,
        3,
        GateParseErrorKind::InvalidNext,
        "next form must have three elements",
    )?;
    let target = identifier(
        &list[1],
        GateParseErrorKind::InvalidNextTarget,
        "next target must be a symbol",
    )?;
    Ok(NextStmt {
        target,
        value: parse_expr(&list[2])?,
    })
}

fn parse_wire(expression: &Spanned<SExpr>) -> Result<WireDecl, GateParseError> {
    let list = exact_list(
        expression,
        3,
        GateParseErrorKind::InvalidWire,
        "wire form must have three elements",
    )?;
    Ok(WireDecl {
        name: identifier(
            &list[1],
            GateParseErrorKind::InvalidWire,
            "wire name must be a symbol",
        )?,
        ty: parse_type(&list[2])?,
    })
}

fn parse_register(expression: &Spanned<SExpr>) -> Result<RegisterDecl, GateParseError> {
    let list = match &expression.value {
        SExpr::List(list) if matches!(list.len(), 3 | 4) => list,
        _ => {
            return Err(error(
                GateParseErrorKind::InvalidRegister,
                "reg form must have three or four elements",
                expression,
            ));
        }
    };
    Ok(RegisterDecl {
        name: identifier(
            &list[1],
            GateParseErrorKind::InvalidRegister,
            "register name must be a symbol",
        )?,
        ty: parse_type(&list[2])?,
        initial: list.get(3).map(parse_expr).transpose()?,
    })
}

fn parse_assign(expression: &Spanned<SExpr>) -> Result<AssignStmt, GateParseError> {
    let list = exact_list(
        expression,
        3,
        GateParseErrorKind::InvalidAssign,
        "assign form must have three elements",
    )?;
    Ok(AssignStmt {
        target: identifier(
            &list[1],
            GateParseErrorKind::InvalidAssignTarget,
            "assign target must be a symbol",
        )?,
        value: parse_expr(&list[2])?,
    })
}

fn parse_expr(expression: &Spanned<SExpr>) -> Result<Spanned<Expr>, GateParseError> {
    let value = match &expression.value {
        SExpr::Symbol(name) => Expr::Reference(Identifier {
            name: name.clone(),
            span: expression.span,
        }),
        SExpr::Integer(value) => Expr::Integer(*value),
        SExpr::List(list) if list.is_empty() => {
            return Err(error(
                GateParseErrorKind::EmptyExpression,
                "empty list cannot be used as an expression",
                expression,
            ));
        }
        SExpr::List(list) => {
            let callee = identifier(
                &list[0],
                GateParseErrorKind::InvalidCallTarget,
                "call target must be a symbol",
            )?;
            if matches!(callee.name.as_str(), "resize" | "truncate") {
                if list.len() != 3 {
                    return Err(error(
                        GateParseErrorKind::InvalidExpression,
                        "resize and truncate require target type and value",
                        expression,
                    ));
                }
                let target = parse_type(&list[1])?;
                let value = Box::new(parse_expr(&list[2])?);
                return Ok(Spanned {
                    value: if callee.name == "resize" {
                        Expr::Resize { target, value }
                    } else {
                        Expr::Truncate { target, value }
                    },
                    span: expression.span,
                });
            }
            if callee.name == "slice" {
                if list.len() != 4 {
                    return Err(error(
                        GateParseErrorKind::InvalidExpression,
                        "slice requires source, offset, and width",
                        expression,
                    ));
                }
                return Ok(Spanned {
                    value: Expr::Slice {
                        value: Box::new(parse_expr(&list[1])?),
                        offset: parse_const_expr(&list[2])?,
                        width: parse_const_expr(&list[3])?,
                    },
                    span: expression.span,
                });
            }
            if callee.name == "concat" {
                if list.len() < 3 {
                    return Err(error(
                        GateParseErrorKind::InvalidExpression,
                        "concat requires at least two operands",
                        expression,
                    ));
                }
                return Ok(Spanned {
                    value: Expr::Concat {
                        values: list[1..].iter().map(parse_expr).collect::<Result<_, _>>()?,
                    },
                    span: expression.span,
                });
            }
            let motion = match callee.name.as_str() {
                "shift-left" => Some(StaticBitMotionSyntaxKind::ShiftLeft),
                "shift-right-logical" => Some(StaticBitMotionSyntaxKind::ShiftRightLogical),
                "shift-right-arithmetic" => Some(StaticBitMotionSyntaxKind::ShiftRightArithmetic),
                "rotate-left" => Some(StaticBitMotionSyntaxKind::RotateLeft),
                "rotate-right" => Some(StaticBitMotionSyntaxKind::RotateRight),
                _ => None,
            };
            if let Some(kind) = motion {
                if list.len() != 3 {
                    return Err(error(
                        GateParseErrorKind::InvalidExpression,
                        "static shift and rotate require source and amount",
                        expression,
                    ));
                }
                return Ok(Spanned {
                    value: Expr::StaticBitMotion {
                        kind,
                        value: Box::new(parse_expr(&list[1])?),
                        amount: parse_const_expr(&list[2])?,
                    },
                    span: expression.span,
                });
            }
            let arguments = list[1..].iter().map(parse_expr).collect::<Result<_, _>>()?;
            Expr::Call { callee, arguments }
        }
        SExpr::Keyword(_) | SExpr::String(_) => {
            return Err(error(
                GateParseErrorKind::InvalidExpression,
                "keyword or string cannot be used as an expression",
                expression,
            ));
        }
    };
    Ok(Spanned {
        value,
        span: expression.span,
    })
}

fn parse_testbench(expression: &Spanned<SExpr>) -> Result<Spanned<TestbenchDecl>, GateParseError> {
    let list = match &expression.value {
        SExpr::List(list) => list,
        _ => {
            return Err(error(
                GateParseErrorKind::InvalidTestbench,
                "invalid testbench",
                expression,
            ));
        }
    };
    let name_expr = list.get(1).ok_or_else(|| {
        error(
            GateParseErrorKind::InvalidTestbenchName,
            "testbench name is required",
            expression,
        )
    })?;
    let name = identifier(
        name_expr,
        GateParseErrorKind::InvalidTestbenchName,
        "testbench name must be a symbol",
    )?;
    let target_form = list.get(2).ok_or_else(|| {
        error(
            GateParseErrorKind::MissingTarget,
            "target form is required after testbench name",
            expression,
        )
    })?;
    let target_list = match &target_form.value {
        SExpr::List(values) if is_symbol(values.first(), "target") => values,
        _ => {
            return Err(error(
                GateParseErrorKind::InvalidTargetPosition,
                "target must immediately follow testbench name",
                target_form,
            ));
        }
    };
    if !matches!(target_list.len(), 2 | 3) {
        return Err(error(
            GateParseErrorKind::InvalidTargetModule,
            "target form must name one module",
            target_form,
        ));
    }
    let target = identifier(
        &target_list[1],
        GateParseErrorKind::InvalidTargetModule,
        "target module must be a symbol",
    )?;
    let target_generics = if target_list.len() == 3 {
        parse_generic_bindings(&target_list[2])?
    } else {
        Vec::new()
    };
    let mut clocks = Vec::new();
    let mut stimulus = None;
    for item in &list[3..] {
        let values = match &item.value {
            SExpr::List(values) => values,
            _ => {
                return Err(error(
                    GateParseErrorKind::UnknownTestbenchItem,
                    "testbench item must be a list",
                    item,
                ));
            }
        };
        if is_symbol(values.first(), "target") {
            return Err(error(
                GateParseErrorKind::DuplicateTarget,
                "testbench has more than one target",
                item,
            ));
        }
        if is_symbol(values.first(), "clock") {
            if stimulus.is_some() {
                return Err(error(
                    GateParseErrorKind::InvalidStimulusPosition,
                    "clock must appear before stimulus",
                    item,
                ));
            }
            clocks.push(Spanned {
                value: parse_testbench_clock(item)?,
                span: item.span,
            });
        } else if is_symbol(values.first(), "stimulus") {
            if stimulus.is_some() {
                return Err(error(
                    GateParseErrorKind::DuplicateStimulus,
                    "testbench has more than one stimulus",
                    item,
                ));
            }
            stimulus = Some(
                values[1..]
                    .iter()
                    .map(parse_testbench_stmt)
                    .collect::<Result<Vec<_>, _>>()?,
            );
        } else {
            return Err(error(
                GateParseErrorKind::UnknownTestbenchItem,
                "unknown testbench item",
                item,
            ));
        }
    }
    let stimulus = stimulus.ok_or_else(|| {
        error(
            GateParseErrorKind::MissingStimulus,
            "stimulus form is required",
            expression,
        )
    })?;
    Ok(Spanned {
        value: TestbenchDecl {
            name,
            target,
            target_generics,
            clocks,
            stimulus,
        },
        span: expression.span,
    })
}

fn parse_testbench_clock(
    expression: &Spanned<SExpr>,
) -> Result<TestbenchClockDecl, GateParseError> {
    let list = exact_list(
        expression,
        4,
        GateParseErrorKind::InvalidTestbenchClock,
        "clock form must have four elements",
    )?;
    let signal = identifier(
        &list[1],
        GateParseErrorKind::InvalidTestbenchClock,
        "clock signal must be a symbol",
    )?;
    Ok(TestbenchClockDecl {
        signal,
        period: parse_time(&list[2], &list[3])?,
    })
}

fn parse_time(
    value_expr: &Spanned<SExpr>,
    unit: &Spanned<SExpr>,
) -> Result<TimeLiteral, GateParseError> {
    let value = match value_expr.value {
        SExpr::Integer(value) if value > 0 => u64::try_from(value).map_err(|_| {
            error(
                GateParseErrorKind::InvalidTime,
                "time value is out of range",
                value_expr,
            )
        })?,
        _ => {
            return Err(error(
                GateParseErrorKind::InvalidTime,
                "time value must be a positive integer",
                value_expr,
            ));
        }
    };
    let unit = match &unit.value {
        SExpr::Symbol(name) => match name.as_str() {
            "fs" => TimeUnit::Femtosecond,
            "ps" => TimeUnit::Picosecond,
            "ns" => TimeUnit::Nanosecond,
            "us" => TimeUnit::Microsecond,
            "ms" => TimeUnit::Millisecond,
            "sec" => TimeUnit::Second,
            _ => {
                return Err(error(
                    GateParseErrorKind::UnknownTimeUnit,
                    "unknown time unit",
                    unit,
                ));
            }
        },
        _ => {
            return Err(error(
                GateParseErrorKind::UnknownTimeUnit,
                "time unit must be a symbol",
                unit,
            ));
        }
    };
    Ok(TimeLiteral { value, unit })
}

fn parse_testbench_stmt(
    expression: &Spanned<SExpr>,
) -> Result<Spanned<TestbenchStmt>, GateParseError> {
    let list = match &expression.value {
        SExpr::List(values) => values,
        _ => {
            return Err(error(
                GateParseErrorKind::UnknownTestbenchItem,
                "stimulus statement must be a list",
                expression,
            ));
        }
    };
    let value = if is_symbol(list.first(), "drive") {
        let list = exact_list(
            expression,
            3,
            GateParseErrorKind::InvalidDrive,
            "drive must have three elements",
        )?;
        TestbenchStmt::Drive {
            target: identifier(
                &list[1],
                GateParseErrorKind::InvalidDriveTarget,
                "drive target must be a symbol",
            )?,
            value: parse_expr(&list[2])?,
        }
    } else if is_symbol(list.first(), "wait") {
        let list = exact_list(
            expression,
            3,
            GateParseErrorKind::InvalidWait,
            "wait must have three elements",
        )?;
        TestbenchStmt::Wait {
            duration: parse_time(&list[1], &list[2])?,
        }
    } else if is_symbol(list.first(), "wait-rising") {
        if !matches!(list.len(), 2 | 3) {
            return Err(error(
                GateParseErrorKind::InvalidWaitRising,
                "wait-rising must have two or three elements",
                expression,
            ));
        }
        let clock = identifier(
            &list[1],
            GateParseErrorKind::InvalidWaitRising,
            "wait-rising clock must be a symbol",
        )?;
        let count = if let Some(count) = list.get(2) {
            match count.value {
                SExpr::Integer(value) if value > 0 => u32::try_from(value).map_err(|_| {
                    error(
                        GateParseErrorKind::InvalidWaitRising,
                        "wait-rising count exceeds u32",
                        count,
                    )
                })?,
                _ => {
                    return Err(error(
                        GateParseErrorKind::InvalidWaitRising,
                        "wait-rising count must be positive",
                        count,
                    ));
                }
            }
        } else {
            1
        };
        TestbenchStmt::WaitRising { clock, count }
    } else if is_symbol(list.first(), "assert") {
        let list = exact_list(
            expression,
            3,
            GateParseErrorKind::InvalidAssert,
            "assert must have three elements",
        )?;
        let message = match &list[2].value {
            SExpr::String(value) => value.clone(),
            _ => {
                return Err(error(
                    GateParseErrorKind::InvalidAssertMessage,
                    "assert message must be a string",
                    &list[2],
                ));
            }
        };
        TestbenchStmt::Assert {
            condition: parse_expr(&list[1])?,
            message,
        }
    } else {
        return Err(error(
            GateParseErrorKind::UnknownTestbenchItem,
            "unknown stimulus statement",
            expression,
        ));
    };
    Ok(Spanned {
        value,
        span: expression.span,
    })
}

fn exact_list<'a>(
    expression: &'a Spanned<SExpr>,
    length: usize,
    kind: GateParseErrorKind,
    message: &str,
) -> Result<&'a [Spanned<SExpr>], GateParseError> {
    match &expression.value {
        SExpr::List(list) if list.len() == length => Ok(list),
        _ => Err(error(kind, message, expression)),
    }
}
fn identifier(
    expression: &Spanned<SExpr>,
    kind: GateParseErrorKind,
    message: &str,
) -> Result<Identifier, GateParseError> {
    match &expression.value {
        SExpr::Symbol(name) => Ok(Identifier {
            name: name.clone(),
            span: expression.span,
        }),
        _ => Err(error(kind, message, expression)),
    }
}
fn is_symbol(expression: Option<&Spanned<SExpr>>, expected: &str) -> bool {
    matches!(expression.map(|value| &value.value), Some(SExpr::Symbol(value)) if value == expected)
}
fn error(kind: GateParseErrorKind, message: &str, expression: &Spanned<SExpr>) -> GateParseError {
    GateParseError::new(kind, message, expression.span)
}
