use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let Some(path) = args.next() else {
        eprintln!("usage: {} <source-file>", program.to_string_lossy());
        return ExitCode::FAILURE;
    };
    if args.next().is_some() {
        eprintln!("usage: {} <source-file>", program.to_string_lossy());
        return ExitCode::FAILURE;
    }
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("{}: {error}", path.to_string_lossy());
            return ExitCode::FAILURE;
        }
    };
    match gatelisp::parse_document(&source) {
        Ok(ast) => {
            println!("{ast:#?}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!(
                "{}:{}:{}: {}",
                path.to_string_lossy(),
                error.span.start.line,
                error.span.start.column,
                error.message
            );
            ExitCode::FAILURE
        }
    }
}
