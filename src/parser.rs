use crate::{ErrorKind, Lexer, ParseError, SExpr, Span, Spanned, Token, TokenKind};

pub fn parse_document(source: &str) -> Result<Vec<Spanned<SExpr>>, ParseError> {
    Parser::new(source)?.document()
}

struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Token,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Result<Self, ParseError> {
        let mut lexer = Lexer::new(source);
        let current = lexer.next_token()?;
        Ok(Self { lexer, current })
    }

    fn document(mut self) -> Result<Vec<Spanned<SExpr>>, ParseError> {
        let mut expressions = Vec::new();
        while self.current.kind != TokenKind::Eof {
            if self.current.kind == TokenKind::RightParen {
                return Err(ParseError::new(
                    ErrorKind::UnexpectedRightParen,
                    "unexpected right parenthesis",
                    self.current.span,
                ));
            }
            expressions.push(self.expression()?);
        }
        Ok(expressions)
    }

    fn expression(&mut self) -> Result<Spanned<SExpr>, ParseError> {
        let token = self.take()?;
        let value = match token.kind {
            TokenKind::LeftParen => return self.list(token.span),
            TokenKind::Symbol(value) => SExpr::Symbol(value),
            TokenKind::Keyword(value) => SExpr::Keyword(value),
            TokenKind::Integer(value) => SExpr::Integer(value),
            TokenKind::String(value) => SExpr::String(value),
            TokenKind::RightParen => {
                return Err(ParseError::new(
                    ErrorKind::UnexpectedRightParen,
                    "unexpected right parenthesis",
                    token.span,
                ));
            }
            TokenKind::Eof => {
                return Err(ParseError::new(
                    ErrorKind::UnexpectedEof,
                    "unexpected end of input",
                    token.span,
                ));
            }
        };
        Ok(Spanned {
            value,
            span: token.span,
        })
    }

    fn list(&mut self, opening: Span) -> Result<Spanned<SExpr>, ParseError> {
        let mut values = Vec::new();
        loop {
            match self.current.kind {
                TokenKind::RightParen => {
                    let closing = self.take()?;
                    return Ok(Spanned {
                        value: SExpr::List(values),
                        span: Span {
                            start: opening.start,
                            end: closing.span.end,
                        },
                    });
                }
                TokenKind::Eof => {
                    return Err(ParseError::new(
                        ErrorKind::UnterminatedList,
                        "unterminated list",
                        opening,
                    ));
                }
                _ => values.push(self.expression()?),
            }
        }
    }

    fn take(&mut self) -> Result<Token, ParseError> {
        let next = self.lexer.next_token()?;
        Ok(std::mem::replace(&mut self.current, next))
    }
}
