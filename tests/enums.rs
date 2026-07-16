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
fn declares_resolves_types_members_and_conversions() {
    let typed = compile_source(include_str!("../examples/enum_state.glisp")).unwrap();
    assert_eq!(typed.enums.len(), 1);
    assert!(matches!(
        typed.modules[0].signals[1].ty,
        HardwareType::Enum(_, 2)
    ));
    let conversion = compile_source(include_str!("../examples/enum_conversion.glisp")).unwrap();
    assert!(matches!(
        conversion.modules[0].assignments[0].value.kind,
        TypedExprKind::EnumFromBits { .. }
    ));
    assert!(matches!(
        conversion.modules[0].assignments[1].value.kind,
        TypedExprKind::EnumToBits { .. }
    ));
}

#[test]
fn rejects_invalid_declarations_types_and_operations() {
    assert!(compile_source("(enum E :width 0 (A 0)) (module m (ports))").is_err());
    assert!(compile_source("(enum E :width 2 (A 4)) (module m (ports))").is_err());
    assert_eq!(
        error("(enum E :width 2 (A 0) (A 1)) (module m (ports (x :out E)) (assign x E.A))"),
        SemanticErrorKind::DuplicateEnumMember
    );
    assert_eq!(
        error("(enum E :width 2 (A 0)) (module m (ports (x :out E)) (assign x 1))"),
        SemanticErrorKind::IntegerOutOfRange
    );
    assert_eq!(
        error(
            "(enum E :width 2 (A 0)) (module m (ports (x :out E)) (assign x (enum-to-bits E.A)))"
        ),
        SemanticErrorKind::AssignTypeMismatch
    );
    assert_eq!(
        error("(enum E :width 2 (A 0)) (module m (ports (x :out bit)) (assign x (= E.A 0)))"),
        SemanticErrorKind::IntegerOutOfRange
    );
}

#[test]
fn lowers_enum_storage_and_zero_cost_conversions() {
    let vhdl = compile_source_to_vhdl(include_str!("../examples/enum_state.glisp")).unwrap();
    assert!(vhdl.contains("unsigned(1 downto 0)"));
    assert!(vhdl.contains("to_unsigned(0, 2)"));
    assert!(vhdl.contains("gl_enum_state_idle"));
    let conversion =
        compile_source_to_vhdl(include_str!("../examples/enum_conversion.glisp")).unwrap();
    assert!(conversion.contains("gl_p0_bits"));
    assert!(!conversion.contains("gl_enum_opcode_nop"));
}
