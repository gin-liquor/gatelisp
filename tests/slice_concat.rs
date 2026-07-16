use gatelisp::{
    CompileError, Expr, HardwareType, SemanticErrorKind, TypedExprKind, WidthExpr, compile_source,
    compile_source_to_vhdl, parse_program,
};

fn semantic_error(source: &str) -> SemanticErrorKind {
    let CompileError::Semantic(error) = compile_source(source).unwrap_err() else {
        panic!("semantic error expected")
    };
    error.kind
}

#[test]
fn parser_builds_slice_const_expressions_and_concat() {
    let program=parse_program("(module m (generics (w :positive 8)) (ports (x :in (unsigned (+ w w))) (y :out (unsigned w))) (assign y (slice x w (* w 1))))").unwrap();
    let gatelisp::ModuleItem::Assign(assign) = &program.modules[0].value.items[0].value else {
        panic!("assign expected")
    };
    assert!(matches!(assign.value.value, Expr::Slice { .. }));
    let program=parse_program("(module m (ports (a :in bit) (b :in bit) (c :in bit) (y :out (unsigned 3))) (assign y (concat a (concat b c))))").unwrap();
    let gatelisp::ModuleItem::Assign(assign) = &program.modules[0].value.items[0].value else {
        panic!("assign expected")
    };
    assert!(matches!(assign.value.value, Expr::Concat { .. }));
    assert!(parse_program("(module m (ports) (assign x (slice x 0)))").is_err());
    assert!(parse_program("(module m (ports) (assign x (concat x)))").is_err());
}

#[test]
fn resolves_slice_types_bounds_and_signed_source() {
    let typed = compile_source(include_str!("../examples/slice_signed.glisp")).unwrap();
    let TypedExprKind::Slice { offset, width, .. } = &typed.modules[0].assignments[0].value.kind
    else {
        panic!("slice expected")
    };
    assert_eq!(*offset, WidthExpr::Constant(0));
    assert_eq!(*width, WidthExpr::Constant(8));
    assert_eq!(
        typed.modules[0].assignments[0].value.ty,
        HardwareType::Unsigned(8)
    );
    assert_eq!(
        semantic_error(
            "(module m (ports (x :in bit) (y :out (unsigned 1))) (assign y (slice x 0 1)))"
        ),
        SemanticErrorKind::SliceSourceNotVector
    );
    assert_eq!(
        semantic_error(
            "(module m (ports (x :in (unsigned 8)) (y :out (unsigned 8))) (assign y (slice x 4 8)))"
        ),
        SemanticErrorKind::SliceOutOfBounds
    );
    assert_eq!(
        semantic_error(
            "(module m (generics (a :positive 1) (b :positive 1)) (ports (x :in (unsigned a)) (y :out (unsigned b))) (assign y (slice x 0 b)))"
        ),
        SemanticErrorKind::SliceRangeUnknown
    );
}

#[test]
fn concat_flattens_preserves_order_and_sums_symbolic_widths() {
    let typed=compile_source("(module m (generics (w :positive 8)) (ports (a :in bit) (b :in (signed w)) (c :in (unsigned w)) (y :out (unsigned (+ (+ 1 w) w)))) (assign y (concat a (concat b c))))").unwrap();
    let value = &typed.modules[0].assignments[0].value;
    let TypedExprKind::Concat { values } = &value.kind else {
        panic!("concat expected")
    };
    assert_eq!(values.len(), 3);
    assert_eq!(values[0].ty, HardwareType::Bit);
    assert!(matches!(value.ty, HardwareType::SymbolicUnsigned(_)));
    assert_eq!(
        semantic_error(
            "(module m (ports (x :in (unsigned 8)) (y :out (unsigned 9))) (assign y (concat 1 x)))"
        ),
        SemanticErrorKind::CannotInferIntegerType
    );
}

#[test]
fn vhdl_lowers_ranges_concat_order_and_bit_helper() {
    let slice = compile_source_to_vhdl(include_str!("../examples/slice.glisp")).unwrap();
    assert!(slice.contains("gl_p0_input(((0 + 8) - 1) downto 0)"));
    let concat = compile_source_to_vhdl(include_str!("../examples/concat.glisp")).unwrap();
    assert!(concat.contains("std_logic_vector(gl_p0_high) & std_logic_vector(gl_p1_low)"));
    assert!(!concat.contains("gl_bit_to_slv"));
    let bits = compile_source_to_vhdl(include_str!("../examples/concat_bits.glisp")).unwrap();
    assert_eq!(bits.matches("function gl_bit_to_slv(").count(), 1);
    assert!(bits.contains("gl_bit_to_slv(gl_p0_valid) & std_logic_vector(gl_p1_payload)"));
    let generic = compile_source_to_vhdl(include_str!("../examples/generic_slice.glisp")).unwrap();
    assert!(generic.contains("downto gl_g0"));
}
