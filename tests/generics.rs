use gatelisp::{
    ConstExprAst, GenericKind, HardwareType, SemanticErrorKind, TypeExpr, WidthExpr,
    compile_source, compile_source_to_vhdl, parse_program,
};

#[test]
fn parser_builds_module_instance_and_target_generics() {
    let source = "(module child (generics (width :positive 8) (count :natural 0)) (ports (x :in (unsigned (+ width 1))))) (module top (generics (w :positive 4)) (ports (x :in (unsigned (+ w 1)))) (instance u child (generics (width w) (count (* w 2))) (ports (x x)))) (testbench tb (target child (generics (width 16))) (stimulus))";
    let program = parse_program(source).unwrap();
    assert_eq!(program.modules[0].value.generics.len(), 2);
    assert!(matches!(
        program.modules[0].value.ports[0].value.ty.value,
        TypeExpr::SymbolicUnsigned(_)
    ));
    let TypeExpr::SymbolicUnsigned(width) = &program.modules[0].value.ports[0].value.ty.value
    else {
        panic!("symbolic width expected")
    };
    assert!(matches!(width.value, ConstExprAst::Add(_, _)));
    let gatelisp::ModuleItem::Instance(instance) = &program.modules[1].value.items[0].value else {
        panic!("instance expected")
    };
    assert_eq!(instance.generics.len(), 2);
    assert_eq!(program.testbenches[0].value.target_generics.len(), 1);
}

#[test]
fn resolves_normalizes_and_substitutes_generics() {
    let source = "(module child (generics (width :positive 8)) (ports (x :in (unsigned width)) (y :out (unsigned width))) (assign y x)) (module top (generics (w :positive 4)) (ports (x :in (unsigned w)) (y :out (unsigned w))) (instance u child (generics (width (+ w 0))) (ports (x x) (y y))))";
    let typed = compile_source(source).unwrap();
    let child = &typed.modules[0];
    assert_eq!(child.generics[0].kind, GenericKind::Positive);
    assert_eq!(child.generics[0].default, 8);
    assert!(matches!(
        child.signals[0].ty,
        HardwareType::SymbolicUnsigned(WidthExpr::Generic(_))
    ));
    let instance = &typed.modules[1].instances[0];
    assert!(!instance.generic_bindings[0].uses_default);
    assert!(matches!(
        instance.generic_bindings[0].value,
        WidthExpr::Generic(_)
    ));
    assert_eq!(instance.connections[0].ty, typed.modules[1].signals[0].ty);
}

#[test]
fn checks_width_minimum_defaults_bindings_and_literals() {
    for (source, kind) in [
        (
            "(module m (generics (n :natural 0)) (ports (x :in (unsigned n))))",
            SemanticErrorKind::InvalidWidthExpression,
        ),
        (
            "(module m (generics (n :positive 0)) (ports))",
            SemanticErrorKind::InvalidGenericDefault,
        ),
        (
            "(module c (generics (n :positive 1)) (ports)) (module p (generics (n :natural 0)) (ports) (instance u c (generics (n n)) (ports)))",
            SemanticErrorKind::InvalidGenericActual,
        ),
        (
            "(module m (generics (w :positive 1)) (ports) (reg r (unsigned w) 255))",
            SemanticErrorKind::IntegerOutOfRange,
        ),
    ] {
        let gatelisp::CompileError::Semantic(error) = compile_source(source).unwrap_err() else {
            panic!("semantic error expected")
        };
        assert_eq!(error.kind, kind);
    }
    compile_source("(module m (generics (n :natural 0)) (ports (x :in (unsigned (+ n 1)))))")
        .unwrap();
}

#[test]
fn lowers_generic_clause_maps_symbolic_width_and_testbench_override() {
    let hierarchy =
        compile_source_to_vhdl(include_str!("../examples/generic_hierarchy.glisp")).unwrap();
    assert!(hierarchy.contains("generic ("));
    assert!(hierarchy.contains("gl_g0 : positive := 8"));
    assert!(hierarchy.contains("unsigned(gl_g0 - 1 downto 0)"));
    assert!(hierarchy.contains("generic map ("));
    assert!(hierarchy.contains("gl_g0 => gl_g1"));
    let testbench =
        compile_source_to_vhdl(include_str!("../examples/generic_testbench.glisp")).unwrap();
    assert!(testbench.contains("gl_g0 => 16"));
    assert!(testbench.contains("gl_tb_s1_input : unsigned(15 downto 0)"));
    assert!(testbench.contains("resize(unsigned'(x\"0000000000001234\"), 16)"));
}
