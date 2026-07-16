use crate::Spanned;

#[derive(Debug, Clone, PartialEq)]
pub enum SExpr {
    List(Vec<Spanned<SExpr>>),
    Symbol(String),
    Keyword(String),
    Integer(i64),
    String(String),
}
