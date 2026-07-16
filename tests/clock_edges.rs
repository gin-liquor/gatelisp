use gatelisp::{
    ClockEdge, FrontendError, GateParseErrorKind, compile_source, compile_source_to_vhdl,
    parse_program,
};

#[test]
fn defaults_to_rising_and_resolves_explicit_edges() {
    let typed = compile_source("(module m (ports (clk :in bit)) (reg a bit 0) (reg b bit 0) (clocked clk (next a 1)) (clocked (falling clk) (next b 1)))").unwrap();
    assert_eq!(typed.modules[0].clocked_blocks[0].edge, ClockEdge::Rising);
    assert_eq!(typed.modules[0].clocked_blocks[1].edge, ClockEdge::Falling);
}

#[test]
fn rejects_malformed_or_expression_clock_specs() {
    for source in [
        "(module m (ports (clk :in bit)) (reg r bit 0) (clocked (rising) (next r 0)))",
        "(module m (ports (clk :in bit)) (reg r bit 0) (clocked (falling clk extra) (next r 0)))",
        "(module m (ports (clk :in bit)) (reg r bit 0) (clocked (unknown clk) (next r 0)))",
        "(module m (ports (clk :in bit) (en :in bit)) (reg r bit 0) (clocked (falling (and clk en)) (next r 0)))",
    ] {
        let Err(FrontendError::GateSyntax(error)) = parse_program(source) else {
            panic!("gate syntax error expected")
        };
        assert_eq!(error.kind, GateParseErrorKind::InvalidClockName);
    }
}

#[test]
fn lowers_rising_falling_and_resets() {
    let vhdl = compile_source_to_vhdl(include_str!("../examples/clock_edges.glisp")).unwrap();
    assert!(vhdl.contains("if rising_edge(gl_p0_clk) then"));
    assert!(vhdl.contains("if falling_edge(gl_p0_clk) then"));
    assert!(vhdl.contains("process(gl_p0_clk, gl_p1_reset)"));
}
