use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let first = args.next();
    let (show_sexpr, path) = match first.as_deref().and_then(|value| value.to_str()) {
        Some("--sexpr") => (true, args.next()),
        _ => (false, first),
    };
    let Some(path) = path else {
        eprintln!(
            "usage: {} [--sexpr] <source-file>",
            program.to_string_lossy()
        );
        return ExitCode::FAILURE;
    };
    if args.next().is_some() {
        eprintln!(
            "usage: {} [--sexpr] <source-file>",
            program.to_string_lossy()
        );
        return ExitCode::FAILURE;
    }
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("{}: {error}", path.to_string_lossy());
            return ExitCode::FAILURE;
        }
    };
    if show_sexpr {
        match gatelisp::parse_document(&source) {
            Ok(ast) => println!("{ast:#?}"),
            Err(error) => return report(&path, error.span, &error.message),
        }
    } else {
        match gatelisp::parse_program(&source) {
            Ok(ast) => println!("{ast:#?}"),
            Err(error) => return report(&path, error.span(), &error.to_string()),
        }
    }
    ExitCode::SUCCESS
}

fn report(path: &std::ffi::OsStr, span: gatelisp::Span, message: &str) -> ExitCode {
    eprintln!(
        "{}:{}:{}: {message}",
        path.to_string_lossy(),
        span.start.line,
        span.start.column
    );
    ExitCode::FAILURE
}
