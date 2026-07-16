/// A location in the source text.
///
/// `offset` is a zero-based UTF-8 byte offset. `line` and `column` are
/// one-based, and `column` counts Unicode scalar values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

/// A half-open source range: `start` is inclusive and `end` is exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

/// A value paired with the source range from which it was parsed.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}
