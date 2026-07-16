mod ast;
mod compile;
mod diagnostic;
mod frontend;
mod gate_parser;
mod hir;
mod lexer;
mod parser;
mod semantic;
mod sexpr;
mod simulation;
mod source;
mod token;
mod vhdl_ast;
mod vhdl_backend;
mod vhdl_formatter;

pub use ast::*;
pub use compile::{CompileError, compile_source, compile_source_to_vhdl};
pub use diagnostic::{ErrorKind, ParseError};
pub use frontend::{FrontendError, parse_program};
pub use gate_parser::{GateParseError, GateParseErrorKind, build_program};
pub use hir::*;
pub use lexer::Lexer;
pub use parser::parse_document;
pub use semantic::{SemanticError, SemanticErrorKind, analyze_program};
pub use sexpr::SExpr;
pub use simulation::{
    SimulationError, SimulationOptions, SimulationResult, SimulationStage, simulate_source,
};
pub use source::{Position, Span, Spanned};
pub use token::{Token, TokenKind};
pub use vhdl_ast::*;
pub use vhdl_backend::{VhdlBackendError, VhdlBackendErrorKind, lower_to_vhdl};
pub use vhdl_formatter::render_vhdl;
