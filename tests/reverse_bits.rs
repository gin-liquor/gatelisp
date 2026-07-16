use gatelisp::{
    CompileError, HardwareType, SemanticErrorKind, TypedExprKind, compile_source,
    compile_source_to_vhdl, parse_program,
};
fn error(source: &str) -> SemanticErrorKind {
    let CompileError::Semantic(e) = compile_source(source).unwrap_err() else {
        panic!("semantic error expected")
    };
    e.kind
}
#[test]
fn parses_and_types_unsigned_signed_generic_and_nested_reverse() {
    let p=parse_program("(module m (ports (x :in (unsigned 8)) (y :out (unsigned 8))) (assign y (reverse-bits (reverse-bits x))))").unwrap();
    assert_eq!(p.modules[0].value.items.len(), 1);
    let t = compile_source(include_str!("../examples/reverse_bits_signed.glisp")).unwrap();
    assert!(matches!(
        t.modules[0].assignments[0].value.kind,
        TypedExprKind::ReverseBits { .. }
    ));
    assert_eq!(
        t.modules[0].assignments[0].value.ty,
        HardwareType::Unsigned(8)
    );
    let g = compile_source(include_str!("../examples/generic_reverse_bits.glisp")).unwrap();
    assert_eq!(
        g.modules[0].assignments[0].value.ty,
        g.modules[0].signals[1].ty
    );
}
#[test]
fn rejects_bit_integer_and_wrong_arity_without_expected_type_inference() {
    assert_eq!(
        error("(module m (ports (x :in bit) (y :out bit)) (assign y (reverse-bits x)))"),
        SemanticErrorKind::InvalidReverseBitsSource
    );
    assert_eq!(
        error("(module m (ports (y :out (unsigned 8))) (assign y (reverse-bits 1)))"),
        SemanticErrorKind::CannotInferIntegerType
    );
    assert_eq!(
        error("(module m (ports (y :out (unsigned 8))) (assign y (reverse-bits)))"),
        SemanticErrorKind::WrongArgumentCount
    );
    assert_eq!(
        error(
            "(module m (ports (x :in (unsigned 8)) (y :out (unsigned 8))) (assign y (reverse-bits x x)))"
        ),
        SemanticErrorKind::WrongArgumentCount
    );
}
#[test]
fn integrates_with_concat_slice_if_clocked_and_testbench() {
    compile_source("(module m (ports (clk :in bit) (s :in bit) (a :in (unsigned 8)) (b :in (unsigned 8)) (y :out (unsigned 8))) (reg r (unsigned 8) 0) (clocked clk (next r (reverse-bits (if s a b)))) (assign y (reverse-bits (slice (concat a b) 0 8))))").unwrap();
    compile_source(include_str!("../examples/reverse_bits_test.glisp")).unwrap();
}
#[test]
fn lowers_one_helper_signed_cast_loop_and_attributes() {
    let v = compile_source_to_vhdl(include_str!("../examples/reverse_bits_test.glisp")).unwrap();
    assert_eq!(v.matches("function gl_reverse_bits(").count(), 1);
    assert!(v.contains("for offset in 0 to value'length - 1 loop"));
    assert!(v.contains("result(result'low + offset) := value(value'high - offset)"));
    let s = compile_source_to_vhdl(include_str!("../examples/reverse_bits_signed.glisp")).unwrap();
    assert!(s.contains("gl_reverse_bits(unsigned(gl_p0_input))"));
    let plain = compile_source_to_vhdl(include_str!("../examples/and_gate.glisp")).unwrap();
    assert!(!plain.contains("gl_reverse_bits"));
}
