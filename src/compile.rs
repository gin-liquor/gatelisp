use std::fmt;

use crate::{
    FrontendError, GateParseError, ParseError, SemanticError, Span, TypedProgram, VhdlBackendError,
    analyze_program, lower_to_vhdl, parse_program, render_vhdl,
};

#[derive(Debug, Clone, PartialEq)]
pub enum CompileError {
    Reader(ParseError),
    GateSyntax(GateParseError),
    Semantic(SemanticError),
    VhdlBackend(VhdlBackendError),
}

impl CompileError {
    pub fn span(&self) -> Span {
        match self {
            Self::Reader(e) => e.span,
            Self::GateSyntax(e) => e.span,
            Self::Semantic(e) => e.span,
            Self::VhdlBackend(_) => Span {
                start: crate::Position {
                    offset: 0,
                    line: 1,
                    column: 1,
                },
                end: crate::Position {
                    offset: 0,
                    line: 1,
                    column: 1,
                },
            },
        }
    }
    pub fn related_span(&self) -> Option<Span> {
        match self {
            Self::Semantic(e) => e.related_span.as_deref().copied(),
            Self::VhdlBackend(_) => None,
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
            Self::VhdlBackend(e) => e.fmt(f),
        }
    }
}

pub fn compile_source_to_vhdl(source: &str) -> Result<String, CompileError> {
    let program = compile_source(source)?;
    let design = lower_to_vhdl(&program).map_err(CompileError::VhdlBackend)?;
    Ok(render_vhdl(&design))
}
impl std::error::Error for CompileError {}

pub fn compile_source(source: &str) -> Result<TypedProgram, CompileError> {
    let program = parse_program(source).map_err(|error| match error {
        FrontendError::Reader(error) => CompileError::Reader(error),
        FrontendError::GateSyntax(error) => CompileError::GateSyntax(error),
    })?;
    analyze_program(&program).map_err(CompileError::Semantic)
}
