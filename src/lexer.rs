use crate::{ErrorKind, ParseError, Position, Span, Token, TokenKind};

pub struct Lexer<'a> {
    source: &'a str,
    offset: usize,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            offset: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn next_token(&mut self) -> Result<Token, ParseError> {
        self.skip_trivia();
        let start = self.position();
        let Some(ch) = self.peek() else {
            return Ok(Token {
                kind: TokenKind::Eof,
                span: Span { start, end: start },
            });
        };
        match ch {
            '(' => {
                self.advance();
                Ok(self.token(TokenKind::LeftParen, start))
            }
            ')' => {
                self.advance();
                Ok(self.token(TokenKind::RightParen, start))
            }
            '"' => self.string(start),
            _ => self.atom(start),
        }
    }

    fn skip_trivia(&mut self) {
        loop {
            while self.peek().is_some_and(char::is_whitespace) {
                self.advance();
            }
            if self.peek() != Some(';') {
                break;
            }
            while !matches!(self.peek(), None | Some('\n') | Some('\r')) {
                self.advance();
            }
        }
    }

    fn string(&mut self, start: Position) -> Result<Token, ParseError> {
        self.advance();
        let mut value = String::new();
        loop {
            match self.peek() {
                None => {
                    return Err(ParseError::new(
                        ErrorKind::UnterminatedString,
                        "unterminated string",
                        Span {
                            start,
                            end: self.position(),
                        },
                    ));
                }
                Some('"') => {
                    self.advance();
                    return Ok(self.token(TokenKind::String(value), start));
                }
                Some('\\') => {
                    let escape_start = self.position();
                    self.advance();
                    let escaped = match self.peek() {
                        Some('"') => '"',
                        Some('\\') => '\\',
                        Some('n') => '\n',
                        Some('r') => '\r',
                        Some('t') => '\t',
                        Some(_) => {
                            self.advance();
                            return Err(ParseError::new(
                                ErrorKind::InvalidEscape,
                                "invalid string escape",
                                Span {
                                    start: escape_start,
                                    end: self.position(),
                                },
                            ));
                        }
                        None => {
                            return Err(ParseError::new(
                                ErrorKind::UnterminatedString,
                                "unterminated string",
                                Span {
                                    start,
                                    end: self.position(),
                                },
                            ));
                        }
                    };
                    self.advance();
                    value.push(escaped);
                }
                Some(ch) => {
                    self.advance();
                    value.push(ch);
                }
            }
        }
    }

    fn atom(&mut self, start: Position) -> Result<Token, ParseError> {
        while self
            .peek()
            .is_some_and(|ch| !ch.is_whitespace() && !matches!(ch, '(' | ')' | ';'))
        {
            self.advance();
        }
        let text = &self.source[start.offset..self.offset];
        let span = Span {
            start,
            end: self.position(),
        };
        let kind = if let Some(keyword) = text.strip_prefix(':') {
            if keyword.is_empty() {
                return Err(ParseError::new(
                    ErrorKind::EmptyKeyword,
                    "empty keyword",
                    span,
                ));
            }
            TokenKind::Keyword(keyword.to_owned())
        } else if is_integer_syntax(text) {
            TokenKind::Integer(parse_integer(text).map_err(|message| {
                let kind = if message == "integer is outside the i64 range" {
                    ErrorKind::IntegerOutOfRange
                } else {
                    ErrorKind::InvalidIntegerLiteral
                };
                ParseError::new(kind, message, span)
            })?)
        } else {
            TokenKind::Symbol(text.to_owned())
        };
        Ok(Token { kind, span })
    }

    fn peek(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.offset += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    fn position(&self) -> Position {
        Position {
            offset: self.offset,
            line: self.line,
            column: self.column,
        }
    }
    fn token(&self, kind: TokenKind, start: Position) -> Token {
        Token {
            kind,
            span: Span {
                start,
                end: self.position(),
            },
        }
    }
}

fn is_integer_syntax(text: &str) -> bool {
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    digits.chars().next().is_some_and(|ch| ch.is_ascii_digit())
        || digits
            .strip_prefix('_')
            .and_then(|rest| rest.chars().next())
            .is_some_and(|ch| ch.is_ascii_digit())
}

fn parse_integer(text: &str) -> Result<i64, String> {
    let (negative, body) = match text.strip_prefix('-') {
        Some(body) => (true, body),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let (radix, digits) = match body.get(..2) {
        Some("0b" | "0B") => (2, &body[2..]),
        Some("0o" | "0O") => (8, &body[2..]),
        Some("0x" | "0X") => (16, &body[2..]),
        _ => (10, body),
    };
    if digits.is_empty() {
        return Err("integer literal requires at least one digit".into());
    }
    let mut value: i128 = 0;
    let mut previous_digit = false;
    for ch in digits.chars() {
        if ch == '_' {
            if !previous_digit {
                return Err("underscore must appear between digits".into());
            }
            previous_digit = false;
            continue;
        }
        let digit = ch
            .to_digit(radix)
            .ok_or_else(|| format!("invalid digit '{ch}' in base-{radix} integer literal"))?;
        value = value
            .checked_mul(i128::from(radix))
            .and_then(|v| v.checked_add(i128::from(digit)))
            .ok_or_else(|| "integer is outside the i64 range".to_owned())?;
        previous_digit = true;
    }
    if !previous_digit {
        return Err("underscore must appear between digits".into());
    }
    if negative {
        i64::try_from(-value).map_err(|_| "integer is outside the i64 range".to_owned())
    } else {
        i64::try_from(value).map_err(|_| "integer is outside the i64 range".to_owned())
    }
}
