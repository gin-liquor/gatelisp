use gatelisp::{
    GateParseErrorKind as G, InstanceId, ModuleItem, PortDirection, SemanticErrorKind as S,
    SignalId, build_program, compile_source, compile_source_to_vhdl, parse_document, parse_program,
};

fn gate_error(source: &str) -> G {
    build_program(&parse_document(source).unwrap())
        .unwrap_err()
        .kind
}
fn semantic_error(source: &str) -> S {
    match compile_source(source).unwrap_err() {
        gatelisp::CompileError::Semantic(error) => error.kind,
        other => panic!("expected semantic error: {other}"),
    }
}

#[test]
fn parser_builds_instances_and_connections() {
    let program = parse_program(
        "(module top (ports) (instance empty0 empty (ports))) (module empty (ports))",
    )
    .unwrap();
    let ModuleItem::Instance(instance) = &program.modules[0].value.items[0].value else {
        panic!("instance expected")
    };
    assert_eq!(instance.name.name, "empty0");
    assert_eq!(instance.module.name, "empty");
    assert!(instance.ports.is_empty());
    let program = parse_program("(module top (ports (a :in bit) (y :out bit)) (instance u child (ports (out y) (input a)))) (module child (ports (input :in bit) (out :out bit)))").unwrap();
    let ModuleItem::Instance(instance) = &program.modules[0].value.items[0].value else {
        panic!("instance expected")
    };
    assert_eq!(instance.ports.len(), 2);
}

#[test]
fn parser_rejects_malformed_instances() {
    for (source, kind) in [
        ("(module m (ports) (instance x))", G::InvalidInstance),
        (
            "(module m (ports) (instance :x child (ports)))",
            G::InvalidInstanceName,
        ),
        (
            "(module m (ports) (instance x :child (ports)))",
            G::InvalidInstanceModule,
        ),
        (
            "(module m (ports) (instance x child (generic)))",
            G::InvalidInstancePorts,
        ),
        (
            "(module m (ports) (instance x child (ports (a))))",
            G::InvalidPortConnection,
        ),
        (
            "(module m (ports) (instance x child (ports (:a b))))",
            G::InvalidFormalPort,
        ),
        (
            "(module m (ports) (instance x child (ports (a 1))))",
            G::InvalidActualSignal,
        ),
        (
            "(module m (ports) (instance x child (ports (a (not b)))))",
            G::InvalidActualSignal,
        ),
    ] {
        assert_eq!(gate_error(source), kind, "{source}");
    }
}

#[test]
fn resolves_ids_normalizes_port_order_and_forward_modules() {
    let hir = compile_source("(module top (ports (a :in bit) (y :out bit)) (instance u child (ports (out y) (input a)))) (module child (ports (input :in bit) (out :out bit)))").unwrap();
    let instance = &hir.modules[0].instances[0];
    assert_eq!(instance.id, InstanceId(0));
    assert_eq!(instance.target_module, hir.modules[1].id);
    assert_eq!(instance.connections[0].formal, SignalId(2));
    assert_eq!(instance.connections[0].actual, SignalId(0));
    assert_eq!(instance.connections[0].direction, PortDirection::Input);
    assert_eq!(hir.module_order, [hir.modules[1].id, hir.modules[0].id]);
}

#[test]
fn rejects_resolution_completeness_direction_and_type_errors() {
    for (source, kind) in [
        (
            "(module top (ports) (instance u missing (ports)))",
            S::UnknownInstanceModule,
        ),
        (
            "(module top (ports) (instance u child (ports)) (instance u child (ports))) (module child (ports))",
            S::DuplicateInstance,
        ),
        (
            "(module top (ports (a :in bit)) (instance u child (ports (bad a)))) (module child (ports (x :in bit)))",
            S::UnknownFormalPort,
        ),
        (
            "(module top (ports (a :in bit)) (instance u child (ports (x a) (x a)))) (module child (ports (x :in bit)))",
            S::DuplicateFormalPort,
        ),
        (
            "(module top (ports) (instance u child (ports))) (module child (ports (x :in bit)))",
            S::MissingFormalPort,
        ),
        (
            "(module top (ports) (instance u child (ports (x missing)))) (module child (ports (x :in bit)))",
            S::UnknownActualSignal,
        ),
        (
            "(module top (ports (a :in (unsigned 1))) (instance u child (ports (x a)))) (module child (ports (x :in bit)))",
            S::InstanceInputTypeMismatch,
        ),
        (
            "(module top (ports (a :in bit)) (instance u child (ports (y a)))) (module child (ports (y :out bit)))",
            S::InvalidInstanceOutputTarget,
        ),
        (
            "(module top (ports) (reg r bit) (instance u child (ports (y r)))) (module child (ports (y :out bit)))",
            S::InvalidInstanceOutputTarget,
        ),
    ] {
        assert_eq!(semantic_error(source), kind, "{source}");
    }
}

#[test]
fn instance_outputs_participate_in_driver_checks() {
    let assign = "(module top (ports (a :in bit)) (wire w bit) (assign w a) (instance u child (ports (y w)))) (module child (ports (y :out bit)))";
    assert_eq!(semantic_error(assign), S::InstanceMultipleDriver);
    let two = "(module top (ports) (wire w bit) (instance u0 child (ports (y w))) (instance u1 child (ports (y w)))) (module child (ports (y :out bit)))";
    assert_eq!(semantic_error(two), S::InstanceMultipleDriver);
    compile_source("(module top (ports (a :in bit)) (instance u0 child (ports (x a))) (instance u1 child (ports (x a)))) (module child (ports (x :in bit)))").unwrap();
}

#[test]
fn detects_direct_and_indirect_recursion() {
    assert_eq!(
        semantic_error("(module a (ports) (instance self a (ports)))"),
        S::RecursiveModule
    );
    assert_eq!(
        semantic_error(
            "(module a (ports) (instance b0 b (ports))) (module b (ports) (instance a0 a (ports)))"
        ),
        S::RecursiveModule
    );
}

#[test]
fn vhdl_uses_direct_instantiation_named_mapping_and_dependency_order() {
    let vhdl = compile_source_to_vhdl(include_str!("../examples/hierarchy.glisp")).unwrap();
    let child = vhdl.find("entity gl_m0_and_gate is").unwrap();
    let parent = vhdl.find("entity gl_m2_hierarchy_top is").unwrap();
    assert!(child < parent);
    assert!(vhdl.contains("gl_i0_and0 : entity work.gl_m0_and_gate"));
    assert!(vhdl.contains("gl_p0_a => gl_p5_a"));
    assert!(vhdl.contains("gl_p2_y => gl_s8_and_result"));
    assert!(vhdl.contains("gl_p4_output => gl_s7_y"));
}

#[test]
fn hierarchical_testbench_is_emitted_after_all_modules() {
    let vhdl = compile_source_to_vhdl(include_str!("../examples/hierarchy_test.glisp")).unwrap();
    assert!(
        vhdl.find("entity gl_m2_hierarchy_top").unwrap()
            < vhdl.find("entity gl_tb0_hierarchy_test").unwrap()
    );
}
