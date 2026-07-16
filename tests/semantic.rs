use gatelisp::{
    BinaryOp, CompileError, HardwareType as T, ModuleId, SemanticError, SemanticErrorKind as K,
    SignalId, SignalKind, TypedExprKind, analyze_program, compile_source, parse_program,
};

fn analyze(source: &str) -> Result<gatelisp::TypedProgram, SemanticError> {
    analyze_program(&parse_program(source).unwrap())
}
fn error(source: &str) -> SemanticError {
    analyze(source).unwrap_err()
}

#[test]
fn builds_hir_and_assigns_stable_ids() {
    let hir = analyze("(module a (ports (i :in bit) (o :out bit)) (wire w bit) (reg r bit 0) (assign w i) (assign o r)) (module b (ports))").unwrap();
    assert_eq!(hir.modules[0].id, ModuleId(0));
    assert_eq!(hir.modules[1].id, ModuleId(1));
    assert_eq!(
        hir.modules[0]
            .signals
            .iter()
            .map(|s| s.id)
            .collect::<Vec<_>>(),
        [SignalId(0), SignalId(1), SignalId(2), SignalId(3)]
    );
    assert!(matches!(hir.modules[0].signals[0].kind, SignalKind::Input));
    assert!(matches!(hir.modules[0].signals[1].kind, SignalKind::Output));
    assert!(matches!(hir.modules[0].signals[2].kind, SignalKind::Wire));
    assert!(matches!(
        hir.modules[0].signals[3].kind,
        SignalKind::Register { initial: Some(_) }
    ));
    assert_eq!(hir.modules[0].assignments[0].target, SignalId(2));
    assert!(matches!(
        hir.modules[0].assignments[0].value.kind,
        TypedExprKind::Signal(SignalId(0))
    ));
}

#[test]
fn supports_all_operators_and_types() {
    let source = "(module m (ports (b1 :in bit) (b2 :in bit) (u1 :in (unsigned 8)) (u2 :in (unsigned 8)) (s1 :in (signed 8)) (s2 :in (signed 8)) (bo :out bit) (uo :out (unsigned 8)) (so :out (signed 8))) (wire w1 bit) (wire w2 bit) (wire w3 bit) (wire w4 bit) (wire w5 bit) (wire w6 bit) (wire w7 bit) (assign w1 (and b1 b2)) (assign w2 (or b1 b2)) (assign w3 (xor b1 b2)) (assign w4 (= u1 u2)) (assign w5 (/= s1 s2)) (assign w6 (< u1 u2)) (assign w7 (>= s1 s2)) (assign bo (if b1 (not b2) b2)) (assign uo (+ u1 1)) (assign so (- s1 s2)))";
    let hir = analyze(source).unwrap();
    let assignments = &hir.modules[0].assignments;
    for (index, op) in [
        BinaryOp::And,
        BinaryOp::Or,
        BinaryOp::Xor,
        BinaryOp::Equal,
        BinaryOp::NotEqual,
        BinaryOp::LessThan,
        BinaryOp::GreaterEqual,
    ]
    .into_iter()
    .enumerate()
    {
        assert!(
            matches!(assignments[index].value.kind, TypedExprKind::Binary { op: actual, .. } if actual == op)
        );
    }
    assert_eq!(assignments[7].value.ty, T::Bit);
    assert_eq!(assignments[8].value.ty, T::Unsigned(8));
    assert_eq!(assignments[9].value.ty, T::Signed(8));
}

#[test]
fn propagates_expected_types_through_literals_and_if() {
    let source = "(module m (ports (sel :in bit) (u :in (unsigned 8)) (o1 :out (unsigned 8)) (o2 :out (unsigned 8)) (o3 :out (unsigned 8))) (assign o1 (+ 1 u)) (assign o2 (+ 1 2)) (assign o3 (if sel 0 255)))";
    let hir = analyze(source).unwrap();
    for assign in &hir.modules[0].assignments {
        assert_eq!(assign.value.ty, T::Unsigned(8));
    }
}

#[test]
fn accepts_integer_boundaries_and_wide_types() {
    let source = "(module m (ports) (reg b0 bit 0) (reg b1 bit 1) (reg umax (unsigned 8) 255) (reg smin (signed 8) -128) (reg smax (signed 8) 127) (reg wideu (unsigned 100) 9223372036854775807) (reg wides (signed 100) -9223372036854775808))";
    analyze(source).unwrap();
}

#[test]
fn detects_duplicate_modules_and_signals_with_related_spans() {
    for (source, kind) in [
        ("(module m (ports)) (module m (ports))", K::DuplicateModule),
        (
            "(module m (ports (x :in bit) (x :out bit)))",
            K::DuplicateSignal,
        ),
        (
            "(module m (ports (x :in bit)) (wire x bit))",
            K::DuplicateSignal,
        ),
        (
            "(module m (ports) (wire x bit) (reg x bit))",
            K::DuplicateSignal,
        ),
    ] {
        let error = error(source);
        assert_eq!(error.kind, kind);
        assert!(error.related_span.is_some());
    }
    analyze("(module m (ports (value :in bit) (Value :out bit)))").unwrap();
}

#[test]
fn detects_resolution_and_drive_errors() {
    for (source, kind) in [
        (
            "(module m (ports (o :out bit)) (assign o missing))",
            K::UndeclaredSignal,
        ),
        (
            "(module m (ports) (assign missing 0))",
            K::UndeclaredAssignTarget,
        ),
        (
            "(module m (ports (i :in bit)) (assign i 0))",
            K::AssignToInput,
        ),
        (
            "(module m (ports) (reg r bit) (assign r 0))",
            K::AssignToRegister,
        ),
        (
            "(module m (ports) (wire w bit) (assign w 0) (assign w 1))",
            K::MultipleDrivers,
        ),
        (
            "(module m (ports (o :out bit)) (assign o 0) (assign o 1))",
            K::MultipleDrivers,
        ),
    ] {
        assert_eq!(error(source).kind, kind, "{source}");
    }
}

#[test]
fn detects_operator_errors() {
    for (source, kind) in [
        (
            "(module m (ports (o :out bit)) (assign o (foo 1)))",
            K::UnknownOperator,
        ),
        (
            "(module m (ports (o :out bit)) (assign o (not)))",
            K::WrongArgumentCount,
        ),
        (
            "(module m (ports (o :out bit)) (assign o (and 0)))",
            K::WrongArgumentCount,
        ),
        (
            "(module m (ports (o :out bit)) (assign o (if 1 0)))",
            K::WrongArgumentCount,
        ),
        (
            "(module m (ports (i :in bit) (o :out bit)) (assign o (+ i i)))",
            K::BitArithmetic,
        ),
        (
            "(module m (ports (i :in bit) (o :out bit)) (assign o (< i i)))",
            K::BitOrdering,
        ),
    ] {
        assert_eq!(error(source).kind, kind, "{source}");
    }
}

#[test]
fn detects_type_errors() {
    for (source, kind) in [
        (
            "(module m (ports (u :in (unsigned 8)) (s :in (signed 8)) (o :out (unsigned 8))) (assign o (and u s)))",
            K::TypeMismatch,
        ),
        (
            "(module m (ports (a :in (unsigned 8)) (b :in (unsigned 9)) (o :out (unsigned 8))) (assign o (+ a b)))",
            K::TypeMismatch,
        ),
        (
            "(module m (ports (i :in bit) (o :out (unsigned 1))) (assign o i))",
            K::AssignTypeMismatch,
        ),
        (
            "(module m (ports (u :in (unsigned 8)) (o :out bit)) (assign o (+ u 1)))",
            K::AssignTypeMismatch,
        ),
        (
            "(module m (ports (c :in (unsigned 1)) (o :out bit)) (assign o (if c 0 1)))",
            K::IfConditionType,
        ),
        (
            "(module m (ports (c :in bit) (u :in (unsigned 8)) (s :in (signed 8)) (o :out (unsigned 8))) (assign o (if c u s)))",
            K::IfBranchType,
        ),
    ] {
        assert_eq!(error(source).kind, kind, "{source}");
    }
}

#[test]
fn detects_literal_range_and_inference_errors() {
    for (source, kind) in [
        ("(module m (ports) (reg x bit 2))", K::IntegerOutOfRange),
        (
            "(module m (ports) (reg x (unsigned 8) -1))",
            K::IntegerOutOfRange,
        ),
        (
            "(module m (ports) (reg x (unsigned 8) 256))",
            K::IntegerOutOfRange,
        ),
        (
            "(module m (ports) (reg x (signed 8) 128))",
            K::IntegerOutOfRange,
        ),
        (
            "(module m (ports) (reg x (signed 8) -129))",
            K::IntegerOutOfRange,
        ),
        (
            "(module m (ports (o :out bit)) (assign o (= 1 2)))",
            K::CannotInferIntegerType,
        ),
    ] {
        assert_eq!(error(source).kind, kind, "{source}");
    }
}

#[test]
fn resolves_forward_output_and_register_references() {
    let source =
        "(module m (ports (o :out bit)) (assign o (and later r)) (wire later bit) (reg r bit 0))";
    analyze(source).unwrap();
    let output_ref =
        "(module m (ports (a :in bit) (o1 :out bit) (o2 :out bit)) (assign o1 a) (assign o2 o1))";
    analyze(output_ref).unwrap();
}

#[test]
fn compile_errors_preserve_frontend_layers_and_spans() {
    assert!(matches!(compile_source("("), Err(CompileError::Reader(_))));
    assert!(matches!(
        compile_source("symbol"),
        Err(CompileError::GateSyntax(_))
    ));
    let error = compile_source("(module m (ports (o :out bit))\n (assign o missing))").unwrap_err();
    assert!(matches!(error, CompileError::Semantic(_)));
    assert_eq!(
        (error.span().start.line, error.span().start.column),
        (2, 12)
    );
}
