use gatelisp::{CompileError, SemanticErrorKind, compile_source, compile_source_to_vhdl};

#[test]
fn supports_multiple_updates_else_omission_edges_and_reset() {
    compile_source(include_str!("../examples/case_do.glisp")).unwrap();
    compile_source(include_str!("../examples/case_fsm.glisp")).unwrap();
    compile_source("(module m (ports (clk :in bit) (s :in bit)) (reg r bit 0) (clocked (rising clk) (case-do s (0 (next r 1)) (1 (next r 0)))))").unwrap();
}

#[test]
fn rejects_duplicate_labels_and_cross_block_drivers() {
    let Err(CompileError::Semantic(error)) = compile_source(
        "(module m (ports (clk :in bit) (s :in bit)) (reg r bit 0) (clocked clk (case-do s (0 (set! r 0)) (0 (set! r 1)))))",
    ) else {
        panic!("error expected")
    };
    assert_eq!(error.kind, SemanticErrorKind::DuplicateCaseLabel);
    let Err(CompileError::Semantic(error)) = compile_source(
        "(module m (ports (clk :in bit) (s :in bit)) (reg r bit 0) (clocked clk (case-do s (0 (set! r 0)))) (clocked (falling clk) (case-do s (1 (set! r 1)))))",
    ) else {
        panic!("error expected")
    };
    assert_eq!(
        error.kind,
        SemanticErrorKind::RegisterMultipleClockedDrivers
    );
}

#[test]
fn lowers_inside_falling_edge_with_exclusive_branches() {
    let vhdl = compile_source_to_vhdl(include_str!("../examples/case_fsm.glisp")).unwrap();
    assert!(vhdl.contains("if falling_edge(gl_p0_clk) then"));
    assert!(vhdl.contains("gl_tmp_0 := gl_s") && vhdl.contains("_state;"));
    assert!(vhdl.contains("elsif") || vhdl.matches("else").count() > 1);
}
