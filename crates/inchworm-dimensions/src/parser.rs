//! Dimension-expression grammar: tokenizer + recursive-descent parser.
//!
//! ```text
//! expr     := term { ("*" | "·" | "/") term }        # left-assoc; "/" binds like "*"
//! term     := factor [ "^" ["-"] ( "(" exponent ")" | INT ) ]
//! factor   := IDENT | "1" | "(" expr ")"
//! exponent := ["-"] INT [ "/" INT ]                   # 2, -1, 1/2, -3/2, only reachable inside parens
//! IDENT    := [A-Za-z_][A-Za-z0-9_]*
//! ```

use std::fmt;
use std::iter::Peekable;
use std::str::CharIndices;

use crate::Exp;
use crate::dimension::Dimension;
use crate::error::DimensionError;

/// Parse an expression against a name-resolver (the registry, or the TOML
/// loader's in-progress symbol table). Errors carry byte offsets so
/// diagnostics can point into the source.
pub(crate) fn parse_dim_expr<'a>(
    src: &'a str,
    resolve: &'a dyn Fn(&str) -> Result<Dimension, DimensionError>,
) -> Result<Dimension, DimensionError> {
    let chars = src.char_indices().peekable();
    let lexer = Lexer { src, chars };
    let tokens = lexer.peekable();
    let mut parser = Parser {
        tokens,
        src,
        resolve,
    };
    let dim = parser.parse_expr()?;
    match parser.advance()? {
        None => Ok(dim),
        Some(spanned) => {
            let token = spanned.token;
            let message = format!("expected end of input, found trailing {token}");
            Err(DimensionError::Parse {
                src: src.into(),
                offset: spanned.offset,
                message,
            })
        }
    }
}

pub(crate) fn extract_idents(src: &str) -> Result<Vec<String>, DimensionError> {
    let chars = src.char_indices().peekable();
    let lexer = Lexer { src, chars };
    let mut idents = Vec::new();
    for item in lexer {
        let spanned = item?;
        if let Token::Ident(ident) = spanned.token {
            idents.push(ident);
        }
    }
    Ok(idents)
}

#[derive(Debug, PartialEq)]
enum Token {
    Ident(String), // IDENT
    Number(i64),   // the INT in "1" and in exponents
    Star,          // "*" or "·", collapse both into one token
    Slash,         // "/", used for both expr division and exponent fraction
    Caret,         // "^"
    Minus,         // "-", only meaningful before an exponent's INT
    LParen,
    RParen,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Ident(ident) => write!(f, "{ident}"),
            Token::Number(n) => write!(f, "{n}"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Caret => write!(f, "^"),
            Token::Minus => write!(f, "-"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
        }
    }
}

#[derive(Debug, PartialEq)]
struct Spanned {
    token: Token,
    offset: usize,
}

struct Lexer<'a> {
    src: &'a str,
    chars: Peekable<CharIndices<'a>>,
}

impl<'a> Lexer<'a> {
    fn consume_while(&mut self, start: usize, pred: impl Fn(char) -> bool) -> &'a str {
        let mut end = start;
        while let Some(&(i, c)) = self.chars.peek() {
            if !pred(c) {
                break;
            }
            end = i + c.len_utf8();
            self.chars.next();
        }
        &self.src[start..end]
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Spanned, DimensionError>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(&(_, c)) = self.chars.peek() {
            if !c.is_whitespace() {
                break;
            }
            self.chars.next();
        }
        let &(start, c) = self.chars.peek()?;
        if c.is_ascii_digit() {
            let consumed = self.consume_while(start, |c| c.is_ascii_digit());
            let maybe_number = consumed.parse::<i64>();
            match maybe_number {
                Ok(number) => {
                    let (token, offset) = (Token::Number(number), start);
                    Some(Ok(Spanned { token, offset }))
                }
                Err(e) => Some(Err(DimensionError::Parse {
                    src: self.src.into(),
                    offset: start,
                    message: e.to_string(),
                })),
            }
        } else if c.is_ascii_alphanumeric() || c == '_' {
            let consumed = self.consume_while(start, |c| c.is_ascii_alphanumeric() || c == '_');
            let (token, offset) = (Token::Ident(consumed.into()), start);
            Some(Ok(Spanned { token, offset }))
        } else if let Some(token) = match c {
            '*' | '·' => Some(Token::Star),
            '-' => Some(Token::Minus),
            '/' => Some(Token::Slash),
            '^' => Some(Token::Caret),
            '(' => Some(Token::LParen),
            ')' => Some(Token::RParen),
            _ => None,
        } {
            self.chars.next();
            let offset = start;
            Some(Ok(Spanned { token, offset }))
        } else {
            self.chars.next();
            Some(Err(DimensionError::Parse {
                src: self.src.into(),
                offset: start,
                message: format!("unsupported char '{c}'"),
            }))
        }
    }
}

struct Parser<'a> {
    tokens: Peekable<Lexer<'a>>,
    src: &'a str,
    resolve: &'a dyn Fn(&str) -> Result<Dimension, DimensionError>,
}

impl<'a> Parser<'a> {
    fn advance(&mut self) -> Result<Option<Spanned>, DimensionError> {
        self.tokens.next().transpose()
    }

    fn parse_factor(&mut self) -> Result<Dimension, DimensionError> {
        let spanned = self.advance()?.ok_or_else(|| DimensionError::Parse {
            src: self.src.into(),
            offset: self.src.len(),
            message: "expected a dimension, `1`, or `(`, found end of input".into(),
        })?;
        match spanned.token {
            Token::Number(1) => Ok(Dimension::dimensionless()),
            Token::Number(n) => {
                // Only `1` is accepted as per grammar rules
                let message = format!("expected a dimension, `1`, or `(`, found a number {n}");
                Err(DimensionError::Parse {
                    src: self.src.into(),
                    offset: spanned.offset,
                    message,
                })
            }
            Token::Ident(name) => (self.resolve)(&name),
            Token::LParen => {
                // Parse expression inside `(...)´
                let inner = self.parse_expr()?;
                self.expect_rparen()?;
                Ok(inner)
            }
            token => {
                let message = format!("expected a dimension, `1`, or `(`, found {token}");
                Err(DimensionError::Parse {
                    src: self.src.into(),
                    offset: spanned.offset,
                    message,
                })
            }
        }
    }

    fn expect_number(&mut self) -> Result<i64, DimensionError> {
        let spanned = self.advance()?.ok_or_else(|| DimensionError::Parse {
            src: self.src.into(),
            offset: self.src.len(),
            message: "expected number, found end of input".into(),
        })?;
        match spanned.token {
            Token::Number(n) => Ok(n),
            token => Err(DimensionError::Parse {
                src: self.src.into(),
                offset: spanned.offset,
                message: format!("expected number, found {token}"),
            }),
        }
    }

    fn expect_rparen(&mut self) -> Result<(), DimensionError> {
        let spanned = self.advance()?.ok_or_else(|| DimensionError::Parse {
            src: self.src.into(),
            offset: self.src.len(),
            message: "expected `)`, found end of input".into(),
        })?;
        match spanned.token {
            Token::RParen => Ok(()),
            token => Err(DimensionError::Parse {
                src: self.src.into(),
                offset: spanned.offset,
                message: format!("expected `)`, found {token}"),
            }),
        }
    }

    fn consume_if(&mut self, expected: &Token) -> Result<bool, DimensionError> {
        match self.tokens.peek() {
            Some(Ok(Spanned { token, .. })) if token == expected => {
                self.advance()?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn parse_exponent(&mut self, allow_fraction: bool) -> Result<Exp, DimensionError> {
        // 1. peek: is the next token Minus? if so, advance() and remember negative = true
        let negative = self.consume_if(&Token::Minus)?;
        // 2. advance(): must be a Number — that's the numerator (negate if step 1 saw '-')
        //    (anything else here, or end of input, is a parse error)
        let num = self.expect_number()?;
        let num = if negative { -num } else { num };
        // 3. peek: is the next token Slash? if so, advance() it, then advance() again expecting
        //    a Number — that's the denominator. If no Slash, denominator is 1.
        if allow_fraction && self.consume_if(&Token::Slash)? {
            let den = self.expect_number()?;
            Exp::new(num, den)
        } else {
            Exp::int(num)
        }
    }

    fn parse_term(&mut self) -> Result<Dimension, DimensionError> {
        let base = self.parse_factor()?;
        // peek: if the next token is Token::Caret, advance() past it, call parse_exponent(),
        // then return base.pow(exp) — otherwise just return base unchanged (implicit exponent 1)
        if !self.consume_if(&Token::Caret)? {
            return Ok(base);
        }
        let outer_negative = self.consume_if(&Token::Minus)?;
        let parenthesized = self.consume_if(&Token::LParen)?;
        let exp = self.parse_exponent(parenthesized)?;
        if parenthesized {
            self.expect_rparen()?;
        }
        let exp = if outer_negative {
            exp.checked_neg()?
        } else {
            exp
        };
        base.pow(exp)
    }

    fn parse_expr(&mut self) -> Result<Dimension, DimensionError> {
        let mut result = self.parse_term()?;
        loop {
            // peek the next token:
            //   Star  -> advance(), rhs = parse_term()?, result = result.try_mul(&rhs)?
            //   Slash -> advance(), rhs = parse_term()?, result = result.try_div(&rhs)?
            //   anything else, or end of input -> break
            if self.consume_if(&Token::Star)? {
                let rhs = self.parse_term()?;
                result = result.try_mul(&rhs)?;
            } else if self.consume_if(&Token::Slash)? {
                let rhs = self.parse_term()?;
                result = result.try_div(&rhs)?;
            } else {
                break;
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::errors_match;

    mod extract_idents {
        use super::*;

        #[test]
        fn multi_factor_expr() {
            let expr = "length * time^2";
            let idents = extract_idents(expr).unwrap();
            assert_eq!(idents, vec!["length", "time"]);
        }

        #[test]
        fn dimensionless() {
            let expr = "1";
            let idents = extract_idents(expr).unwrap();
            let empty_vec: Vec<String> = Vec::new();
            assert_eq!(idents, empty_vec);
        }

        #[test]
        fn keeps_duplicate() {
            let expr = "length / length";
            let idents = extract_idents(expr).unwrap();
            assert_eq!(idents, vec!["length", "length"]);
        }

        #[test]
        fn propagates_bad_char() {
            let expr = "length @ length";
            let err = extract_idents(expr).unwrap_err();
            let expected_err = DimensionError::Parse {
                src: expr.into(),
                offset: 7,
                message: "".into(),
            };
            assert!(errors_match(&err, &expected_err));
        }
    }
}
