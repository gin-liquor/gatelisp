use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    path::PathBuf,
    process::ExitCode,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    SExpr,
    Ast,
    Hir,
    Vhdl,
}

struct Options {
    mode: Mode,
    input: PathBuf,
    output: Option<PathBuf>,
}

fn main() -> ExitCode {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let options = match parse_options(args.collect()) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            return usage(&program);
        }
    };
    let source = match fs::read_to_string(&options.input) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("{}: {error}", options.input.display());
            return ExitCode::FAILURE;
        }
    };
    let rendered = match options.mode {
        Mode::SExpr => match gatelisp::parse_document(&source) {
            Ok(value) => format!("{value:#?}\n"),
            Err(error) => return report(&options.input, error.span, None, &error.message),
        },
        Mode::Ast => match gatelisp::parse_program(&source) {
            Ok(value) => format!("{value:#?}\n"),
            Err(error) => return report(&options.input, error.span(), None, &error.to_string()),
        },
        Mode::Hir => match gatelisp::compile_source(&source) {
            Ok(value) => format!("{value:#?}\n"),
            Err(error) => {
                return report(
                    &options.input,
                    error.span(),
                    error.related_span(),
                    &error.to_string(),
                );
            }
        },
        Mode::Vhdl => match gatelisp::compile_source_to_vhdl(&source) {
            Ok(value) => value,
            Err(error) => {
                return report(
                    &options.input,
                    error.span(),
                    error.related_span(),
                    &error.to_string(),
                );
            }
        },
    };
    if let Some(output) = options.output {
        if let Err(error) = fs::write(&output, rendered) {
            eprintln!("{}: {error}", output.display());
            return ExitCode::FAILURE;
        }
    } else {
        print!("{rendered}");
    }
    ExitCode::SUCCESS
}

fn parse_options(args: Vec<OsString>) -> Result<Options, String> {
    let mut mode = None;
    let mut input = None;
    let mut output = None;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        let text = arg.to_string_lossy();
        match text.as_ref() {
            "--sexpr" => set_mode(&mut mode, Mode::SExpr)?,
            "--ast" => set_mode(&mut mode, Mode::Ast)?,
            "--hir" => set_mode(&mut mode, Mode::Hir)?,
            "--vhdl" => set_mode(&mut mode, Mode::Vhdl)?,
            "-o" | "--output" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| format!("{text} requires a file path"))?;
                if output.replace(PathBuf::from(value)).is_some() {
                    return Err("output option specified more than once".into());
                }
            }
            _ if text.starts_with('-') => return Err(format!("unknown option: {text}")),
            _ => {
                if input.replace(PathBuf::from(arg)).is_some() {
                    return Err("multiple input files specified".into());
                }
            }
        }
        index += 1;
    }
    let mode = mode.unwrap_or(Mode::Vhdl);
    if output.is_some() && mode != Mode::Vhdl {
        return Err("-o/--output is valid only for VHDL output".into());
    }
    Ok(Options {
        mode,
        input: input.ok_or_else(|| "input file is required".to_owned())?,
        output,
    })
}

fn set_mode(mode: &mut Option<Mode>, value: Mode) -> Result<(), String> {
    if mode.replace(value).is_some() {
        Err("display modes are mutually exclusive".into())
    } else {
        Ok(())
    }
}

fn usage(program: &OsStr) -> ExitCode {
    eprintln!(
        "usage: {} [--sexpr|--ast|--hir|--vhdl] <source-file> [-o <file>]",
        program.to_string_lossy()
    );
    ExitCode::FAILURE
}

fn report(
    path: &std::path::Path,
    span: gatelisp::Span,
    related: Option<gatelisp::Span>,
    message: &str,
) -> ExitCode {
    eprintln!(
        "{}:{}:{}: {message}",
        path.display(),
        span.start.line,
        span.start.column
    );
    if let Some(related) = related {
        eprintln!(
            "{}:{}:{}: related declaration is here",
            path.display(),
            related.start.line,
            related.start.column
        );
    }
    ExitCode::FAILURE
}
