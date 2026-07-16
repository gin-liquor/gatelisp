use gatelisp::{
    ClockEdge, ClockedBlockId, CompileError, GateParseError, GateParseErrorKind as G, ModuleItem,
    ResetKind, SemanticError, SemanticErrorKind as S, SignalId, SignalKind, TypedExprKind,
    analyze_program, build_program, compile_source, parse_document, parse_program,
};

fn gate_error(source: &str) -> GateParseError {
    build_program(&parse_document(source).unwrap()).unwrap_err()
}
fn semantic_error(source: &str) -> SemanticError {
    analyze_program(&parse_program(source).unwrap()).unwrap_err()
}

#[test]
fn parser_builds_sync_async_and_resetless_clocked_ast() {
    let source = "(module m (ports (clk :in bit) (rst :in bit)) (reg a bit) (reg b bit) (clocked clk (reset :sync rst (next a 0) (next b 1)) (next a 1) (next b 0)) (clocked clk (next a 0)))";
    let program = parse_program(source).unwrap();
    let ModuleItem::Clocked(first) = &program.modules[0].value.items[2].value else {
        panic!("expected clocked")
    };
    assert_eq!(first.clock.name, "clk");
    assert_eq!(first.updates.len(), 2);
    assert_eq!(
        first.reset.as_ref().unwrap().value.kind,
        ResetKind::Synchronous
    );
    let async_program = parse_program("(module m (ports (clk :in bit) (rst :in bit)) (reg r bit) (clocked clk (reset :async rst (next r 0)) (next r 1)))").unwrap();
    let ModuleItem::Clocked(block) = &async_program.modules[0].value.items[1].value else {
        panic!("expected clocked")
    };
    assert_eq!(
        block.reset.as_ref().unwrap().value.kind,
        ResetKind::Asynchronous
    );
}

#[test]
fn parser_rejects_invalid_clocked_forms() {
    for (source, kind) in [
        ("(module m (ports) (clocked))", G::InvalidClocked),
        (
            "(module m (ports) (clocked :clk (next r 0)))",
            G::InvalidClockName,
        ),
        ("(module m (ports) (clocked clk))", G::EmptyClocked),
        (
            "(module m (ports) (clocked clk (assign x 0)))",
            G::UnknownClockedItem,
        ),
        (
            "(module m (ports) (clocked clk (reset :sync r (next x 0)) (reset :sync r (next x 0)) (next x 0)))",
            G::MultipleResets,
        ),
        (
            "(module m (ports) (clocked clk (next x 0) (reset :sync r (next x 0))))",
            G::ResetAfterNext,
        ),
        (
            "(module m (ports) (clocked clk (reset sync r (next x 0)) (next x 0)))",
            G::InvalidResetKind,
        ),
        (
            "(module m (ports) (clocked clk (reset :low r (next x 0)) (next x 0)))",
            G::UnknownResetKind,
        ),
        (
            "(module m (ports) (clocked clk (reset :sync :r (next x 0)) (next x 0)))",
            G::InvalidResetSignal,
        ),
        (
            "(module m (ports) (clocked clk (reset :sync r) (next x 0)))",
            G::EmptyReset,
        ),
        (
            "(module m (ports) (clocked clk (reset :sync r (assign x 0)) (next x 0)))",
            G::InvalidResetItem,
        ),
        ("(module m (ports) (clocked clk (next x)))", G::InvalidNext),
        (
            "(module m (ports) (clocked clk (next x 0 1)))",
            G::InvalidNext,
        ),
        (
            "(module m (ports) (clocked clk (next :x 0)))",
            G::InvalidNextTarget,
        ),
    ] {
        assert_eq!(gate_error(source).kind, kind, "{source}");
    }
}

#[test]
fn hir_resolves_clock_reset_updates_and_preserves_initial_value() {
    let hir = compile_source(include_str!("../examples/counter.glisp")).unwrap();
    let module = &hir.modules[0];
    let block = &module.clocked_blocks[0];
    assert_eq!(block.id, ClockedBlockId(0));
    assert_eq!(block.clock, SignalId(0));
    assert_eq!(block.edge, ClockEdge::Rising);
    let reset = block.reset.as_ref().unwrap();
    assert_eq!(reset.kind, ResetKind::Synchronous);
    assert_eq!(reset.signal, SignalId(1));
    assert_eq!(block.updates[0].target, SignalId(4));
    assert_eq!(reset.updates[0].target, SignalId(4));
    assert!(matches!(
        module.signals[4].kind,
        SignalKind::Register { initial: Some(_) }
    ));
    assert!(matches!(
        block.updates[0].value.kind,
        TypedExprKind::If { .. }
    ));
}

#[test]
fn supports_multiple_blocks_same_clock_forward_regs_and_cross_domain_reads() {
    let source = "(module m (ports (a :in bit) (b :in bit)) (clocked a (next x y)) (clocked a (next y x)) (clocked b (next z x)) (reg x bit) (reg y bit) (reg z bit))";
    let hir = compile_source(source).unwrap();
    assert_eq!(hir.modules[0].clocked_blocks.len(), 3);
}

#[test]
fn rejects_invalid_clock_and_reset_sources() {
    for (source, kind) in [
        (
            "(module m (ports) (reg r bit) (clocked missing (next r 0)))",
            S::UndeclaredClock,
        ),
        (
            "(module m (ports (clk :out bit)) (reg r bit) (clocked clk (next r 0)))",
            S::InvalidClockSource,
        ),
        (
            "(module m (ports) (wire clk bit) (reg r bit) (clocked clk (next r 0)))",
            S::InvalidClockSource,
        ),
        (
            "(module m (ports) (reg clk bit) (reg r bit) (clocked clk (next r 0)))",
            S::InvalidClockSource,
        ),
        (
            "(module m (ports (clk :in (unsigned 1))) (reg r bit) (clocked clk (next r 0)))",
            S::InvalidClockType,
        ),
        (
            "(module m (ports (clk :in bit)) (reg r bit) (clocked clk (reset :sync missing (next r 0)) (next r 0)))",
            S::UndeclaredReset,
        ),
        (
            "(module m (ports (clk :in bit) (rst :out bit)) (reg r bit) (clocked clk (reset :sync rst (next r 0)) (next r 0)))",
            S::InvalidResetSource,
        ),
        (
            "(module m (ports (clk :in bit) (rst :in (unsigned 1))) (reg r bit) (clocked clk (reset :sync rst (next r 0)) (next r 0)))",
            S::InvalidResetType,
        ),
    ] {
        assert_eq!(semantic_error(source).kind, kind, "{source}");
    }
}

#[test]
fn rejects_invalid_next_targets_duplicates_and_multiple_blocks() {
    for (source, kind) in [
        (
            "(module m (ports (clk :in bit)) (clocked clk (next missing 0)))",
            S::UndeclaredNextTarget,
        ),
        (
            "(module m (ports (clk :in bit) (x :in bit)) (clocked clk (next x 0)))",
            S::NextTargetNotRegister,
        ),
        (
            "(module m (ports (clk :in bit) (x :out bit)) (clocked clk (next x 0)))",
            S::NextTargetNotRegister,
        ),
        (
            "(module m (ports (clk :in bit)) (wire x bit) (clocked clk (next x 0)))",
            S::NextTargetNotRegister,
        ),
        (
            "(module m (ports (clk :in bit)) (reg x bit) (clocked clk (next x 0) (next x 1)))",
            S::DuplicateNext,
        ),
        (
            "(module m (ports (clk :in bit) (rst :in bit)) (reg x bit) (clocked clk (reset :sync rst (next x 0) (next x 1)) (next x 0)))",
            S::DuplicateNext,
        ),
        (
            "(module m (ports (clk :in bit)) (reg x bit) (clocked clk (next x 0)) (clocked clk (next x 1)))",
            S::RegisterMultipleClockedDrivers,
        ),
    ] {
        assert_eq!(semantic_error(source).kind, kind, "{source}");
    }
}

#[test]
fn rejects_next_types_ranges_and_reset_set_mismatches() {
    for (source, kind) in [
        (
            "(module m (ports (clk :in bit) (u :in (unsigned 8))) (reg x bit) (clocked clk (next x u)))",
            S::NextTypeMismatch,
        ),
        (
            "(module m (ports (clk :in bit)) (reg x bit) (clocked clk (next x 2)))",
            S::IntegerOutOfRange,
        ),
        (
            "(module m (ports (clk :in bit) (rst :in bit) (u :in (unsigned 8))) (reg x bit) (clocked clk (reset :sync rst (next x u)) (next x 0)))",
            S::NextTypeMismatch,
        ),
        (
            "(module m (ports (clk :in bit) (rst :in bit)) (reg x bit) (clocked clk (reset :sync rst (next x 2)) (next x 0)))",
            S::IntegerOutOfRange,
        ),
        (
            "(module m (ports (clk :in bit) (rst :in bit)) (reg x bit) (reg y bit) (clocked clk (reset :sync rst (next x 0)) (next x 0) (next y 0)))",
            S::ResetTargetMissing,
        ),
        (
            "(module m (ports (clk :in bit) (rst :in bit)) (reg x bit) (reg y bit) (clocked clk (reset :sync rst (next x 0) (next y 0)) (next x 0)))",
            S::ResetTargetExtra,
        ),
    ] {
        assert_eq!(semantic_error(source).kind, kind, "{source}");
    }
}

#[test]
fn multiple_driver_error_has_related_span_and_compile_layer() {
    let source = "(module m (ports (clk :in bit)) (reg x bit) (clocked clk (next x 0))\n(clocked clk (next x 1)))";
    let error = compile_source(source).unwrap_err();
    let CompileError::Semantic(error) = error else {
        panic!("expected semantic error")
    };
    assert_eq!(error.kind, S::RegisterMultipleClockedDrivers);
    assert!(error.related_span.is_some());
    assert_eq!(error.span.start.line, 2);
}
