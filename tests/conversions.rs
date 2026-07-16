use gatelisp::{
    CompileError, ConversionKind, Expr, HardwareType, SemanticErrorKind, TypedExprKind,
    compile_source, compile_source_to_vhdl, parse_program,
};

fn semantic_error(source: &str) -> SemanticErrorKind {
    let CompileError::Semantic(error) = compile_source(source).unwrap_err() else {
        panic!("semantic error expected")
    };
    error.kind
}

#[test]
fn parser_preserves_conversion_target_types_and_nesting() {
    let program = parse_program("(module m (ports (a :in (unsigned 8)) (y :out (signed 16))) (assign y (resize (signed 16) (as-signed a))))").unwrap();
    let gatelisp::ModuleItem::Assign(assign) = &program.modules[0].value.items[0].value else {
        panic!("assign expected")
    };
    let Expr::Resize { target, value } = &assign.value.value else {
        panic!("resize expected")
    };
    assert_eq!(target.value, gatelisp::TypeExpr::Signed(16));
    assert!(matches!(value.value, Expr::Call { .. }));
    assert!(parse_program("(module m (ports) (assign x (resize (unsigned 8))))").is_err());
    assert!(parse_program("(module m (ports) (assign x (truncate bit x)))").is_ok());
}

#[test]
fn types_resize_truncate_and_reinterpretation() {
    let source = "(module m (ports (u8 :in (unsigned 8)) (s16 :in (signed 16)) (u16 :out (unsigned 16)) (s8 :out (signed 8)) (r :out (signed 8))) (assign u16 (resize (unsigned 16) u8)) (assign s8 (truncate (signed 8) s16)) (assign r (as-signed u8)))";
    let typed = compile_source(source).unwrap();
    let assignments = &typed.modules[0].assignments;
    assert!(matches!(
        assignments[0].value.kind,
        TypedExprKind::Convert {
            kind: ConversionKind::Resize,
            ..
        }
    ));
    assert!(matches!(
        assignments[1].value.kind,
        TypedExprKind::Convert {
            kind: ConversionKind::Truncate,
            ..
        }
    ));
    assert!(matches!(
        assignments[2].value.kind,
        TypedExprKind::Convert {
            kind: ConversionKind::AsSigned,
            ..
        }
    ));
    assert_eq!(assignments[0].value.ty, HardwareType::Unsigned(16));
    assert_eq!(assignments[1].value.ty, HardwareType::Signed(8));
}

#[test]
fn rejects_invalid_directions_signedness_bits_literals_and_implicit_casts() {
    for (source, kind) in [
        (
            "(module m (ports (x :in (unsigned 16)) (y :out (unsigned 8))) (assign y (resize (unsigned 8) x)))",
            SemanticErrorKind::InvalidResizeDirection,
        ),
        (
            "(module m (ports (x :in (unsigned 8)) (y :out (unsigned 16))) (assign y (truncate (unsigned 16) x)))",
            SemanticErrorKind::InvalidTruncateDirection,
        ),
        (
            "(module m (ports (x :in (unsigned 8)) (y :out (signed 16))) (assign y (resize (signed 16) x)))",
            SemanticErrorKind::ConversionSignednessMismatch,
        ),
        (
            "(module m (ports (x :in bit) (y :out (unsigned 8))) (assign y (resize (unsigned 8) x)))",
            SemanticErrorKind::ConversionSourceNotVector,
        ),
        (
            "(module m (ports (y :out (unsigned 8))) (assign y (resize (unsigned 8) 1)))",
            SemanticErrorKind::CannotInferIntegerType,
        ),
        (
            "(module m (ports (x :in (unsigned 8)) (y :out (signed 8))) (assign y x))",
            SemanticErrorKind::AssignTypeMismatch,
        ),
    ] {
        assert_eq!(semantic_error(source), kind);
    }
}

#[test]
fn proves_supported_symbolic_relations_and_rejects_unknown_ones() {
    compile_source(include_str!("../examples/generic_resize.glisp")).unwrap();
    compile_source("(module m (generics (w :positive 1)) (ports (x :in (unsigned 8)) (y :out (unsigned (+ w 15)))) (assign y (resize (unsigned (+ w 15)) x)))").unwrap();
    let unknown = "(module m (generics (a :positive 1) (b :positive 1)) (ports (x :in (unsigned a)) (y :out (unsigned b))) (assign y (resize (unsigned b) x)))";
    assert_eq!(
        semantic_error(unknown),
        SemanticErrorKind::WidthRelationUnknown
    );
    compile_source("(module m (generics (w :positive 1)) (ports (x :in (unsigned (+ w 1))) (y :out (unsigned w))) (assign y (truncate (unsigned w) x)))").unwrap();
}

#[test]
fn conversion_results_work_inside_binary_if_and_clocked_expressions() {
    let source = "(module m (ports (clk :in bit) (sel :in bit) (a :in (unsigned 8)) (b :in (unsigned 8)) (y :out (unsigned 9))) (reg r (unsigned 9) 0) (clocked clk (next r (+ (resize (unsigned 9) a) (resize (unsigned 9) b)))) (assign y (if sel r (resize (unsigned 9) a))))";
    compile_source(source).unwrap();
}

#[test]
fn vhdl_uses_numeric_resize_truncate_helpers_and_reinterpret_casts() {
    let resize = compile_source_to_vhdl(include_str!("../examples/resize_unsigned.glisp")).unwrap();
    assert!(resize.contains("resize(gl_p0_input, 16)"));
    assert!(!resize.contains("gl_truncate_unsigned"));
    let truncate = compile_source_to_vhdl(include_str!("../examples/truncate.glisp")).unwrap();
    assert_eq!(
        truncate.matches("function gl_truncate_unsigned(").count(),
        1
    );
    assert!(truncate.contains("value(value'low + size - 1 downto value'low)"));
    assert!(truncate.contains("gl_truncate_unsigned(gl_p0_input, 8)"));
    let reinterpret =
        compile_source_to_vhdl(include_str!("../examples/reinterpret.glisp")).unwrap();
    assert!(reinterpret.contains("signed(gl_p0_input)"));
    assert!(reinterpret.contains("unsigned(gl_s3_internal)"));
    let generic = compile_source_to_vhdl(include_str!("../examples/generic_resize.glisp")).unwrap();
    assert!(generic.contains("resize(gl_p0_input, (gl_g0 + 1))"));
}
