use std::fmt;

use crate::{
    AssignStmt, ClockedDecl, Expr, Identifier, ModuleDecl, ModuleItem, NextStmt, PortDecl,
    PortDirection, Program, RegisterDecl, ResetDecl, ResetKind, SExpr, Span, Spanned,
    TestbenchClockDecl, TestbenchDecl, TestbenchStmt, TimeLiteral, TimeUnit, TypeExpr, WireDecl,
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
    let ports_expr = list.get(2).ok_or_else(|| {
        error(
            GateParseErrorKind::MissingPorts,
            "ports form is required after module name",
            expression,
        )
    })?;
    let ports = parse_ports(ports_expr)?;
    let mut items = Vec::new();
    for item in &list[3..] {
        items.push(parse_item(item)?);
    }
    Ok(Spanned {
        value: ModuleDecl { name, ports, items },
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
            let width = parse_width(&list[1])?;
            if is_symbol(list.first(), "unsigned") {
                TypeExpr::Unsigned(width)
            } else {
                TypeExpr::Signed(width)
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

fn parse_width(expression: &Spanned<SExpr>) -> Result<u32, GateParseError> {
    match expression.value {
        SExpr::Integer(0) => Err(error(
            GateParseErrorKind::ZeroWidth,
            "type width must be greater than zero",
            expression,
        )),
        SExpr::Integer(value) if value < 0 => Err(error(
            GateParseErrorKind::NegativeWidth,
            "type width cannot be negative",
            expression,
        )),
        SExpr::Integer(value) => u32::try_from(value).map_err(|_| {
            error(
                GateParseErrorKind::WidthOutOfRange,
                "type width exceeds u32",
                expression,
            )
        }),
        _ => Err(error(
            GateParseErrorKind::InvalidType,
            "type width must be an integer",
            expression,
        )),
    }
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
    if target_list.len() != 2 {
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
