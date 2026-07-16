use gatelisp::{
    CompileError, HardwareType, SemanticErrorKind, TypedExprKind, WidthExpr, compile_source,
    compile_source_to_vhdl,
};

fn error(source: &str) -> SemanticErrorKind {
    match compile_source(source) {
        Err(CompileError::Semantic(error)) => error.kind,
        result => panic!("semantic error expected, got {result:?}"),
    }
}

#[test]
fn types_concrete_signed_symbolic_and_composed_sources() {
    let typed = compile_source(include_str!("../examples/bit_at.glisp")).unwrap();
    assert!(
        typed.modules[0]
            .assignments
            .iter()
            .all(|a| a.value.ty == HardwareType::Bit)
    );
    let generic = compile_source(include_str!("../examples/generic_bit_at.glisp")).unwrap();
    let TypedExprKind::BitAt { index, .. } = &generic.modules[0].assignments[0].value.kind else {
        panic!("bit-at expected")
    };
    assert_eq!(
        *index,
        WidthExpr::Subtract(
            Box::new(WidthExpr::Generic(generic.modules[0].generics[0].id)),
            Box::new(WidthExpr::Constant(1))
        )
    );
    compile_source("(module m (ports (s :in bit) (a :in (unsigned 4)) (b :in (unsigned 4)) (y :out bit)) (assign y (bit-at (if s a b) 2)))").unwrap();
    compile_source("(module m (ports (a :in (unsigned 4)) (b :in (unsigned 4)) (y :out bit)) (assign y (bit-at (slice (concat a b) 2 4) 3)))").unwrap();
}

#[test]
fn rejects_invalid_source_runtime_index_and_unproven_ranges() {
    assert_eq!(
        error("(module m (ports (x :in bit) (y :out bit)) (assign y (bit-at x 0)))"),
        SemanticErrorKind::InvalidBitAtSource
    );
    assert_eq!(
        error("(module m (ports (y :out bit)) (assign y (bit-at 1 0)))"),
        SemanticErrorKind::CannotInferIntegerType
    );
    assert_eq!(
        error("(module m (ports (x :in (unsigned 8)) (y :out bit)) (assign y (bit-at x 8)))"),
        SemanticErrorKind::BitAtIndexOutOfRange
    );
    assert_eq!(
        error(
            "(module m (generics (w :positive 8)) (ports (x :in (unsigned w)) (y :out bit)) (assign y (bit-at x 7)))"
        ),
        SemanticErrorKind::BitAtIndexRangeUnknown
    );
    assert_eq!(
        error(
            "(module m (ports (i :in (unsigned 4)) (x :in (unsigned 8)) (y :out bit)) (assign y (bit-at x i)))"
        ),
        SemanticErrorKind::UnknownGeneric
    );
}

#[test]
fn lowers_one_helper_casts_signed_and_handles_composed_values() {
    let vhdl = compile_source_to_vhdl(include_str!("../examples/bit_at.glisp")).unwrap();
    assert_eq!(vhdl.matches("function gl_bit_at(").count(), 1);
    assert!(vhdl.contains("gl_bit_at(gl_p0_data, 0)"));
    assert!(vhdl.contains("gl_bit_at(unsigned(gl_p1_signed_data), 7)"));
    assert!(vhdl.contains("gl_bit_at(gl_reverse_bits(gl_p0_data), 3)"));
    assert!(
        !compile_source_to_vhdl(include_str!("../examples/and_gate.glisp"))
            .unwrap()
            .contains("gl_bit_at")
    );
}

#[test]
fn works_in_clocked_and_testbench_expressions() {
    compile_source("(module m (ports (clk :in bit) (x :in (unsigned 8)) (y :out bit)) (reg r bit 0) (clocked clk (next r (bit-at x 7))) (assign y r))").unwrap();
    compile_source("(module m (ports (x :in (unsigned 8)) (y :out (unsigned 8))) (assign y x)) (testbench t (target m) (stimulus (drive x 1) (wait 1 ns) (assert (= (bit-at y 0) 1) \"selected bit\")))").unwrap();
}
