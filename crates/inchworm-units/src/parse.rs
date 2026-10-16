//! Unit-expression grammar: tokenizer + recursive-descent parser.
//!
//! ```text
//! expr     := term { ("*" | "·" | "×" | "/") term }
//! term     := factor [ ("^" ["-"] ("(" exponent ")" | INT)) | SUPERSCRIPT ]
//! factor   := IDENT | "1" | "(" expr ")"
//! exponent := ["-"] INT [ "/" INT ]
//! IDENT    := [A-Za-z_][A-Za-z0-9_]*
//! ```

use std::{
    fmt::{self, Write},
    iter::Peekable,
    str::CharIndices,
};

use inchworm_dimensions::Exp;

use crate::{DeltaUnit, Unit, UnitError};

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

/// Parse an expression against a name-resolver (the registry).
/// Errors carry byte offsets so diagnostics can point into the source.
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
            let message = format!("expected end of input, found trailing {token}");
            Err(UnitError::Parse {
                src: src.into(),
                offset: spanned.offset,
                message,
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

pub(crate) fn validate_ident(candidate: &str) -> Result<(), UnitError> {
    if !is_valid_ident(candidate) {
        Err(UnitError::InvalidName {
            name: candidate.into(),
        })
    } else {
        Ok(())
    }
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
    resolve: &'a dyn Fn(&str) -> Result<Unit, UnitError>,
}

impl<'a> Parser<'a> {
    fn advance(&mut self) -> Result<Option<Spanned>, UnitError> {
        self.tokens.next().transpose()
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

    fn expect_int(&mut self) -> Result<i64, UnitError> {
        let spanned = self.advance()?.ok_or_else(|| UnitError::Parse {
            src: self.src.into(),
            offset: self.src.len(),
            message: "expected number, found end of input".into(),
        })?;
        match spanned.token {
            Token::Int(n) => Ok(n),
            token => Err(UnitError::Parse {
                src: self.src.into(),
                offset: spanned.offset,
                message: format!("expected number, found {token}"),
            }),
        }
    }

    fn parse_exponent(&mut self, allow_fraction: bool) -> Result<Exp, UnitError> {
        let negative = self.consume_if(&Token::Minus)?;
        let num = self.expect_int()?;
        let num = if negative { -num } else { num };
        if allow_fraction && self.consume_if(&Token::Slash)? {
            let den = self.expect_int()?;
            Exp::new(num, den).map_err(UnitError::Dimension)
        } else {
            Ok(Exp::int(num))
        }
    }

    fn parse_factor(&mut self) -> Result<Unit, UnitError> {
        let spanned = self.advance()?.ok_or_else(|| UnitError::Parse {
            src: self.src.into(),
            offset: self.src.len(),
            message: "expected a unit, `1`, or `(`, found end of input".into(),
        })?;
        match spanned.token {
            Token::Int(1) => Ok(Unit::Delta(DeltaUnit::dimensionless())),
            Token::Ident(name) => (self.resolve)(&name),
            Token::LParen => {
                let inner = self.parse_expr()?;
                self.expect_rparen()?;
                Ok(inner)
            }
            Token::Int(_) | Token::Float(_) => {
                let message = "numeric factors are not allowed in unit expressions";
                Err(UnitError::Parse {
                    src: self.src.into(),
                    offset: spanned.offset,
                    message: message.into(),
                })
            }
            token => {
                let message = format!("expected a unit, `1`, or `(`, found {token}");
                Err(UnitError::Parse {
                    src: self.src.into(),
                    offset: spanned.offset,
                    message,
                })
            }
        }
    }

    fn parse_term(&mut self) -> Result<Unit, UnitError> {
        let base = self.parse_factor()?;
        if let Some(Ok(Spanned {
            token: Token::Superscript(n),
            ..
        })) = self.tokens.peek()
        {
            let n = *n;
            self.advance()?;
            return base.pow(Exp::int(n));
        }
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

    fn parse_expr(&mut self) -> Result<Unit, UnitError> {
        let mut result = self.parse_term()?;
        loop {
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
    use crate::test_utils::{errors_match, mks_registry, parse_error_at};

    #[test]
    fn name_and_symbol_agree() {
        let ureg = mks_registry();
        assert_eq!(ureg.parse("meter").unwrap(), ureg.parse("m").unwrap());
    }

    #[test]
    fn product_quotient_and_powers() {
        let ureg = mks_registry();
        let m = ureg.get("m").unwrap();
        let s = ureg.get("s").unwrap();
        let kg = ureg.get("kg").unwrap();
        let m2 = m.pow(Exp::int(2)).unwrap();
        let s2 = s.pow(Exp::int(2)).unwrap();
        let kg_m2_over_s2 = kg.try_mul(&m2).unwrap().try_div(&s2).unwrap();
        assert_eq!(ureg.parse("kg*m^2/s^2").unwrap(), kg_m2_over_s2);
    }

    #[test]
    fn star_dot_and_times_are_multiplication() {
        let ureg = mks_registry();
        let m = ureg.get("m").unwrap();
        let kg = ureg.get("kg").unwrap();
        let kg_m = kg.try_mul(&m).unwrap();
        assert_eq!(ureg.parse("kg*m").unwrap(), kg_m);
        assert_eq!(ureg.parse("kg·m").unwrap(), kg_m);
        assert_eq!(ureg.parse("kg×m").unwrap(), kg_m);
    }

    #[test]
    fn superscripts_match_carets() {
        let ureg = mks_registry();
        assert_eq!(ureg.parse("m²").unwrap(), ureg.parse("m^2").unwrap());
        assert_eq!(ureg.parse("s⁻¹").unwrap(), ureg.parse("s^-1").unwrap());
    }

    #[test]
    fn fractional_exponent() {
        let ureg = mks_registry();
        let m = ureg.get("m").unwrap();
        let sqrt_m = m.pow(Exp::new(1, 2).unwrap()).unwrap();
        assert_eq!(ureg.parse("m^(1/2)").unwrap(), sqrt_m);
    }

    #[test]
    fn one_is_dimensionless() {
        let ureg = mks_registry();
        let s = ureg.get("s").unwrap();
        let s_recip = s.recip().unwrap();
        assert_eq!(
            ureg.parse("1").unwrap(),
            Unit::Delta(DeltaUnit::dimensionless())
        );
        assert_eq!(ureg.parse("1/s").unwrap(), s_recip);
    }

    #[test]
    fn division_is_left_associative() {
        let ureg = mks_registry();
        assert_eq!(ureg.parse("m/s/K").unwrap(), ureg.parse("m/(s*K)").unwrap())
    }

    #[test]
    fn lone_point_unit() {
        let ureg = mks_registry();
        assert!(matches!(ureg.parse("degC").unwrap(), Unit::Point(_)));
    }

    #[test]
    fn point_in_compound_is_not_composable() {
        let ureg = mks_registry();
        let cases = ["degC/s", "degC^2", "1/degC", "s*degC"];
        let expected = UnitError::NotComposable {
            name: "degree_Celsius".into(),
        };
        for expr in cases {
            let actual = ureg.parse(expr).unwrap_err();
            assert!(errors_match(&actual, &expected), "{expr}: {actual:?}");
        }
    }

    #[test]
    fn rejects_numeric_factor() {
        let ureg = mks_registry();
        let cases = [("2*m", 0), ("1.5*m", 0)];
        for (expr, offset) in cases {
            let actual = ureg.parse(expr).unwrap_err();
            let expected = parse_error_at(offset);
            assert!(errors_match(&actual, &expected), "{expr}: {actual:?}");
        }
    }

    #[test]
    fn rejects_unknown_unit() {
        let ureg = mks_registry();
        let actual = ureg.parse("unknown").unwrap_err();
        let expected = UnitError::UnknownUnit {
            name: "unknown".into(),
            registry: ureg.name().into(),
        };
        assert!(errors_match(&actual, &expected));
    }

    #[test]
    fn syntax_errors_report_offset() {
        let ureg = mks_registry();
        let cases = [("m*", 2), ("(m", 2), ("m^", 2), ("*m", 0)];
        for (expr, offset) in cases {
            let actual = ureg.parse(expr).unwrap_err();
            let expected = parse_error_at(offset);
            assert!(errors_match(&actual, &expected), "{expr}: {actual:?}");
        }
    }

    #[test]
    fn display_round_trips() {
        let ureg = mks_registry();
        let m = ureg.get("m").unwrap();
        let s = ureg.get("s").unwrap();
        let kg = ureg.get("kg").unwrap();
        let kelvin = ureg.get("K").unwrap();
        let joule = ureg.get("J").unwrap();
        let s_recip = s.recip().unwrap();
        let sqrt_m = m.pow(Exp::new(1, 2).unwrap()).unwrap();
        let m2 = m.pow(Exp::int(2)).unwrap();
        let s2 = s.pow(Exp::int(2)).unwrap();
        let heat_capacity_unit = kg
            .try_mul(&m2)
            .unwrap()
            .try_div(&s2)
            .unwrap()
            .try_div(&kelvin)
            .unwrap();
        let cases = [joule, heat_capacity_unit, s_recip, sqrt_m];
        for unit in cases {
            assert_eq!(ureg.parse(&unit.to_string()).unwrap(), unit);
            assert_eq!(ureg.parse(&format!("{unit:#}")).unwrap(), unit);
        }
    }
}
