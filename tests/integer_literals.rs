use gatelisp::{ErrorKind, SExpr, parse_document};

#[test]
fn accepts_radix_literals_and_digit_separators() {
    let values = parse_document("0 1_000_000 0b1010_0101 0B11 0o12_34 0O7 0x08_02_AA 0XfF -0x80")
        .unwrap()
        .into_iter()
        .map(|item| match item.value {
            SExpr::Integer(value) => value,
            _ => panic!("integer expected"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        values,
        [0, 1_000_000, 0b1010_0101, 3, 0o1234, 7, 0x0802AA, 255, -128]
    );
}

#[test]
fn compiles_radix_literal_example() {
    gatelisp::compile_source(include_str!("../examples/radix_literals.glisp")).unwrap();
}

#[test]
fn rejects_invalid_radix_literals() {
    for source in [
        "0b", "0o", "0x", "0b2", "0o8", "0xG", "0x_FF", "0xFF_", "1__0", "_1",
    ] {
        let error = parse_document(source).unwrap_err();
        assert_eq!(error.kind, ErrorKind::InvalidIntegerLiteral, "{source}");
    }
}
