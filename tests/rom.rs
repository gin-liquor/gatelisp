use gatelisp::{CompileError, SemanticErrorKind, compile_source, compile_source_to_vhdl};

fn error(source: &str) -> SemanticErrorKind {
    match compile_source(source) {
        Err(CompileError::Semantic(error)) => error.kind,
        other => panic!("semantic error expected: {other:?}"),
    }
}

#[test]
fn parses_sparse_rom_and_reads_exact_width_address() {
    let typed = compile_source(include_str!("../examples/rom_async.glisp")).unwrap();
    assert_eq!(typed.roms.len(), 1);
    assert_eq!(typed.roms[0].depth, 8);
    let vhdl = compile_source_to_vhdl(include_str!("../examples/rom_async.glisp")).unwrap();
    assert!(vhdl.contains("type gl_rom_table_t is array"));
    assert!(vhdl.contains("gl_rom_table(to_integer(gl_p0_address))"));
    assert!(vhdl.contains("others => \"00000000\""));
}

#[test]
fn rejects_invalid_rom_declarations_and_addresses() {
    assert!(compile_source("(rom r :address-width 0 :data-width 8 :default 0)").is_err());
    assert_eq!(
        error("(rom r :address-width 3 :data-width 8 :default 0 (8 1)) (module m (ports))"),
        SemanticErrorKind::InvalidRomAddress
    );
    assert_eq!(
        error("(rom r :address-width 3 :data-width 8 :default 0 (1 1) (1 2)) (module m (ports))"),
        SemanticErrorKind::DuplicateRomAddress
    );
    assert_eq!(
        error("(rom r :address-width 3 :data-width 8 :default 256) (module m (ports))"),
        SemanticErrorKind::InvalidRomDefault
    );
}

#[test]
fn rejects_wrong_rom_read_address_types_and_unknown_roms() {
    assert_eq!(
        error(
            "(rom r :address-width 3 :data-width 8 :default 0) (module m (ports (a :in (unsigned 4)) (o :out (unsigned 8))) (assign o (rom-read r a)))"
        ),
        SemanticErrorKind::InvalidRomRead
    );
    assert_eq!(
        error(
            "(module m (ports (a :in (unsigned 3)) (o :out (unsigned 8))) (assign o (rom-read missing a)))"
        ),
        SemanticErrorKind::UnknownRom
    );
}
