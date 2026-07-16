use gatelisp::{
    Expr, FrontendError, GateParseErrorKind as K, ModuleItem, PortDirection, TypeExpr,
    build_program, parse_document, parse_program,
};

fn error(source: &str) -> gatelisp::GateParseError {
    let sexprs = parse_document(source).unwrap();
    build_program(&sexprs).unwrap_err()
}

#[test]
fn parses_minimal_empty_ports_and_multiple_modules() {
    let program = parse_program("(module a (ports)) (module b (ports))").unwrap();
    assert_eq!(program.modules.len(), 2);
    assert!(program.modules[0].value.ports.is_empty());
}

#[test]
fn parses_ports_and_all_types() {
    let program =
        parse_program("(module m (ports (a :in bit) (b :out (unsigned 8)) (c :in (signed 16))))")
            .unwrap();
    let ports = &program.modules[0].value.ports;
    assert_eq!(ports[0].value.direction, PortDirection::Input);
    assert_eq!(ports[0].value.ty.value, TypeExpr::Bit);
    assert_eq!(ports[1].value.ty.value, TypeExpr::Unsigned(8));
    assert_eq!(ports[2].value.ty.value, TypeExpr::Signed(16));
}

#[test]
fn parses_declarations_assigns_and_nested_expressions() {
    let source = "(module m (ports) (wire w bit) (reg r (unsigned 8)) (reg q (signed 9) -1) (assign w (if s (+ r 1) (foo))))";
    let program = parse_program(source).unwrap();
    let items = &program.modules[0].value.items;
    assert!(matches!(items[0].value, ModuleItem::Wire(_)));
    assert!(matches!(items[1].value, ModuleItem::Register(ref reg) if reg.initial.is_none()));
    assert!(
        matches!(items[2].value, ModuleItem::Register(ref reg) if matches!(reg.initial.as_ref().map(|v| &v.value), Some(Expr::Integer(-1))))
    );
    assert!(
        matches!(items[3].value, ModuleItem::Assign(ref assign) if matches!(assign.value.value, Expr::Call { .. }))
    );
}

#[test]
fn parses_and_gate_and_integer_expression() {
    let program = parse_program(include_str!("../examples/and_gate.glisp")).unwrap();
    assert_eq!(program.modules[0].value.name.name, "and-gate");
    let integer = parse_program("(module m (ports) (assign x 123))").unwrap();
    assert!(
        matches!(integer.modules[0].value.items[0].value, ModuleItem::Assign(ref value) if matches!(value.value.value, Expr::Integer(123)))
    );
}

#[test]
fn rejects_top_level_and_module_shape_errors() {
    for (source, kind) in [
        ("x", K::TopLevelNotModule),
        ("()", K::EmptyTopLevelList),
        ("(other x)", K::TopLevelNotModule),
        ("(module)", K::MissingModuleName),
        ("(module :m (ports))", K::InvalidModuleName),
        ("(module m)", K::MissingPorts),
        ("(module m (wire x bit))", K::InvalidPortsPosition),
        ("(module m (ports) (ports))", K::InvalidPortsPosition),
    ] {
        assert_eq!(error(source).kind, kind, "{source}");
    }
}

#[test]
fn rejects_bad_ports_and_types() {
    for (source, kind) in [
        ("(module m (ports (a :in)))", K::InvalidPort),
        ("(module m (ports (:a :in bit)))", K::InvalidPortName),
        ("(module m (ports (a :inout bit)))", K::InvalidPortDirection),
        ("(module m (ports (a :in mystery)))", K::InvalidType),
        ("(module m (ports (a :in (unsigned))))", K::InvalidType),
        ("(module m (ports (a :in (unsigned 8 9))))", K::InvalidType),
        ("(module m (ports (a :in (unsigned 0))))", K::ZeroWidth),
        ("(module m (ports (a :in (signed -1))))", K::NegativeWidth),
        (
            "(module m (ports (a :in (unsigned 4294967296))))",
            K::WidthOutOfRange,
        ),
    ] {
        assert_eq!(error(source).kind, kind, "{source}");
    }
}

#[test]
fn rejects_bad_module_items() {
    for (source, kind) in [
        ("(module m (ports) (wire x))", K::InvalidWire),
        ("(module m (ports) (reg x))", K::InvalidRegister),
        ("(module m (ports) (reg x bit 0 1))", K::InvalidRegister),
        ("(module m (ports) (assign x))", K::InvalidAssign),
        ("(module m (ports) (assign :x 1))", K::InvalidAssignTarget),
        ("(module m (ports) (clocked x))", K::UnknownModuleItem),
    ] {
        assert_eq!(error(source).kind, kind, "{source}");
    }
}

#[test]
fn rejects_invalid_expressions() {
    for (source, kind) in [
        ("(module m (ports) (assign x ()))", K::EmptyExpression),
        ("(module m (ports) (assign x (1 a)))", K::InvalidCallTarget),
        ("(module m (ports) (assign x :bad))", K::InvalidExpression),
        (
            "(module m (ports) (assign x \"bad\"))",
            K::InvalidExpression,
        ),
    ] {
        assert_eq!(error(source).kind, kind, "{source}");
    }
}

#[test]
fn errors_have_relevant_spans_and_frontend_layers_are_distinct() {
    let syntax = error("(module m (ports)\n  (assign x ()))");
    assert_eq!((syntax.span.start.line, syntax.span.start.column), (2, 13));
    assert!(matches!(parse_program("("), Err(FrontendError::Reader(_))));
    assert!(matches!(
        parse_program("symbol"),
        Err(FrontendError::GateSyntax(_))
    ));
}
