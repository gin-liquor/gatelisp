use std::{env, ffi::OsStr, fs, process::ExitCode};

#[derive(Clone, Copy)]
enum Mode {
    SExpr,
    Ast,
    Hir,
}

fn main() -> ExitCode {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let first = args.next();
    let (mode, path) = match first.as_deref().and_then(OsStr::to_str) {
        Some("--sexpr") => (Mode::SExpr, args.next()),
        Some("--ast") => (Mode::Ast, args.next()),
        Some("--hir") => (Mode::Hir, args.next()),
        _ => (Mode::Hir, first),
    };
    let Some(path) = path else {
        return usage(&program);
    };
    if args.next().is_some() {
        return usage(&program);
    }
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("{}: {error}", path.to_string_lossy());
            return ExitCode::FAILURE;
        }
    };
    match mode {
        Mode::SExpr => match gatelisp::parse_document(&source) {
            Ok(value) => println!("{value:#?}"),
            Err(error) => return report(&path, error.span, None, &error.message),
        },
        Mode::Ast => match gatelisp::parse_program(&source) {
            Ok(value) => println!("{value:#?}"),
            Err(error) => return report(&path, error.span(), None, &error.to_string()),
        },
        Mode::Hir => match gatelisp::compile_source(&source) {
            Ok(value) => println!("{value:#?}"),
            Err(error) => {
                return report(
                    &path,
                    error.span(),
                    error.related_span(),
                    &error.to_string(),
                );
            }
        },
    }
    ExitCode::SUCCESS
}

fn usage(program: &OsStr) -> ExitCode {
    eprintln!(
        "usage: {} [--sexpr|--ast|--hir] <source-file>",
        program.to_string_lossy()
    );
    ExitCode::FAILURE
}

fn report(
    path: &OsStr,
    span: gatelisp::Span,
    related: Option<gatelisp::Span>,
    message: &str,
) -> ExitCode {
    eprintln!(
        "{}:{}:{}: {message}",
        path.to_string_lossy(),
        span.start.line,
        span.start.column
    );
    if let Some(related) = related {
        eprintln!(
            "{}:{}:{}: related declaration is here",
            path.to_string_lossy(),
            related.start.line,
            related.start.column
        );
    }
    ExitCode::FAILURE
}
