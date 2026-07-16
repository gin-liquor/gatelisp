use std::fmt;

use crate::{GateParseError, ParseError, Program, build_program, parse_document};

#[derive(Debug, Clone, PartialEq)]
pub enum FrontendError {
    Reader(ParseError),
    GateSyntax(GateParseError),
}

impl FrontendError {
    pub fn span(&self) -> crate::Span {
        match self {
            Self::Reader(error) => error.span,
            Self::GateSyntax(error) => error.span,
        }
    }
}
impl fmt::Display for FrontendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reader(error) => error.fmt(f),
            Self::GateSyntax(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for FrontendError {}

pub fn parse_program(source: &str) -> Result<Program, FrontendError> {
    let expressions = parse_document(source).map_err(FrontendError::Reader)?;
    build_program(&expressions).map_err(FrontendError::GateSyntax)
}
