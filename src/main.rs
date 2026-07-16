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
    let arguments = args.collect::<Vec<_>>();
    if arguments.first().is_some_and(|value| value == "test") {
        return run_tests(&program, &arguments[1..]);
    }
    let options = match parse_options(arguments) {
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

fn run_tests(program: &OsStr, args: &[OsString]) -> ExitCode {
    let mut input = None;
    let mut options = gatelisp::SimulationOptions::default();
    let mut index = 0;
    while index < args.len() {
        let text = args[index].to_string_lossy();
        let destination = match text.as_ref() {
            "--testbench" => Some(0),
            "--vcd" => Some(1),
            "--ghdl" => Some(2),
            "--work-dir" => Some(3),
            _ if text.starts_with('-') => {
                eprintln!("unknown test option: {text}");
                return usage(program);
            }
            _ => {
                if input.replace(PathBuf::from(&args[index])).is_some() {
                    eprintln!("multiple input files specified");
                    return usage(program);
                }
                None
            }
        };
        if let Some(destination) = destination {
            index += 1;
            let Some(value) = args.get(index) else {
                eprintln!("{text} requires a value");
                return usage(program);
            };
            match destination {
                0 => options.testbench = Some(value.to_string_lossy().into_owned()),
                1 => options.vcd_path = Some(PathBuf::from(value)),
                2 => options.ghdl_path = Some(PathBuf::from(value)),
                _ => options.work_directory = Some(PathBuf::from(value)),
            }
        }
        index += 1;
    }
    let Some(input) = input else {
        eprintln!("test input file is required");
        return usage(program);
    };
    let source = match fs::read_to_string(&input) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("{}: {error}", input.display());
            return ExitCode::FAILURE;
        }
    };
    match gatelisp::simulate_source(&source, &options) {
        Ok(results) => {
            for result in results {
                print!("{}", result.stdout);
                eprint!("{}", result.stderr);
                println!("testbench passed: {}", result.testbench_name);
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("simulation {:?}: {error}", error.stage);
            ExitCode::FAILURE
        }
    }
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
        "usage: {} [--sexpr|--ast|--hir|--vhdl] <source-file> [-o <file>]\n       {} test <source-file> [--testbench <name>] [--vcd <file>] [--ghdl <path>] [--work-dir <dir>]",
        program.to_string_lossy(),
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
