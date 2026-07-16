use gatelisp::{
    HardwareType, ModuleId, SignalId, SignalKind, Span, TypedExpr, TypedExprKind, TypedModule,
    TypedProgram, TypedSignal, VhdlBackendErrorKind, VhdlDesign, VhdlDesignUnit, VhdlEntity,
    VhdlIdentifier, compile_source_to_vhdl, lower_to_vhdl, render_vhdl,
};

fn compile_example(name: &str) -> String {
    let source = std::fs::read_to_string(format!("examples/{name}.glisp")).unwrap();
    compile_source_to_vhdl(&source).unwrap()
}

#[test]
fn generated_vhdl_matches_golden_files() {
    for (name, expected) in [
        ("and_gate", include_str!("golden/and_gate.expected.vhd")),
        (
            "typed_adder",
            include_str!("golden/typed_adder.expected.vhd"),
        ),
        ("counter", include_str!("golden/counter.expected.vhd")),
        (
            "async_counter",
            include_str!("golden/async_counter.expected.vhd"),
        ),
        ("nested_if", include_str!("golden/nested_if.expected.vhd")),
        (
            "swap_registers",
            include_str!("golden/swap_registers.expected.vhd"),
        ),
        ("hierarchy", include_str!("golden/hierarchy.expected.vhd")),
        (
            "hierarchy_test",
            include_str!("golden/hierarchy_test.expected.vhd"),
        ),
        (
            "register_child",
            include_str!("golden/register_child.expected.vhd"),
        ),
        (
            "generic_register",
            include_str!("golden/generic_register.expected.vhd"),
        ),
        (
            "generic_hierarchy",
            include_str!("golden/generic_hierarchy.expected.vhd"),
        ),
        (
            "generic_testbench",
            include_str!("golden/generic_testbench.expected.vhd"),
        ),
        (
            "resize_unsigned",
            include_str!("golden/resize_unsigned.expected.vhd"),
        ),
        (
            "resize_signed",
            include_str!("golden/resize_signed.expected.vhd"),
        ),
        ("truncate", include_str!("golden/truncate.expected.vhd")),
        (
            "reinterpret",
            include_str!("golden/reinterpret.expected.vhd"),
        ),
        ("wide_adder", include_str!("golden/wide_adder.expected.vhd")),
        (
            "generic_resize",
            include_str!("golden/generic_resize.expected.vhd"),
        ),
        (
            "conversion_test",
            include_str!("golden/conversion_test.expected.vhd"),
        ),
    ] {
        assert_eq!(
            compile_example(name).replace("\r\n", "\n"),
            expected.replace("\r\n", "\n"),
            "{name}"
        );
    }
}

#[test]
fn lowers_entity_ports_internal_outputs_and_processes() {
    let text = compile_example("and_gate");
    assert!(text.contains("entity gl_m0_and_gate is"));
    assert!(text.contains("gl_p0_a : in std_logic"));
    assert!(text.contains("gl_p2_y : out std_logic"));
    assert!(text.contains("signal gl_s2_y : std_logic;"));
    assert!(text.contains("gl_comb_0 : process(all)"));
    assert!(text.contains("gl_p2_y <= gl_s2_y;"));
}

#[test]
fn lowers_types_literals_operators_and_comparisons() {
    let text = compile_source_to_vhdl("(module Ops (ports (a :in (unsigned 8)) (b :in (unsigned 8)) (s :in (signed 64)) (o :out bit)) (wire u (unsigned 100)) (reg r (signed 64) -9223372036854775808) (assign o (and (= a b) (/= a b))))").unwrap();
    assert!(text.contains("unsigned(7 downto 0)"));
    assert!(text.contains("unsigned(99 downto 0)"));
    assert!(text.contains("signed(63 downto 0) := resize(signed'(x\"8000000000000000\"), 64)"));
    assert!(text.contains("gl_bool_to_sl((gl_p0_a = gl_p1_b))"));
    assert!(text.contains("gl_bool_to_sl((gl_p0_a /= gl_p1_b))"));
}

#[test]
fn lowers_nested_if_to_fully_assigned_temporary() {
    let text = compile_example("nested_if");
    assert!(text.contains("variable gl_tmp_0 : unsigned(7 downto 0);"));
    assert!(text.contains("if (gl_p0_select = '1') then"));
    assert!(text.contains("gl_tmp_0 := gl_p2_b;"));
    assert!(text.contains("gl_tmp_0 := gl_p3_c;"));
    assert!(text.contains("gl_s4_y <= (gl_p1_a + gl_tmp_0);"));
}

#[test]
fn lowers_sync_async_and_simultaneous_register_updates() {
    let sync = compile_example("counter");
    assert!(sync.contains("gl_seq_0 : process(gl_p0_clk)"));
    assert!(sync.contains("if rising_edge(gl_p0_clk) then"));
    assert!(sync.contains("if (gl_p1_rst = '1') then"));
    let async_text = compile_example("async_counter");
    assert!(async_text.contains("gl_seq_0 : process(gl_p0_clk, gl_p1_rst)"));
    assert!(async_text.contains("else\n      if rising_edge(gl_p0_clk) then"));
    let swap = compile_example("swap_registers");
    assert!(swap.contains("gl_s4_reg_a <= gl_s5_reg_b;"));
    assert!(swap.contains("gl_s5_reg_b <= gl_s4_reg_a;"));
}

#[test]
fn mangles_names_deterministically_and_keeps_modules_unique() {
    let source = "(module and-gate (ports (Value :in bit) (value :out bit)) (assign value Value)) (module λ (ports))";
    let first = compile_source_to_vhdl(source).unwrap();
    let second = compile_source_to_vhdl(source).unwrap();
    assert_eq!(first, second);
    assert!(first.contains("gl_m0_and_gate"));
    assert!(first.contains("gl_p0_value"));
    assert!(first.contains("gl_p1_value"));
    assert!(first.contains("gl_m1__"));
}

#[test]
fn formatter_handles_empty_entity_multiple_units_and_is_clean() {
    let design = VhdlDesign {
        units: vec![
            VhdlDesignUnit::Entity(VhdlEntity {
                name: VhdlIdentifier("gl_m0_empty".into()),
                generics: vec![],
                ports: vec![],
            }),
            VhdlDesignUnit::Entity(VhdlEntity {
                name: VhdlIdentifier("gl_m1_empty".into()),
                generics: vec![],
                ports: vec![],
            }),
        ],
    };
    let text = render_vhdl(&design);
    assert!(text.ends_with('\n'));
    assert!(text.lines().all(|line| !line.ends_with(' ')));
    assert_eq!(text, render_vhdl(&design));
}

#[test]
fn rejects_nonliteral_initializers_and_invalid_hir() {
    let error = compile_source_to_vhdl("(module m (ports (i :in bit)) (reg r bit i))").unwrap_err();
    assert!(matches!(error, gatelisp::CompileError::VhdlBackend(_)));
    let span = Span {
        start: gatelisp::Position {
            offset: 0,
            line: 1,
            column: 1,
        },
        end: gatelisp::Position {
            offset: 0,
            line: 1,
            column: 1,
        },
    };
    let invalid = TypedProgram {
        modules: vec![TypedModule {
            id: ModuleId(0),
            name: "bad".into(),
            name_span: span,
            generics: vec![],
            span,
            signals: vec![TypedSignal {
                id: SignalId(0),
                name: "bad".into(),
                kind: SignalKind::Wire,
                ty: HardwareType::Unsigned(0),
                declaration_span: span,
            }],
            assignments: vec![],
            clocked_blocks: vec![],
            instances: vec![],
        }],
        testbenches: vec![],
        module_order: vec![ModuleId(0)],
    };
    assert_eq!(
        lower_to_vhdl(&invalid).unwrap_err().kind,
        VhdlBackendErrorKind::InvalidTypeWidth
    );

    let missing = TypedProgram {
        modules: vec![TypedModule {
            id: ModuleId(0),
            name: "bad".into(),
            name_span: span,
            generics: vec![],
            span,
            signals: vec![],
            clocked_blocks: vec![],
            instances: vec![],
            assignments: vec![gatelisp::TypedAssign {
                target: SignalId(99),
                value: TypedExpr {
                    kind: TypedExprKind::Integer(0),
                    ty: HardwareType::Bit,
                    span,
                },
                span,
            }],
        }],
        testbenches: vec![],
        module_order: vec![ModuleId(0)],
    };
    assert_eq!(
        lower_to_vhdl(&missing).unwrap_err().kind,
        VhdlBackendErrorKind::MissingSignal
    );
}
