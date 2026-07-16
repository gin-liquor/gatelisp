use gatelisp::{
    CompileError, HardwareType, SemanticErrorKind, TypedExprKind, compile_source,
    compile_source_to_vhdl,
};

fn error(source: &str) -> SemanticErrorKind {
    match compile_source(source) {
        Err(CompileError::Semantic(error)) => error.kind,
        other => panic!("semantic error expected: {other:?}"),
    }
}

#[test]
fn types_selectors_results_nesting_and_expected_literals() {
    let typed = compile_source(include_str!("../examples/case_expr.glisp")).unwrap();
    assert!(matches!(
        typed.modules[0].assignments[0].value.kind,
        TypedExprKind::Case { .. }
    ));
    assert_eq!(
        typed.modules[0].assignments[0].value.ty,
        HardwareType::Unsigned(8)
    );
    compile_source(include_str!("../examples/generic_case.glisp")).unwrap();
    compile_source("(module m (ports (s :in bit) (x :in (signed 4)) (y :out (signed 4))) (assign y (case s (0 x) (1 -1) (else 0))))").unwrap();
    compile_source("(module m (ports (s :in bit) (y :out bit)) (assign y (case (case s (0 s) (1 1) (else 0)) (0 1) (1 0) (else 0))))").unwrap();
}

#[test]
fn rejects_runtime_duplicate_out_of_range_and_result_mismatch() {
    assert_eq!(
        error(
            "(module m (ports (s :in (unsigned 2)) (x :in (unsigned 2)) (y :out bit)) (assign y (case s (x 0) (else 1))))"
        ),
        SemanticErrorKind::InvalidCaseLabel
    );
    assert_eq!(
        error(
            "(module m (ports (s :in bit) (y :out bit)) (assign y (case s (0 0) (0 1) (else 0))))"
        ),
        SemanticErrorKind::DuplicateCaseLabel
    );
    assert_eq!(
        error(
            "(module m (ports (s :in (unsigned 2)) (y :out bit)) (assign y (case s (4 0) (else 1))))"
        ),
        SemanticErrorKind::IntegerOutOfRange
    );
    assert_eq!(
        error(
            "(module m (ports (s :in bit) (a :in (unsigned 2)) (b :in (signed 2)) (y :out (unsigned 2))) (assign y (case s (0 a) (1 b) (else a))))"
        ),
        SemanticErrorKind::CaseBranchTypeMismatch
    );
}

#[test]
fn lowers_selector_once_to_deterministic_if_chain() {
    let vhdl = compile_source_to_vhdl(include_str!("../examples/case_expr.glisp")).unwrap();
    assert!(vhdl.contains("gl_tmp_0 := gl_p0_selector;"));
    assert!(vhdl.contains("if (gl_tmp_0 = resize(unsigned'"));
    assert!(vhdl.contains("else"));
}
