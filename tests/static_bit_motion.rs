use gatelisp::{
    CompileError, HardwareType, SemanticErrorKind, StaticBitMotionKind, TypedExprKind, WidthExpr,
    compile_source, compile_source_to_vhdl,
};

fn semantic_error(source: &str) -> SemanticErrorKind {
    match compile_source(source) {
        Err(CompileError::Semantic(error)) => error.kind,
        result => panic!("semantic error expected, got {result:?}"),
    }
}

#[test]
fn types_all_five_operations_and_preserves_type() {
    let source = "(module m (ports (u :in (unsigned 8)) (s :in (signed 8)) (a :out (unsigned 8)) (b :out (unsigned 8)) (c :out (signed 8)) (d :out (unsigned 8)) (e :out (signed 8))) (assign a (shift-left u 0)) (assign b (shift-right-logical u 7)) (assign c (shift-right-arithmetic s 1)) (assign d (rotate-left u 2)) (assign e (rotate-right s 3)))";
    let typed = compile_source(source).unwrap();
    let kinds = [
        StaticBitMotionKind::ShiftLeft,
        StaticBitMotionKind::ShiftRightLogical,
        StaticBitMotionKind::ShiftRightArithmetic,
        StaticBitMotionKind::RotateLeft,
        StaticBitMotionKind::RotateRight,
    ];
    for (assignment, expected) in typed.modules[0].assignments.iter().zip(kinds) {
        assert!(
            matches!(&assignment.value.kind, TypedExprKind::StaticBitMotion { kind, .. } if *kind == expected)
        );
    }
    assert_eq!(
        typed.modules[0].assignments[2].value.ty,
        HardwareType::Signed(8)
    );
}

#[test]
fn resolves_normalizes_and_proves_generic_amount() {
    let typed = compile_source(include_str!("../examples/generic_shift.glisp")).unwrap();
    let TypedExprKind::StaticBitMotion { amount, .. } = &typed.modules[0].assignments[0].value.kind
    else {
        panic!("motion expected")
    };
    assert_eq!(*amount, WidthExpr::Generic(typed.modules[0].generics[0].id));
    assert!(compile_source("(module m (generics (n :positive 2)) (ports (x :in (unsigned (+ n 2))) (y :out (unsigned (+ n 2)))) (assign y (rotate-left x (+ 0 (* 1 n)))))").is_ok());
}

#[test]
fn rejects_invalid_sources_signedness_and_ranges() {
    assert_eq!(
        semantic_error("(module m (ports (x :in bit) (y :out bit)) (assign y (shift-left x 0)))"),
        SemanticErrorKind::InvalidStaticBitMotionSource
    );
    assert_eq!(
        semantic_error(
            "(module m (ports (x :in (signed 8)) (y :out (signed 8))) (assign y (shift-right-logical x 1)))"
        ),
        SemanticErrorKind::InvalidStaticBitMotionSignedness
    );
    assert_eq!(
        semantic_error(
            "(module m (ports (x :in (unsigned 8)) (y :out (unsigned 8))) (assign y (shift-right-arithmetic x 1)))"
        ),
        SemanticErrorKind::InvalidStaticBitMotionSignedness
    );
    assert_eq!(
        semantic_error(
            "(module m (ports (x :in (unsigned 8)) (y :out (unsigned 8))) (assign y (rotate-right x 8)))"
        ),
        SemanticErrorKind::StaticBitMotionAmountOutOfRange
    );
    assert_eq!(
        semantic_error(
            "(module m (generics (w :positive 8) (n :natural 1)) (ports (x :in (unsigned w)) (y :out (unsigned w))) (assign y (shift-left x n)))"
        ),
        SemanticErrorKind::StaticBitMotionAmountRangeUnknown
    );
    assert_eq!(
        semantic_error(
            "(module m (ports (n :in (unsigned 8)) (x :in (unsigned 8)) (y :out (unsigned 8))) (assign y (shift-left x n)))"
        ),
        SemanticErrorKind::UnknownGeneric
    );
}

#[test]
fn lowers_to_numeric_std_calls_and_integrates_with_complex_sources() {
    let vhdl = compile_source_to_vhdl(include_str!("../examples/shift_rotate_test.glisp")).unwrap();
    for call in [
        "shift_left(",
        "shift_right(",
        "rotate_left(",
        "rotate_right(",
    ] {
        assert!(vhdl.contains(call));
    }
    assert!(!vhdl.contains("function gl_shift"));
    let nested = compile_source_to_vhdl("(module m (ports (sel :in bit) (a :in (unsigned 8)) (b :in (unsigned 8)) (y :out (unsigned 8))) (assign y (reverse-bits (rotate-left (if sel a b) 1))))").unwrap();
    assert!(nested.contains("rotate_left(gl_tmp_0, 1)"));
}

#[test]
fn supports_clocked_and_testbench_forms() {
    compile_source("(module m (ports (clk :in bit) (x :in (unsigned 8)) (y :out (unsigned 8))) (reg r (unsigned 8) 0) (clocked clk (next r (shift-left x 1))) (assign y r))").unwrap();
    compile_source(include_str!("../examples/generic_shift_test.glisp")).unwrap();
}
