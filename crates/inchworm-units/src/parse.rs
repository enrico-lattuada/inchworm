//! Unit-expression grammar: tokenizer + recursive-descent parser.
//!
//! ```text
//! expr     := term { ("*" | "·" | "×" | "/") term }
//! term     := factor [ ("^" ["-"] ("(" exponent ")" | INT)) | SUPERSCRIPT ]
//! factor   := IDENT | NUMBER | "(" expr ")"
//! exponent := ["-"] INT [ "/" INT ]
//! IDENT    := [A-Za-z_][A-Za-z0-9_]*
//! ```

use std::fmt;
use std::iter::Peekable;
use std::str::CharIndices;

use inchworm_dimensions::Exp;

use crate::{Unit, UnitError};

pub(crate) fn parse_unit_expr<'a>(
    src: &'a str,
    resolve: &'a dyn Fn(&str) -> Result<Unit, UnitError>,
) -> Result<Unit, UnitError> {
    let chars = src.char_indices().peekable();
    let lexer = Lexer { src, chars };
    let tokens = lexer.peekable();
    let mut parser = Parser {
        tokens,
        src,
        resolve,
    };
    let unit = parser.parse_expr()?;
    match parser.advance()? {
        None => Ok(unit),
        Some(spanned) => {
            let token = spanned.token;
            Err(UnitError::Parse {
                src: src.into(),
                offset: spanned.offset,
                message: format!("expected end of input, found trailing {token}"),
            })
        }
    }
}

pub(crate) fn is_valid_ident(src: &str) -> bool {
    let chars = src.char_indices().peekable();
    let lexer = Lexer { src, chars };
    let mut tokens = Vec::new();
    for item in lexer {
        match item {
            Ok(spanned) => tokens.push(spanned.token),
            Err(_) => return false,
        }
    }
    matches!(tokens.as_slice(), [Token::Ident(_)])
}

#[derive(Debug, PartialEq)]
enum Token {
    Ident(String),    // IDENT
    Int(i64),         // INT
    Float(f64),       // FLOAT
    Superscript(i64), // ², ⁻¹
    Star,             // "*", "·", or "×", collapse both into one token
    Slash,            // "/", used for both expr division and exponent fraction
    Caret,            // "^"
    Minus,            // "-", only meaningful before an exponent's INT
    LParen,
    RParen,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Ident(ident) => write!(f, "{ident}"),
            Token::Int(n) => write!(f, "{n}"),
            Token::Float(n) => write!(f, "{n}"),
            Token::Superscript(n) => write!(f, "^({n})"),
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

fn superscript_to_digit(c: char) -> Option<char> {
    match c {
        '⁰' => Some('0'),
        '¹' => Some('1'),
        '²' => Some('2'),
        '³' => Some('3'),
        '⁴' => Some('4'),
        '⁵' => Some('5'),
        '⁶' => Some('6'),
        '⁷' => Some('7'),
        '⁸' => Some('8'),
        '⁹' => Some('9'),
        '⁻' => Some('-'),
        _ => None,
    }
}

pub(crate) fn digit_to_superscript(c: char) -> Option<char> {
    match c {
        '0' => Some('⁰'),
        '1' => Some('¹'),
        '2' => Some('²'),
        '3' => Some('³'),
        '4' => Some('⁴'),
        '5' => Some('⁵'),
        '6' => Some('⁶'),
        '7' => Some('⁷'),
        '8' => Some('⁸'),
        '9' => Some('⁹'),
        _ => None,
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Spanned, UnitError>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(&(_, c)) = self.chars.peek() {
            if !c.is_whitespace() {
                break;
            }
            self.chars.next();
        }
        let &(start, c) = self.chars.peek()?;
        if c.is_ascii_digit() {
            let consumed = self.consume_while(start, |c| c.is_ascii_digit() || c == '.');
            if let Ok(number) = consumed.parse::<i64>() {
                let (token, offset) = (Token::Int(number), start);
                Some(Ok(Spanned { token, offset }))
            } else {
                match consumed.parse::<f64>() {
                    Ok(number) => {
                        let (token, offset) = (Token::Float(number), start);
                        Some(Ok(Spanned { token, offset }))
                    }
                    Err(e) => Some(Err(UnitError::Parse {
                        src: self.src.into(),
                        offset: start,
                        message: e.to_string(),
                    })),
                }
            }
        } else if c.is_ascii_alphanumeric() || c == '_' {
            let consumed = self.consume_while(start, |c| c.is_ascii_alphanumeric() || c == '_');
            let (token, offset) = (Token::Ident(consumed.into()), start);
            Some(Ok(Spanned { token, offset }))
        } else if superscript_to_digit(c).is_some() {
            let consumed: String = self
                .consume_while(start, |c| superscript_to_digit(c).is_some())
                .chars()
                .map(|c| superscript_to_digit(c).expect("should be a superscript here"))
                .collect();
            match consumed.parse::<i64>() {
                Ok(number) => {
                    let (token, offset) = (Token::Superscript(number), start);
                    Some(Ok(Spanned { token, offset }))
                }
                Err(e) => Some(Err(UnitError::Parse {
                    src: self.src.into(),
                    offset: start,
                    message: e.to_string(),
                })),
            }
        } else if let Some(token) = match c {
            '*' | '·' | '×' => Some(Token::Star),
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
            Some(Err(UnitError::Parse {
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
    resolve: &'a dyn Fn(&str) -> Result<Unit, UnitError>,
}

impl<'a> Parser<'a> {
    fn advance(&mut self) -> Result<Option<Spanned>, UnitError> {
        self.tokens.next().transpose()
    }

    fn parse_factor(&mut self) -> Result<Unit, UnitError> {
        let spanned = self.advance()?.ok_or_else(|| UnitError::Parse {
            src: self.src.into(),
            offset: self.src.len(),
            message: "expected a unit, a number, or `(`, found end of input".into(),
        })?;
        match spanned.token {
            Token::Int(n) => Ok(Unit::scaled(n as f64)),
            Token::Float(n) => Ok(Unit::scaled(n)),
            Token::Ident(name) => (self.resolve)(&name),
            Token::LParen => {
                // Parse expression inside `(...)´
                let inner = self.parse_expr()?;
                self.expect_rparen()?;
                Ok(inner)
            }
            token => Err(UnitError::Parse {
                src: self.src.into(),
                offset: spanned.offset,
                message: format!("expected a unit, a number, or `(`, found {token}"),
            }),
        }
    }

    fn expect_int(&mut self) -> Result<i64, UnitError> {
        let spanned = self.advance()?.ok_or_else(|| UnitError::Parse {
            src: self.src.into(),
            offset: self.src.len(),
            message: "expected integer, found end of input".into(),
        })?;
        match spanned.token {
            Token::Int(n) => Ok(n),
            token => Err(UnitError::Parse {
                src: self.src.into(),
                offset: spanned.offset,
                message: format!("expected integer, found {token}"),
            }),
        }
    }

    fn expect_rparen(&mut self) -> Result<(), UnitError> {
        let spanned = self.advance()?.ok_or_else(|| UnitError::Parse {
            src: self.src.into(),
            offset: self.src.len(),
            message: "expected `)`, found end of input".into(),
        })?;
        match spanned.token {
            Token::RParen => Ok(()),
            token => Err(UnitError::Parse {
                src: self.src.into(),
                offset: spanned.offset,
                message: format!("expected `)`, found {token}"),
            }),
        }
    }

    fn consume_if(&mut self, expected: &Token) -> Result<bool, UnitError> {
        match self.tokens.peek() {
            Some(Ok(Spanned { token, .. })) if token == expected => {
                self.advance()?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn parse_exponent(&mut self, allow_fraction: bool) -> Result<Exp, UnitError> {
        // 1. peek: is the next token Minus? if so, advance() and remember negative = true
        let negative = self.consume_if(&Token::Minus)?;
        // 2. advance(): must be a Number — that's the numerator (negate if step 1 saw '-')
        //    (anything else here, or end of input, is a parse error)
        let num = self.expect_int()?;
        let num = if negative { -num } else { num };
        // 3. peek: is the next token Slash? if so, advance() it, then advance() again expecting
        //    a Int — that's the denominator. If no Slash, denominator is 1.
        if allow_fraction && self.consume_if(&Token::Slash)? {
            let den = self.expect_int()?;
            Ok(Exp::new(num, den)?)
        } else {
            Ok(Exp::int(num))
        }
    }

    fn consume_superscript(&mut self) -> Result<Option<i64>, UnitError> {
        if !matches!(
            self.tokens.peek(),
            Some(Ok(Spanned {
                token: Token::Superscript(_),
                ..
            }))
        ) {
            return Ok(None);
        }
        if let Spanned {
            token: Token::Superscript(number),
            ..
        } = self
            .advance()?
            .expect("confirmed above to be a Token::Superscript()")
        {
            Ok(Some(number))
        } else {
            unreachable!(
                "confirmed above to be a Token::Superscript(), should never reach this point."
            )
        }
    }

    fn parse_term(&mut self) -> Result<Unit, UnitError> {
        let base = self.parse_factor()?;
        // peek: if the next token is Token::Caret, advance() past it, call parse_exponent(),
        // then return base.pow(exp) - otherwise just return base unchanged (implicit exponent 1)
        if self.consume_if(&Token::Caret)? {
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
        } else if let Some(exp) = self.consume_superscript()? {
            base.pow(Exp::int(exp))
        } else {
            Ok(base)
        }
    }

    fn peek_starts_factor(&mut self) -> bool {
        matches!(
            self.tokens.peek(),
            Some(Ok(Spanned {
                token: Token::LParen | Token::Int(_) | Token::Float(_) | Token::Ident(_),
                ..
            }))
        )
    }

    fn parse_expr(&mut self) -> Result<Unit, UnitError> {
        let mut result = self.parse_term()?;
        loop {
            // peek the next token:
            //   Star  -> advance(), rhs = parse_term()?, result = result.try_mul(&rhs)?
            //   Slash -> advance(), rhs = parse_term()?, result = result.try_div(&rhs)?
            //   (Ident/Float/Int/LParen) -> rhs = parse_term()?, result = result.try_mul(&rhs)?
            //   anything else, or end of input -> break
            if self.consume_if(&Token::Star)? {
                let rhs = self.parse_term()?;
                result = result.try_mul(&rhs)?;
            } else if self.consume_if(&Token::Slash)? {
                let rhs = self.parse_term()?;
                result = result.try_div(&rhs)?;
            } else if self.peek_starts_factor() {
                let rhs = self.parse_term()?;
                result = result.try_mul(&rhs)?;
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

    mod is_valid_ident {
        use super::*;

        #[test]
        fn accepts_simple_identifier() {
            assert!(is_valid_ident("meter"));
        }

        #[test]
        fn accepts_identifier_with_digits_and_underscore() {
            assert!(is_valid_ident("meters_per_second2"));
        }

        #[test]
        fn accepts_leading_underscore() {
            assert!(is_valid_ident("_private"));
        }

        #[test]
        fn rejects_multi_word_name() {
            assert!(!is_valid_ident("newton meter"));
        }

        #[test]
        fn rejects_leading_digit() {
            assert!(!is_valid_ident("2volts"));
        }

        #[test]
        fn rejects_unsupported_character() {
            assert!(!is_valid_ident("µmeter"));
        }

        #[test]
        fn rejects_empty_string() {
            assert!(!is_valid_ident(""));
        }

        #[test]
        fn rejects_bare_number() {
            assert!(!is_valid_ident("1"));
        }

        #[test]
        fn rejects_lone_operator() {
            assert!(!is_valid_ident("*"));
        }

        #[test]
        fn rejects_bare_float() {
            assert!(!is_valid_ident("9.81"));
        }
    }
}
