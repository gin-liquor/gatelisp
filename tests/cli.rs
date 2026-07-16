use std::process::Command;

fn glispc(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_glispc"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn defaults_to_clean_vhdl_stdout() {
    let output = glispc(&["examples/and_gate.glisp"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("library ieee;"));
    assert!(!stdout.contains("Finished"));
    assert!(output.stderr.is_empty());
}

#[test]
fn supports_all_display_modes() {
    for (mode, marker) in [
        ("--vhdl", "entity "),
        ("--hir", "TypedProgram"),
        ("--ast", "Program"),
        ("--sexpr", "Spanned"),
    ] {
        let output = glispc(&[mode, "examples/and_gate.glisp"]);
        assert!(output.status.success(), "{mode}");
        assert!(String::from_utf8(output.stdout).unwrap().contains(marker));
    }
}

#[test]
fn writes_short_and_long_output_options() {
    for option in ["-o", "--output"] {
        let path = format!("target/cli-{option}.vhd").replace("--", "long-");
        let output = glispc(&["--vhdl", "examples/and_gate.glisp", option, &path]);
        assert!(output.status.success());
        assert!(output.stdout.is_empty());
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .starts_with("library ieee;")
        );
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn rejects_invalid_option_combinations_and_missing_input() {
    for args in [
        vec!["--hir", "--ast", "examples/and_gate.glisp"],
        vec!["--hir", "examples/and_gate.glisp", "-o", "target/no.vhd"],
        vec!["--unknown", "examples/and_gate.glisp"],
        vec!["--vhdl"],
    ] {
        assert!(!glispc(&args).status.success(), "{args:?}");
    }
}

#[test]
fn reports_output_and_compile_failures() {
    assert!(
        !glispc(&["examples/and_gate.glisp", "-o", "missing-parent/out.vhd"])
            .status
            .success()
    );
    let path = "target/cli-invalid.glisp";
    std::fs::write(path, "(module invalid (ports) (assign missing 0))").unwrap();
    assert!(!glispc(&["--vhdl", path]).status.success());
    std::fs::remove_file(path).unwrap();
}
