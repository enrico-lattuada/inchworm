//! Unit-expression grammar: tokenizer + recursive-descent parser.
//!
//! ```text
//! expr     := term { ("*" | "·" | "×" | "/") term }
//! term     := factor [ ("^" ["-"] ("(" exponent ")" | INT)) | SUPERSCRIPT ]
//! factor   := IDENT | "1" | "(" expr ")"
//! exponent := ["-"] INT [ "/" INT ]
//! IDENT    := [A-Za-z_][A-Za-z0-9_]*
//! ```

use std::fmt::{self, Write};
use std::iter::Peekable;
use std::str::CharIndices;

use inchworm_dimensions::Exp;

use crate::{DeltaUnit, UnitError};

/// Star (mul) char
pub(crate) const MUL_CHAR: char = '*';
/// Pretty star (mul) char
pub(crate) const PRETTY_MUL_CHAR: char = '·';
/// Alt pretty star (mul) char
pub(crate) const ALT_PRETTY_MUL_CHAR: char = '×';
/// Minus char
pub(crate) const MINUS_CHAR: char = '-';
/// Slash (div) char
pub(crate) const SLASH_CHAR: char = '/';
/// Caret (exponentiation) char
pub(crate) const CARET_CHAR: char = '^';
/// Left parenthesis char
pub(crate) const LPAREN_CHAR: char = '(';
/// Right parenthesis char
pub(crate) const RPAREN_CHAR: char = ')';
/// Decimal separator char
pub(crate) const DECIMAL_SEP_CHAR: char = '.';
/// Unit of coherent 1 char
pub(crate) const UNITARY_IDENT_CHAR: char = '1';

pub(crate) fn parse_unit_expr<'a>(
    src: &'a str,
    resolve: &'a dyn Fn(&str) -> Result<DeltaUnit, UnitError>,
) -> Result<DeltaUnit, UnitError> {
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
            Token::Superscript(n) => {
                write!(f, "{CARET_CHAR}{LPAREN_CHAR}{n}{RPAREN_CHAR}")
            }
            Token::Star => f.write_char(MUL_CHAR),
            Token::Slash => f.write_char(SLASH_CHAR),
            Token::Caret => f.write_char(CARET_CHAR),
            Token::Minus => f.write_char(MINUS_CHAR),
            Token::LParen => f.write_char(LPAREN_CHAR),
            Token::RParen => f.write_char(RPAREN_CHAR),
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
            let consumed =
                self.consume_while(start, |c| c.is_ascii_digit() || c == DECIMAL_SEP_CHAR);
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
            MUL_CHAR | PRETTY_MUL_CHAR | ALT_PRETTY_MUL_CHAR => Some(Token::Star),
            MINUS_CHAR => Some(Token::Minus),
            SLASH_CHAR => Some(Token::Slash),
            CARET_CHAR => Some(Token::Caret),
            LPAREN_CHAR => Some(Token::LParen),
            RPAREN_CHAR => Some(Token::RParen),
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
    resolve: &'a dyn Fn(&str) -> Result<DeltaUnit, UnitError>,
}

impl<'a> Parser<'a> {
    fn advance(&mut self) -> Result<Option<Spanned>, UnitError> {
        self.tokens.next().transpose()
    }

    fn parse_factor(&mut self) -> Result<DeltaUnit, UnitError> {
        let spanned = self.advance()?.ok_or_else(|| UnitError::Parse {
            src: self.src.into(),
            offset: self.src.len(),
            message: format!(
                "expected a unit, `{UNITARY_IDENT_CHAR}`, or `{LPAREN_CHAR}`, found end of input; \
                bare numbers are not units, only `{UNITARY_IDENT_CHAR}` is allowed."
            ),
        })?;
        match spanned.token {
            Token::Int(1) => Ok(DeltaUnit::empty()),
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
                message: format!(
                    "expected a unit, `{UNITARY_IDENT_CHAR}`, or `{LPAREN_CHAR}`, found {token}; \
                    bare numbers are not units, only `{UNITARY_IDENT_CHAR}` is allowed."
                ),
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
            message: format!("expected `{RPAREN_CHAR}`, found end of input"),
        })?;
        match spanned.token {
            Token::RParen => Ok(()),
            token => Err(UnitError::Parse {
                src: self.src.into(),
                offset: spanned.offset,
                message: format!("expected `{RPAREN_CHAR}`, found {token}"),
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

    fn parse_term(&mut self) -> Result<DeltaUnit, UnitError> {
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
                token: Token::LParen | Token::Int(1) | Token::Ident(_),
                ..
            }))
        )
    }

    fn parse_expr(&mut self) -> Result<DeltaUnit, UnitError> {
        let mut result = self.parse_term()?;
        loop {
            // peek the next token:
            //   Star  -> advance(), rhs = parse_term()?, result = result.try_mul(&rhs)?
            //   Slash -> advance(), rhs = parse_term()?, result = result.try_div(&rhs)?
            //   (Ident/1/LParen) -> rhs = parse_term()?, result = result.try_mul(&rhs)?
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

    mod parse_unit_expr {
        use super::*;
        use crate::test_utils::{errors_match, mks_registry, parse_error_at};

        #[test]
        fn parses_compound_expression() {
            let registry = mks_registry();
            let meter = registry.get("meter").unwrap();
            let second = registry.get("second").unwrap();
            assert_eq!(
                &registry.parse("meter / second^2").unwrap(),
                &meter.try_div(&second.pow(Exp::int(2)).unwrap()).unwrap()
            );
        }

        #[test]
        fn parses_parenthesized_expression() {
            let registry = mks_registry();
            let meter = registry.get("meter").unwrap();
            assert_eq!(
                &registry.parse("(meter)^2").unwrap(),
                &meter.pow(Exp::int(2)).unwrap()
            );
        }

        #[test]
        fn rejects_trailing_token() {
            let registry = mks_registry();
            let err = registry.parse("meter )").unwrap_err();
            assert!(errors_match(&err, &parse_error_at(6)));
        }

        #[test]
        fn parses_times_sign() {
            let registry = mks_registry();
            let meter = registry.get("meter").unwrap();
            let meter_square = registry.parse("meter × meter").unwrap();
            assert_eq!(&meter_square, &meter.try_mul(&meter).unwrap());
        }

        #[test]
        fn parses_implicit_multiplication() {
            let registry = mks_registry();
            let meter = registry.get("meter").unwrap();
            let meter_square = registry.parse("meter meter").unwrap();
            assert_eq!(&meter_square, &meter.try_mul(&meter).unwrap());
        }

        #[test]
        fn parses_superscript_exponent() {
            let registry = mks_registry();
            let meter = registry.get("meter").unwrap();
            let meter_square = registry.parse("meter²").unwrap();
            assert_eq!(&meter_square, &meter.pow(Exp::int(2)).unwrap());
        }

        #[test]
        fn parses_negative_superscript_exponent() {
            let registry = mks_registry();
            let meter = registry.get("meter").unwrap();
            let spatial_frequency = registry.parse("meter⁻¹").unwrap();
            assert_eq!(&spatial_frequency, &meter.pow(Exp::int(-1)).unwrap());
        }

        #[test]
        fn rejects_lone_superscript_minus() {
            let registry = mks_registry();
            let err = registry.parse("meter⁻").unwrap_err();
            assert!(errors_match(&err, &parse_error_at(5)));
        }

        /// The literal `1` alone is the empty (dimensionless) unit.
        #[test]
        fn parses_literal_one_as_empty_unit() {
            let registry = mks_registry();
            assert_eq!(registry.parse("1").unwrap(), DeltaUnit::empty());
        }

        /// `1` works as a numerator: `1/s` is the reciprocal of `s`.
        #[test]
        fn parses_reciprocal_with_literal_one() {
            let registry = mks_registry();
            let second = registry.get("second").unwrap();
            assert_eq!(registry.parse("1/s").unwrap(), second.recip().unwrap());
        }

        /// `1` is a multiplicative identity on either side of implicit multiplication.
        #[test]
        fn parses_literal_one_in_implicit_multiplication() {
            let registry = mks_registry();
            let meter = registry.get("meter").unwrap();
            assert_eq!(registry.parse("1 m").unwrap(), meter);
            assert_eq!(registry.parse("m 1").unwrap(), meter);
        }

        /// An integer other than `1` is not a unit, even on its own.
        #[test]
        fn rejects_bare_integer() {
            let registry = mks_registry();
            let err = registry.parse("60").unwrap_err();
            assert!(errors_match(&err, &parse_error_at(0)));
        }

        /// An integer factor before `*` is rejected at the integer.
        #[test]
        fn rejects_integer_factor_on_left() {
            let registry = mks_registry();
            let err = registry.parse("2*m").unwrap_err();
            assert!(errors_match(&err, &parse_error_at(0)));
        }

        /// An integer factor after `*` is rejected at the integer, not at the operator.
        #[test]
        fn rejects_integer_factor_on_right() {
            let registry = mks_registry();
            let err = registry.parse("m*2").unwrap_err();
            assert!(errors_match(&err, &parse_error_at(2)));
        }

        /// A float factor is rejected, even with implicit multiplication.
        #[test]
        fn rejects_float_factor() {
            let registry = mks_registry();
            let err = registry.parse("9.81 m").unwrap_err();
            assert!(errors_match(&err, &parse_error_at(0)));
        }

        /// Only the integer `1` is special: `1.0` is a float and is rejected.
        #[test]
        fn rejects_float_literal_one() {
            let registry = mks_registry();
            let err = registry.parse("1.0/s").unwrap_err();
            assert!(errors_match(&err, &parse_error_at(0)));
        }

        /// The error for a bare number says numbers are not units and names `1` as the only exception.
        #[test]
        fn explains_why_bare_numbers_are_rejected() {
            let registry = mks_registry();
            for src in ["60", "m*"] {
                let err = registry.parse(src).unwrap_err();
                let UnitError::Parse { message, .. } = err else {
                    panic!("expected a Parse error, got {err:?}");
                };
                assert!(message.contains("not units"), "message was: {message}");
                assert!(
                    message.contains("only `1` is allowed"),
                    "message was: {message}"
                );
            }
        }
    }
}
