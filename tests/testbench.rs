use gatelisp::{
    GateParseErrorKind as G, SemanticErrorKind as S, SignalId, SimulationOptions, SimulationStage,
    TestbenchStmt, TimeUnit, TypedTestbenchStmt, build_program, compile_source,
    compile_source_to_vhdl, parse_document, parse_program, simulate_source,
};

fn gate_error(source: &str) -> G {
    build_program(&parse_document(source).unwrap())
        .unwrap_err()
        .kind
}
fn semantic_error(source: &str) -> S {
    match compile_source(source).unwrap_err() {
        gatelisp::CompileError::Semantic(error) => error.kind,
        _ => panic!("semantic error expected"),
    }
}

#[test]
fn parses_testbench_forms_and_mixed_top_level() {
    let source = "(module m (ports (clk :in bit) (a :in bit) (q :out bit))) (testbench t (target m) (clock clk 10 ns) (stimulus (drive a 1) (wait 2 us) (wait-rising clk) (wait-rising clk 3) (assert (= q 1) \"ok\")))";
    let program = parse_program(source).unwrap();
    assert_eq!(program.modules.len(), 1);
    assert_eq!(program.testbenches.len(), 1);
    let tb = &program.testbenches[0].value;
    assert_eq!(tb.target.name, "m");
    assert_eq!(tb.clocks[0].value.period.unit, TimeUnit::Nanosecond);
    assert!(matches!(tb.stimulus[0].value, TestbenchStmt::Drive { .. }));
    assert!(matches!(
        tb.stimulus[2].value,
        TestbenchStmt::WaitRising { count: 1, .. }
    ));
}

#[test]
fn rejects_invalid_testbench_syntax() {
    for (source, kind) in [
        ("(testbench)", G::InvalidTestbenchName),
        ("(testbench t (stimulus))", G::InvalidTargetPosition),
        (
            "(testbench t (target m) (target m) (stimulus))",
            G::DuplicateTarget,
        ),
        (
            "(testbench t (target :m) (stimulus))",
            G::InvalidTargetModule,
        ),
        (
            "(testbench t (target m) (clock c 0 ns) (stimulus))",
            G::InvalidTime,
        ),
        (
            "(testbench t (target m) (clock c 1 tick) (stimulus))",
            G::UnknownTimeUnit,
        ),
        ("(testbench t (target m))", G::MissingStimulus),
        (
            "(testbench t (target m) (stimulus) (stimulus))",
            G::DuplicateStimulus,
        ),
        (
            "(testbench t (target m) (stimulus (drive :x 0)))",
            G::InvalidDriveTarget,
        ),
        (
            "(testbench t (target m) (stimulus (wait 0 ns)))",
            G::InvalidTime,
        ),
        (
            "(testbench t (target m) (stimulus (wait-rising c 0)))",
            G::InvalidWaitRising,
        ),
        (
            "(testbench t (target m) (stimulus (assert 1 not-string)))",
            G::InvalidAssertMessage,
        ),
    ] {
        assert_eq!(gate_error(source), kind, "{source}");
    }
}

#[test]
fn resolves_target_clocks_drives_and_assert_references() {
    let hir = compile_source(include_str!("../examples/counter_test.glisp")).unwrap();
    let tb = &hir.testbenches[0];
    assert_eq!(tb.target, hir.modules[0].id);
    assert_eq!(tb.clocks[0].signal, SignalId(0));
    assert!(matches!(
        tb.statements[0],
        TypedTestbenchStmt::Drive {
            target: SignalId(1),
            ..
        }
    ));
    assert!(matches!(
        tb.statements[6],
        TypedTestbenchStmt::Assert { .. }
    ));
}

#[test]
fn rejects_invalid_testbench_semantics() {
    for (source, kind) in [
        (
            "(module m (ports)) (testbench t (target missing) (stimulus))",
            S::UnknownTestbenchTarget,
        ),
        (
            "(module m (ports)) (testbench t (target m) (stimulus)) (testbench t (target m) (stimulus))",
            S::DuplicateTestbench,
        ),
        (
            "(module m (ports (o :out bit))) (testbench t (target m) (clock o 1 ns) (stimulus))",
            S::InvalidTestbenchClock,
        ),
        (
            "(module m (ports (c :in bit))) (testbench t (target m) (clock c 1 ns) (clock c 2 ns) (stimulus))",
            S::DuplicateTestbenchClock,
        ),
        (
            "(module m (ports (c :in bit))) (testbench t (target m) (clock c 1 ns) (stimulus (drive c 0)))",
            S::DriveClock,
        ),
        (
            "(module m (ports (o :out bit))) (testbench t (target m) (stimulus (drive o 0)))",
            S::InvalidDriveTarget,
        ),
        (
            "(module m (ports (c :in bit))) (testbench t (target m) (stimulus (wait-rising c)))",
            S::UnknownWaitClock,
        ),
        (
            "(module m (ports (u :out (unsigned 8)))) (testbench t (target m) (stimulus (assert u \"bad\")))",
            S::InvalidAssertType,
        ),
    ] {
        assert_eq!(semantic_error(source), kind, "{source}");
    }
}

#[test]
fn lowers_complete_vhdl_testbench() {
    let vhdl = compile_source_to_vhdl(include_str!("../examples/counter_test.glisp")).unwrap();
    for text in [
        "entity gl_tb0_counter_test is",
        "architecture sim",
        "gl_dut : entity work.gl_m0_counter",
        "gl_p0_clk => gl_tb_s0_clk",
        "constant gl_clk_period_0 : time := 10 ns;",
        "gl_clock_0 : process",
        "wait until rising_edge(gl_tb_s0_clk);",
        "wait for 1 fs;",
        "severity error;",
        "GateLisp testbench passed: counter-test",
        "stop;",
    ] {
        assert!(vhdl.contains(text), "missing {text}");
    }
}

#[test]
fn simulation_api_reports_missing_ghdl_and_selection_errors() {
    let source = include_str!("../examples/and_gate_test.glisp");
    let missing = SimulationOptions {
        ghdl_path: Some("definitely-missing-ghdl".into()),
        ..SimulationOptions::default()
    };
    assert_eq!(
        simulate_source(source, &missing).unwrap_err().stage,
        SimulationStage::Detection
    );
    let selection = SimulationOptions {
        testbench: Some("missing".into()),
        ..SimulationOptions::default()
    };
    assert_eq!(
        simulate_source(source, &selection).unwrap_err().stage,
        SimulationStage::Selection
    );
    assert_eq!(
        simulate_source("(module m (ports))", &SimulationOptions::default())
            .unwrap_err()
            .stage,
        SimulationStage::Selection
    );
}
