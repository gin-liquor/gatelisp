use hlisp::{ErrorKind, Lexer, SExpr, TokenKind, parse_document};

#[test]
fn parses_empty_nested_and_multiple_lists() {
    let ast = parse_document("() (a (b))").unwrap();
    assert_eq!(ast.len(), 2);
    assert!(matches!(ast[0].value, SExpr::List(ref xs) if xs.is_empty()));
    assert!(matches!(ast[1].value, SExpr::List(ref xs) if xs.len() == 2));
}

#[test]
fn classifies_atoms() {
    let ast = parse_document("123 -45 - + :in and-gate state?").unwrap();
    assert!(matches!(ast[0].value, SExpr::Integer(123)));
    assert!(matches!(ast[1].value, SExpr::Integer(-45)));
    assert!(matches!(ast[2].value, SExpr::Symbol(ref s) if s == "-"));
    assert!(matches!(ast[3].value, SExpr::Symbol(ref s) if s == "+"));
    assert!(matches!(ast[4].value, SExpr::Keyword(ref s) if s == "in"));
}

#[test]
fn skips_comments_and_crlf() {
    let ast = parse_document("; comment\r\n(a ; tail\r\n b)").unwrap();
    assert!(matches!(ast[0].value, SExpr::List(ref xs) if xs.len() == 2));
    assert_eq!(ast[0].span.start.line, 2);
    assert_eq!(ast[0].span.end.line, 3);
}

#[test]
fn reads_string_escapes() {
    let ast = parse_document(r#""quote: \" slash: \\ n:\n r:\r t:\t""#).unwrap();
    assert!(
        matches!(ast[0].value, SExpr::String(ref s) if s == "quote: \" slash: \\ n:\n r:\r t:\t")
    );
}

#[test]
fn lexer_exposes_tokens_with_spans() {
    let mut lexer = Lexer::new("α :in");
    let symbol = lexer.next_token().unwrap();
    assert!(matches!(symbol.kind, TokenKind::Symbol(ref s) if s == "α"));
    assert_eq!(symbol.span.start.offset, 0);
    assert_eq!(symbol.span.end.offset, 2);
    assert_eq!(symbol.span.end.column, 2);
}

#[test]
fn reports_structural_errors() {
    let right = parse_document(")").unwrap_err();
    assert_eq!(right.kind, ErrorKind::UnexpectedRightParen);
    let list = parse_document("(a").unwrap_err();
    assert_eq!(list.kind, ErrorKind::UnterminatedList);
    assert_eq!(list.span.start.column, 1);
}

#[test]
fn reports_lexical_errors() {
    for (source, kind) in [
        ("\"abc", ErrorKind::UnterminatedString),
        ("\"\\x\"", ErrorKind::InvalidEscape),
        (":", ErrorKind::EmptyKeyword),
        ("9223372036854775808", ErrorKind::IntegerOutOfRange),
    ] {
        assert_eq!(parse_document(source).unwrap_err().kind, kind);
    }
}

#[test]
fn tracks_one_based_lines_and_columns() {
    let ast = parse_document("\n  (x)").unwrap();
    assert_eq!(ast[0].span.start.line, 2);
    assert_eq!(ast[0].span.start.column, 3);
    assert_eq!(ast[0].span.end.column, 6);
}

#[test]
fn spans_are_half_open_and_include_list_parentheses() {
    let ast = parse_document("(α)").unwrap();
    assert_eq!(ast[0].span.start.offset, 0);
    assert_eq!(ast[0].span.end.offset, 4);
    assert_eq!(ast[0].span.start.column, 1);
    assert_eq!(ast[0].span.end.column, 4);

    let SExpr::List(items) = &ast[0].value else {
        panic!("expected a list");
    };
    assert_eq!(items[0].span.start.offset, 1);
    assert_eq!(items[0].span.end.offset, 3);
    assert_eq!(items[0].span.start.column, 2);
    assert_eq!(items[0].span.end.column, 3);
}

#[test]
fn crlf_advances_the_line_exactly_once() {
    let ast = parse_document("a\r\nb\r\nc").unwrap();
    assert_eq!(
        ast.iter()
            .map(|item| item.span.start.line)
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
    assert!(ast.iter().all(|item| item.span.start.column == 1));
}

#[test]
fn utf8_is_safe_in_symbols_keywords_and_strings() {
    let ast = parse_document("日本語 :入力 \"λ値\"").unwrap();
    assert!(matches!(ast[0].value, SExpr::Symbol(ref value) if value == "日本語"));
    assert!(matches!(ast[1].value, SExpr::Keyword(ref value) if value == "入力"));
    assert!(matches!(ast[2].value, SExpr::String(ref value) if value == "λ値"));
}

#[test]
fn distinguishes_sign_symbols_from_signed_integers() {
    let ast = parse_document("- + -0 +42").unwrap();
    assert!(matches!(ast[0].value, SExpr::Symbol(ref value) if value == "-"));
    assert!(matches!(ast[1].value, SExpr::Symbol(ref value) if value == "+"));
    assert!(matches!(ast[2].value, SExpr::Integer(0)));
    assert!(matches!(ast[3].value, SExpr::Integer(42)));
}

#[test]
fn lexical_errors_cover_the_entire_atom() {
    let keyword = parse_document(":").unwrap_err();
    assert_eq!(keyword.kind, ErrorKind::EmptyKeyword);
    assert_eq!((keyword.span.start.offset, keyword.span.end.offset), (0, 1));

    for source in ["9223372036854775808", "-9223372036854775809"] {
        let error = parse_document(source).unwrap_err();
        assert_eq!(error.kind, ErrorKind::IntegerOutOfRange);
        assert_eq!(error.span.end.offset, source.len());
    }
}

#[test]
fn unterminated_nested_list_points_to_its_own_opening_parenthesis() {
    let error = parse_document("(outer\n  (inner").unwrap_err();
    assert_eq!(error.kind, ErrorKind::UnterminatedList);
    assert_eq!((error.span.start.line, error.span.start.column), (2, 3));
    assert_eq!((error.span.start.offset, error.span.end.offset), (9, 10));
}
