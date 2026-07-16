use std::fmt;

use crate::{
    FrontendError, GateParseError, ParseError, SemanticError, Span, TypedProgram, analyze_program,
    parse_program,
};

#[derive(Debug, Clone, PartialEq)]
pub enum CompileError {
    Reader(ParseError),
    GateSyntax(GateParseError),
    Semantic(SemanticError),
}

impl CompileError {
    pub fn span(&self) -> Span {
        match self {
            Self::Reader(e) => e.span,
            Self::GateSyntax(e) => e.span,
            Self::Semantic(e) => e.span,
        }
    }
    pub fn related_span(&self) -> Option<Span> {
        match self {
            Self::Semantic(e) => e.related_span.as_deref().copied(),
            _ => None,
        }
    }
}
impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reader(e) => e.fmt(f),
            Self::GateSyntax(e) => e.fmt(f),
            Self::Semantic(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for CompileError {}

pub fn compile_source(source: &str) -> Result<TypedProgram, CompileError> {
    let program = parse_program(source).map_err(|error| match error {
        FrontendError::Reader(error) => CompileError::Reader(error),
        FrontendError::GateSyntax(error) => CompileError::GateSyntax(error),
    })?;
    analyze_program(&program).map_err(CompileError::Semantic)
}
