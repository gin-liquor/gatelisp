use crate::{
    VhdlArchitecture, VhdlConcurrentStatement, VhdlDeclaration, VhdlDesign, VhdlDesignUnit,
    VhdlEntity, VhdlExpression, VhdlGenericKind, VhdlPortMode, VhdlProcess, VhdlSensitivity,
    VhdlSequentialStatement, VhdlType,
};
use std::fmt::Write;

pub fn render_vhdl(design: &VhdlDesign) -> String {
    let mut out = String::new();
    for (index, unit) in design.units.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        let simulation =
            matches!(unit, VhdlDesignUnit::Architecture(value) if value.name.0 == "sim");
        context(&mut out, simulation);
        match unit {
            VhdlDesignUnit::Entity(value) => entity(&mut out, value),
            VhdlDesignUnit::Architecture(value) => architecture(&mut out, value),
        }
    }
    out
}

fn context(out: &mut String, simulation: bool) {
    out.push_str("library ieee;\nuse ieee.std_logic_1164.all;\nuse ieee.numeric_std.all;\n");
    if simulation {
        out.push_str("library std;\nuse std.env.all;\n");
    }
    out.push('\n');
}

fn entity(out: &mut String, value: &VhdlEntity) {
    let _ = writeln!(out, "entity {} is", value.name.0);
    if !value.generics.is_empty() {
        out.push_str("  generic (\n");
        for (i, generic) in value.generics.iter().enumerate() {
            let kind = match generic.kind {
                VhdlGenericKind::Natural => "natural",
                VhdlGenericKind::Positive => "positive",
            };
            let suffix = if i + 1 == value.generics.len() {
                ""
            } else {
                ";"
            };
            let _ = writeln!(
                out,
                "    {} : {} := {}{}",
                generic.name.0, kind, generic.default, suffix
            );
        }
        out.push_str("  );\n");
    }
    if !value.ports.is_empty() {
        out.push_str("  port (\n");
        for (i, port) in value.ports.iter().enumerate() {
            let mode = match port.mode {
                VhdlPortMode::In => "in",
                VhdlPortMode::Out => "out",
            };
            let suffix = if i + 1 == value.ports.len() { "" } else { ";" };
            let _ = writeln!(
                out,
                "    {} : {} {}{}",
                port.name.0,
                mode,
                type_text(&port.ty),
                suffix
            );
        }
        out.push_str("  );\n");
    }
    let _ = writeln!(out, "end entity {};", value.name.0);
}

fn architecture(out: &mut String, value: &VhdlArchitecture) {
    let _ = writeln!(
        out,
        "architecture {} of {} is",
        value.name.0, value.entity_name.0
    );
    for declaration in &value.declarations {
        match declaration {
            VhdlDeclaration::Signal { name, ty, initial } => {
                let _ = write!(out, "  signal {} : {}", name.0, type_text(ty));
                if let Some(initial) = initial { let _ = write!(out, " := {}", expr(initial)); }
                out.push_str(";\n");
            }
            VhdlDeclaration::BoolToStdLogicFunction => out.push_str("  function gl_bool_to_sl(value : boolean) return std_logic is\n  begin\n    if value then\n      return '1';\n    else\n      return '0';\n    end if;\n  end function gl_bool_to_sl;\n"),
            VhdlDeclaration::TruncateUnsignedFunction => out.push_str("  function gl_truncate_unsigned(value : unsigned; size : positive) return unsigned is\n  begin\n    return value(value'low + size - 1 downto value'low);\n  end function gl_truncate_unsigned;\n"),
            VhdlDeclaration::TruncateSignedFunction => out.push_str("  function gl_truncate_signed(value : signed; size : positive) return signed is\n  begin\n    return value(value'low + size - 1 downto value'low);\n  end function gl_truncate_signed;\n"),
            VhdlDeclaration::BitToVectorFunction => out.push_str("  function gl_bit_to_slv(value : std_logic) return std_logic_vector is\n  begin\n    return std_logic_vector'(0 => value);\n  end function gl_bit_to_slv;\n"),
            VhdlDeclaration::Constant { name, ty, value } => { let _ = writeln!(out, "  constant {} : {} := {};", name.0, ty, expr(value)); }
        }
    }
    out.push_str("begin\n");
    for statement in &value.statements {
        concurrent(out, statement);
    }
    let _ = writeln!(out, "end architecture {};", value.name.0);
}

fn concurrent(out: &mut String, statement: &VhdlConcurrentStatement) {
    match statement {
        VhdlConcurrentStatement::Assignment { target, value } => {
            let _ = writeln!(out, "  {} <= {};", target.0, expr(value));
        }
        VhdlConcurrentStatement::Process(process) => process_text(out, process),
        VhdlConcurrentStatement::EntityInstance {
            label,
            entity,
            generics,
            ports,
        } => {
            if ports.is_empty() && generics.is_empty() {
                let _ = writeln!(out, "  {} : entity work.{};", label.0, entity.0);
                return;
            }
            let _ = writeln!(out, "  {} : entity work.{}", label.0, entity.0);
            if !generics.is_empty() {
                out.push_str("    generic map (\n");
                for (index, (formal, actual)) in generics.iter().enumerate() {
                    let suffix = if index + 1 == generics.len() { "" } else { "," };
                    let _ = writeln!(out, "      {} => {}{}", formal.0, expr(actual), suffix);
                }
                out.push_str(if ports.is_empty() {
                    "    );\n"
                } else {
                    "    )\n"
                });
            }
            if ports.is_empty() {
                return;
            }
            out.push_str("    port map (\n");
            for (index, (formal, actual)) in ports.iter().enumerate() {
                let suffix = if index + 1 == ports.len() { "" } else { "," };
                let _ = writeln!(out, "      {} => {}{}", formal.0, actual.0, suffix);
            }
            out.push_str("    );\n");
        }
    }
}

fn process_text(out: &mut String, process: &VhdlProcess) {
    let sensitivity = match &process.sensitivity {
        VhdlSensitivity::None => String::new(),
        VhdlSensitivity::All => "all".to_owned(),
        VhdlSensitivity::Signals(values) => values
            .iter()
            .map(|v| v.0.as_str())
            .collect::<Vec<_>>()
            .join(", "),
    };
    if sensitivity.is_empty() {
        let _ = writeln!(out, "  {} : process", process.label.0);
    } else {
        let _ = writeln!(out, "  {} : process({})", process.label.0, sensitivity);
    }
    for variable in &process.variables {
        let _ = writeln!(
            out,
            "    variable {} : {};",
            variable.name.0,
            type_text(&variable.ty)
        );
    }
    out.push_str("  begin\n");
    statements(out, &process.statements, 2);
    let _ = writeln!(out, "  end process {};", process.label.0);
}

fn statements(out: &mut String, values: &[VhdlSequentialStatement], level: usize) {
    let pad = "  ".repeat(level);
    for value in values {
        match value {
            VhdlSequentialStatement::SignalAssignment { target, value } => {
                let _ = writeln!(out, "{pad}{} <= {};", target.0, expr(value));
            }
            VhdlSequentialStatement::VariableAssignment { target, value } => {
                let _ = writeln!(out, "{pad}{} := {};", target.0, expr(value));
            }
            VhdlSequentialStatement::If {
                condition,
                then_statements,
                else_statements,
            } => {
                let _ = writeln!(out, "{pad}if {} then", expr(condition));
                statements(out, then_statements, level + 1);
                if !else_statements.is_empty() {
                    let _ = writeln!(out, "{pad}else");
                    statements(out, else_statements, level + 1);
                }
                let _ = writeln!(out, "{pad}end if;");
            }
            VhdlSequentialStatement::WaitFor(value) => {
                let _ = writeln!(out, "{pad}wait for {};", expr(value));
            }
            VhdlSequentialStatement::WaitUntil(value) => {
                let _ = writeln!(out, "{pad}wait until {};", expr(value));
            }
            VhdlSequentialStatement::ForLoop {
                variable,
                from,
                to,
                statements: body,
            } => {
                let _ = writeln!(out, "{pad}for {} in {} to {} loop", variable.0, from, to);
                statements(out, body, level + 1);
                let _ = writeln!(out, "{pad}end loop;");
            }
            VhdlSequentialStatement::InfiniteLoop(body) => {
                let _ = writeln!(out, "{pad}loop");
                statements(out, body, level + 1);
                let _ = writeln!(out, "{pad}end loop;");
            }
            VhdlSequentialStatement::Assert { condition, message } => {
                let _ = writeln!(out, "{pad}assert {}", expr(condition));
                let _ = writeln!(out, "{pad}  report \"{}\"", escape_string(message));
                let _ = writeln!(out, "{pad}  severity error;");
            }
            VhdlSequentialStatement::Report(message) => {
                let _ = writeln!(
                    out,
                    "{pad}report \"{}\" severity note;",
                    escape_string(message)
                );
            }
            VhdlSequentialStatement::Stop => {
                let _ = writeln!(out, "{pad}stop;");
            }
            VhdlSequentialStatement::Wait => {
                let _ = writeln!(out, "{pad}wait;");
            }
        }
    }
}

fn escape_string(value: &str) -> String {
    value.replace('"', "\"\"").replace(['\r', '\n'], " ")
}

fn type_text(value: &VhdlType) -> String {
    match value {
        VhdlType::StdLogic => "std_logic".into(),
        VhdlType::Unsigned(w) => format!("unsigned({} downto 0)", width_high(w)),
        VhdlType::Signed(w) => format!("signed({} downto 0)", width_high(w)),
    }
}
fn width_high(value: &VhdlExpression) -> String {
    if let VhdlExpression::Literal(v) = value
        && let Ok(width) = v.parse::<u64>()
    {
        return width.saturating_sub(1).to_string();
    }
    format!("{} - 1", expr(value))
}
fn expr(value: &VhdlExpression) -> String {
    match value {
        VhdlExpression::Name(value) => value.0.clone(),
        VhdlExpression::Literal(value) => value.clone(),
        VhdlExpression::Unary { op, operand } => format!("({op} {})", expr(operand)),
        VhdlExpression::Binary { op, left, right } => {
            format!("({} {op} {})", expr(left), expr(right))
        }
        VhdlExpression::Call {
            function,
            arguments,
        } => format!(
            "{}({})",
            function.0,
            arguments.iter().map(expr).collect::<Vec<_>>().join(", ")
        ),
        VhdlExpression::Slice { value, high, low } => {
            format!("{}({} downto {})", expr(value), expr(high), expr(low))
        }
        VhdlExpression::Concatenate(values) => format!(
            "({})",
            values.iter().map(expr).collect::<Vec<_>>().join(" & ")
        ),
    }
}
