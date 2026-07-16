use gatelisp::compile_source_to_vhdl;

fn compile(source: &str) -> String {
    compile_source_to_vhdl(source).expect("vhdl")
}

#[test]
fn reads_and_writes_register_array() {
    let source = "(module rf (ports (clk :in bit) (ra :in (unsigned 2)) (wa :in (unsigned 2)) (wd :in (unsigned 8)) (rd :out (unsigned 8))) (register-array regs :address-width 2 :data-width 8 :initial 0) (assign rd (register-array-read regs ra)) (clocked clk (register-array-write regs wa wd)))";
    let vhdl = compile(source);
    assert!(vhdl.contains("type gl_register_array_regs_t"));
    assert!(vhdl.contains("gl_register_array_regs(to_integer(gl_p"));
}

#[test]
fn rejects_invalid_register_array_forms() {
    let source =
        "(module m (ports) (register-array regs :address-width 0 :data-width 8 :initial 0))";
    assert!(compile_source_to_vhdl(source).is_err());
}
