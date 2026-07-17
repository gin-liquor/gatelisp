use gatelisp::{CompileError, SemanticErrorKind, compile_source, compile_source_to_vhdl};

#[test]
fn parses_and_lowers_nested_case_do() {
    let source = "(module m (ports (clk :in bit) (outer :in bit) (inner :in bit)) (reg r bit 0) (clocked clk (case-do outer (0 (case-do inner (0 (next r 1)) (else (next r 0)))) (else (next r 0)))))";
    compile_source(source).unwrap();
    let vhdl = compile_source_to_vhdl(source).unwrap();
    assert!(vhdl.contains("gl_tmp_"));
    assert!(vhdl.contains("if "));
}

#[test]
fn compiles_nested_case_do_example() {
    compile_source(include_str!("../examples/nested_case_do.glisp")).unwrap();
}

#[test]
fn lowers_nested_case_do_in_else_arm() {
    let source = "(module m (ports (clk :in bit) (outer :in bit) (inner :in bit)) (reg r bit 0) (clocked clk (case-do outer (0 (next r 0)) (else (case-do inner (0 (next r 1)) (else (next r 0)))))))";
    let vhdl = compile_source_to_vhdl(source).unwrap();
    assert!(vhdl.matches("gl_tmp_").count() >= 2);
}

#[test]
fn rejects_direct_and_nested_updates_in_one_arm() {
    let source = "(module m (ports (clk :in bit) (outer :in bit) (inner :in bit)) (reg r bit 0) (clocked clk (case-do outer (0 (next r 0) (case-do inner (0 (next r 1)))))))";
    let Err(CompileError::Semantic(error)) = compile_source(source) else {
        panic!("semantic error expected");
    };
    assert_eq!(error.kind, SemanticErrorKind::DuplicateNext);
}

#[test]
fn permits_one_array_write_port_across_exclusive_arms() {
    let source = "(module m (ports (clk :in bit) (s :in bit) (a :in (unsigned 2))) (register-array regs :address-width 2 :data-width 8 :initial 0) (clocked clk (case-do s (0 (register-array-write regs a 1)) (else (register-array-write regs a 2)))))";
    compile_source(source).unwrap();
}
