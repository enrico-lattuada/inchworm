//! Unit-expression grammar: tokenizer + recursive-descent parser.
//!
//! ```text
//! expr     := term { ("*" | "·" | "×" | "/" | ADJ) term }
//! term     := factor [ ("^" ["-"] ("(" exponent ")" | INT)) | SUPERSCRIPT ]
//! factor   := IDENT | NUMBER | "(" expr ")"       (NUMBER other than 1 only in definitions)
//! exponent := ["-"] INT [ "/" INT ]
//! ADJ      := no operator: the next term follows directly, same precedence as "*"
//! IDENT    := one or more chars accepted by inchworm_dimensions::is_ident_char that are not operators;
//!             not starting with an ASCII digit
//! NUMBER   := DIGIT+ ["." DIGIT*] [("e" | "E") ["+" | "-"] DIGIT+]
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

/// A unit expression with a numeric factor, as written in a definition: `3600 s` is `3600 × s`.
/// Outside definitions the factor is always `1`.
#[derive(Debug)]
pub(crate) struct UnitExpression {
    factor: f64,
    unit: Unit,
}

impl UnitExpression {
    /// The number `n` as an expression: factor `n`, dimensionless unit.
    fn from_number(n: f64) -> Self {
        Self {
            factor: n,
            unit: Unit::Delta(DeltaUnit::dimensionless()),
        }
    }

    /// The unit `unit` as an expression: factor `1`.
    fn from_unit(unit: Unit) -> Self {
        Self { factor: 1.0, unit }
    }

    /// The product: factors multiply, units combine with [`Unit::try_mul`].
    fn try_mul(self, rhs: &Self) -> Result<Self, UnitError> {
        let factor = self.factor * rhs.factor;
        let unit = self.unit.try_mul(&rhs.unit)?;
        Ok(Self { factor, unit })
    }

    /// The quotient: factors divide, units combine with [`Unit::try_div`].
    fn try_div(self, rhs: &Self) -> Result<Self, UnitError> {
        let factor = self.factor / rhs.factor;
        let unit = self.unit.try_div(&rhs.unit)?;
        Ok(Self { factor, unit })
    }

    /// Raises both the factor and the unit to `exp`: `(2 m)^2` is `4 m²`.
    fn pow(self, exp: Exp) -> Result<Self, UnitError> {
        let factor = self.factor.powf(exp.to_f64());
        let unit = self.unit.pow(exp)?;
        Ok(Self { factor, unit })
    }

    /// The factor and the unit.
    pub(crate) fn into_parts(self) -> (f64, Unit) {
        (self.factor, self.unit)
    }
}

/// Which numbers the parser accepts as factors.
#[derive(Debug, PartialEq)]
enum Mode {
    /// Unit expressions (`parse`): only `1`.
    Unit,
    /// Definition expressions: any number.
    Definition,
}

/// Parses `src` against a name resolver (the registry), accepting the numbers `mode` allows.
/// Errors carry byte offsets so diagnostics can point into the source.
fn parse<'a>(
    src: &'a str,
    resolve: &'a dyn Fn(&str) -> Result<Unit, UnitError>,
    mode: Mode,
) -> Result<UnitExpression, UnitError> {
    let lexer = Lexer::new(src);
    let tokens = lexer.peekable();
    let mut parser = Parser {
        tokens,
        src,
        resolve,
        mode,
    };
    let expr = parser.parse_expr()?;
    match parser.advance()? {
        None => Ok(expr),
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

/// Parses a unit expression: the only number allowed is `1`, reported at its offset otherwise.
pub(crate) fn parse_unit_expr<'a>(
    src: &'a str,
    resolve: &'a dyn Fn(&str) -> Result<Unit, UnitError>,
) -> Result<Unit, UnitError> {
    let (factor, unit) = parse(src, resolve, Mode::Unit)?.into_parts();
    debug_assert_eq!(factor, 1.0);
    Ok(unit)
}

/// Parses a definition expression: any positive number is a factor (`3600 s`, `1/12 ft`).
pub(crate) fn parse_definition<'a>(
    src: &'a str,
    resolve: &'a dyn Fn(&str) -> Result<Unit, UnitError>,
) -> Result<UnitExpression, UnitError> {
    parse(src, resolve, Mode::Definition)
}

pub(crate) fn is_valid_ident(src: &str) -> bool {
    let lexer = Lexer::new(src);
    let mut tokens = Vec::new();
    for item in lexer {
        match item {
            Ok(spanned) => tokens.push(spanned.token),
            Err(_) => return false,
        }
    }
    matches!(tokens.as_slice(), [Token::Ident(ident)] if ident == src)
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

/// The operator token `c` stands for, if any.
fn operator_token(c: char) -> Option<Token> {
    match c {
        MUL_CHAR | PRETTY_MUL_CHAR | ALT_PRETTY_MUL_CHAR => Some(Token::Star),
        MINUS_CHAR => Some(Token::Minus),
        SLASH_CHAR => Some(Token::Slash),
        CARET_CHAR => Some(Token::Caret),
        LPAREN_CHAR => Some(Token::LParen),
        RPAREN_CHAR => Some(Token::RParen),
        _ => None,
    }
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
    fn new(src: &'a str) -> Self {
        Self {
            src,
            chars: src.char_indices().peekable(),
        }
    }

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

/// The byte length of a valid exponent part (`e3`, `E-12`, `e+2`) at the start of `rest`,
/// or 0 if there is none.
fn exponent_len(rest: &str) -> usize {
    let Some(after_e) = rest.strip_prefix(['e', 'E']) else {
        return 0;
    };
    let after_sign = after_e.strip_prefix(['+', '-']).unwrap_or(after_e);
    let digits = after_sign.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        0
    } else {
        rest.len() - after_sign.len() + digits
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

/// Whether `c` can be part of an identifier: anything except whitespace,
/// control characters, operators, superscripts, and reserved characters.
fn is_ident_char(c: char) -> bool {
    inchworm_dimensions::is_ident_char(c) && operator_token(c).is_none()
}

/// Whether token can begin a factor, so that it multiplies by adjacency.
fn starts_factor(token: &Token) -> bool {
    matches!(
        token,
        Token::Ident(_) | Token::Int(_) | Token::Float(_) | Token::LParen
    )
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
            let end = start + consumed.len();
            let exp_len = exponent_len(&self.src[end..]);
            for _ in 0..exp_len {
                self.chars.next();
            }
            let literal = &self.src[start..end + exp_len];
            if let Ok(number) = literal.parse::<i64>() {
                let (token, offset) = (Token::Int(number), start);
                Some(Ok(Spanned { token, offset }))
            } else {
                match literal.parse::<f64>() {
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
        } else if let Some(token) = operator_token(c) {
            self.chars.next();
            let offset = start;
            Some(Ok(Spanned { token, offset }))
        } else if c.is_control() {
            self.chars.next();
            Some(Err(UnitError::Parse {
                src: self.src.into(),
                offset: start,
                message: format!("unsupported control character {c:?}"),
            }))
        } else if !inchworm_dimensions::is_ident_char(c) {
            self.chars.next();
            Some(Err(UnitError::Parse {
                src: self.src.into(),
                offset: start,
                message: format!("reserved character {c:?}"),
            }))
        } else {
            let consumed = self.consume_while(start, is_ident_char);
            let (token, offset) = (Token::Ident(consumed.into()), start);
            Some(Ok(Spanned { token, offset }))
        }
    }
}

struct Parser<'a> {
    tokens: Peekable<Lexer<'a>>,
    src: &'a str,
    resolve: &'a dyn Fn(&str) -> Result<Unit, UnitError>,
    mode: Mode,
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

    fn parse_exponent(&mut self) -> Result<Exp, UnitError> {
        let negative = self.consume_if(&Token::Minus)?;
        let num = self.expect_int()?;
        let num = if negative { -num } else { num };
        if self.consume_if(&Token::Slash)? {
            let den = self.expect_int()?;
            Exp::new(num, den).map_err(UnitError::Dimension)
        } else {
            Ok(Exp::int(num))
        }
    }

    fn parse_factor(&mut self) -> Result<UnitExpression, UnitError> {
        let spanned = self.advance()?.ok_or_else(|| {
            let message = match self.mode {
                Mode::Unit => "expected a unit, `1`, or `(`, found end of input",
                Mode::Definition => "expected a unit, a number, or `(`, found end of input",
            };
            UnitError::Parse {
                src: self.src.into(),
                offset: self.src.len(),
                message: message.into(),
            }
        })?;
        match spanned.token {
            Token::Int(1) => Ok(UnitExpression::from_number(1.0)),
            Token::Ident(name) => Ok(UnitExpression::from_unit((self.resolve)(&name)?)),
            Token::LParen => {
                let inner = self.parse_expr()?;
                self.expect_rparen()?;
                Ok(inner)
            }
            Token::Int(n) if self.mode == Mode::Definition => {
                Ok(UnitExpression::from_number(n as f64))
            }
            Token::Float(n) if self.mode == Mode::Definition => Ok(UnitExpression::from_number(n)),
            Token::Int(_) | Token::Float(_) => {
                let message = "numeric factors are not allowed in unit expressions";
                Err(UnitError::Parse {
                    src: self.src.into(),
                    offset: spanned.offset,
                    message: message.into(),
                })
            }
            token => {
                let message = match self.mode {
                    Mode::Unit => format!("expected a unit, `1`, or `(`, found {token}"),
                    Mode::Definition => format!("expected a unit, a number, or `(`, found {token}"),
                };
                Err(UnitError::Parse {
                    src: self.src.into(),
                    offset: spanned.offset,
                    message,
                })
            }
        }
    }

    fn parse_term(&mut self) -> Result<UnitExpression, UnitError> {
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
        let exp = if parenthesized {
            self.parse_exponent()?
        } else {
            let n = self.expect_int()?;
            Exp::int(n)
        };
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

    fn parse_expr(&mut self) -> Result<UnitExpression, UnitError> {
        let mut result = self.parse_term()?;
        loop {
            if self.consume_if(&Token::Star)? {
                let rhs = self.parse_term()?;
                result = result.try_mul(&rhs)?;
            } else if self.consume_if(&Token::Slash)? {
                let rhs = self.parse_term()?;
                result = result.try_div(&rhs)?;
            } else if let Some(Ok(Spanned { token, .. })) = self.tokens.peek()
                && starts_factor(token)
            {
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
    use crate::{
        DerivedDef,
        test_utils::{errors_match, mks_registry, parse_error_at},
    };

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
    fn adjacency_is_multiplication() {
        let ureg = mks_registry();
        assert_eq!(ureg.parse("kg m").unwrap(), ureg.parse("kg·m").unwrap());
        assert_eq!(ureg.parse("m(s)").unwrap(), ureg.parse("m·s").unwrap());
        assert_eq!(ureg.parse("m²s").unwrap(), ureg.parse("m²·s").unwrap());
    }

    #[test]
    fn adjacency_groups_left_to_right() {
        let ureg = mks_registry();
        assert_eq!(
            ureg.parse("J / kg K").unwrap(),
            ureg.parse("J·K/kg").unwrap()
        );
    }

    #[test]
    fn adjacency_mixes_with_operator() {
        let ureg = mks_registry();
        assert_eq!(
            ureg.parse("kg m / s").unwrap(),
            ureg.parse("kg·m/s").unwrap()
        );
        assert_eq!(ureg.parse("m / s s").unwrap(), ureg.parse("m").unwrap());
    }

    #[test]
    fn allows_unit_one_adjacent() {
        let ureg = mks_registry();
        assert_eq!(ureg.parse("1 m").unwrap(), ureg.parse("m").unwrap());
    }

    #[test]
    fn rejects_adjacent_numeric_factor() {
        let ureg = mks_registry();
        let actual = ureg.parse("m 2").unwrap_err();
        let expected = parse_error_at(2);
        assert!(errors_match(&actual, &expected));
    }

    #[test]
    fn point_unit_adjacent_is_not_composable() {
        let ureg = mks_registry();
        let cases = ["degC m", "m degC"];
        let expected = UnitError::NotComposable {
            name: "degree_Celsius".into(),
        };
        for expr in cases {
            let actual = ureg.parse(expr).unwrap_err();
            assert!(errors_match(&actual, &expected), "{expr}");
        }
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
        let cases = [("2*m", 0), ("1.5*m", 0), ("1e3 m", 0), ("m*1e-3", 2)];
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
        let cases = [
            ("m*", 2),
            ("(m", 2),
            ("m^", 2),
            ("*m", 0),
            ("m^--2", 3),
            ("m^2.5", 2),
        ];
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

    #[track_caller]
    fn tokens(src: &str) -> Vec<Token> {
        let lexer = Lexer::new(src);
        lexer
            .map(|item| item.map(|spanned| spanned.token))
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    #[test]
    fn exponent_len_measures_exponent_part() {
        let cases = [
            ("e3m", 2),
            ("E-12", 4),
            ("e+2", 3),
            ("em", 0),
            ("e-m", 0),
            ("e", 0),
            ("", 0),
            ("m", 0),
        ];
        for (rest, expected) in cases {
            assert_eq!(exponent_len(rest), expected, "{rest:?}");
        }
    }

    #[test]
    fn lexes_scientific_notation() {
        let cases = [
            ("1e3", 1e3),
            ("1.5e-3", 1.5e-3),
            ("6.02214076E23", 6.022_140_76e23),
            ("2e+2", 2e2),
        ];
        for (src, expected) in cases {
            assert_eq!(tokens(src), [Token::Float(expected)], "{src}");
        }
    }

    #[test]
    fn e_without_digits_is_an_identifier() {
        assert_eq!(tokens("2em"), [Token::Int(2), Token::Ident("em".into())]);
        assert_eq!(tokens("2e"), [Token::Int(2), Token::Ident("e".into())]);
        assert_eq!(
            tokens("2e-m"),
            [
                Token::Int(2),
                Token::Ident("e".into()),
                Token::Minus,
                Token::Ident("m".into()),
            ]
        );
    }

    #[test]
    fn exponent_ends_number() {
        assert_eq!(
            tokens("2e3m"),
            [Token::Float(2e3), Token::Ident("m".into())]
        );
    }

    #[test]
    fn accepts_non_ascii_identifiers() {
        let cases = ["µm", "Ω", "Θ", "°C", "′", "″", "%", "‰"];
        for name in cases {
            assert!(is_valid_ident(name), "{name:?} is not a valid ident")
        }
    }

    #[test]
    fn accepts_quote_symbols() {
        assert!(is_valid_ident("'"));
        assert!(is_valid_ident("\""));
    }

    #[test]
    fn superscript_ends_identifier() {
        assert_eq!(
            tokens("Ω²"),
            vec![Token::Ident("Ω".into()), Token::Superscript(2)],
        );
        assert_eq!(
            tokens("m⁻¹"),
            vec![Token::Ident("m".into()), Token::Superscript(-1)],
        );
    }

    #[test]
    fn operators_end_identifier() {
        let cases = [
            ("Ω*s", Token::Star),
            ("Ω·s", Token::Star),
            ("Ω×s", Token::Star),
            ("Ω/s", Token::Slash),
        ];
        for (src, op) in cases {
            assert_eq!(
                tokens(src),
                vec![Token::Ident("Ω".into()), op, Token::Ident("s".into())]
            );
        }
    }

    #[test]
    fn rejects_reserved_chars() {
        let cases = ['.', ',', '+', '=', '[', ']', '{', '}', ';', ':'];
        for r in cases {
            let src = format!("m{r}s");
            assert!(!is_valid_ident(&src), "{src} is a valid ident");
        }
    }

    #[test]
    fn rejects_surrounding_whitespace() {
        let cases = [" m", "m ", "m\u{A0}"];
        for src in cases {
            assert!(!is_valid_ident(src), "{src:?} is valid ident");
        }
    }

    #[test]
    fn reports_reserved_char() {
        let ureg = mks_registry();
        let UnitError::Parse {
            offset, message, ..
        } = ureg.parse("m;s").unwrap_err()
        else {
            panic!("not a parse error")
        };
        assert_eq!(offset, 1);
        assert_eq!(message, "reserved character ';'");
    }

    #[test]
    fn rejects_control_chars() {
        let ureg = mks_registry();
        let cases = [
            ("m\u{7}", "unsupported control character '\\u{7}'"),
            ("m\u{80}", "unsupported control character '\\u{80}'"),
        ];
        for (expr, expected_message) in cases {
            let UnitError::Parse {
                offset, message, ..
            } = ureg.parse(expr).unwrap_err()
            else {
                panic!("not a parse error")
            };
            assert_eq!(offset, 1);
            assert_eq!(message, expected_message);
        }
    }

    #[test]
    fn reports_byte_offsets_after_multibyte_chars() {
        let mut ureg = mks_registry();
        let Unit::Delta(m) = ureg.get("m").unwrap() else {
            panic!("m not a delta unit")
        };
        ureg.add_derived(DerivedDef::new("micrometre", "µm", &m).scale(1e-6))
            .unwrap();
        let actual = ureg.parse("µm / .").unwrap_err();
        let expected = UnitError::Parse {
            src: "µm / .".into(),
            offset: 6,
            message: "".into(),
        };
        assert!(errors_match(&actual, &expected));
    }

    mod properties {
        use super::*;
        use crate::test_utils::{build, factors};
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn display_round_trips(factors in factors()) {
                let ureg = mks_registry();
                let unit = Unit::Delta(build(&ureg, &factors));
                prop_assert_eq!(&ureg.parse(&unit.to_string()).unwrap(), &unit);
                prop_assert_eq!(&ureg.parse(&format!("{unit:#}")).unwrap(), &unit);
            }

            #[test]
            fn adjacency_equals_star(factors in factors()) {
                let ureg = mks_registry();
                let unit = Unit::Delta(build(&ureg, &factors));
                let adj_expr = unit.to_string().replace('*', " ");
                prop_assert_eq!(&ureg.parse(&adj_expr).unwrap(), &unit);
            }
        }
    }

    mod parse_definition {
        use super::*;
        use crate::{UnitRegistry, test_utils::assert_close};

        fn assert_definition(
            ureg: &UnitRegistry,
            src: &str,
            expected_factor: f64,
            expected_unit: &str,
        ) {
            let (factor, unit) = ureg.parse_definition(src).unwrap().into_parts();
            assert_close(factor, expected_factor);
            assert_eq!(unit, ureg.parse(expected_unit).unwrap(), "{src}");
        }

        #[test]
        fn accepts_numbers() {
            let ureg = mks_registry();
            let cases = [
                ("3600 s", 3600.0, "s"),
                ("4.184 J", 4.184, "J"),
                ("1.5e-3 m", 1.5e-3, "m"),
                ("1 m", 1.0, "m"),
            ];
            for (src, factor, unit) in cases {
                assert_definition(&ureg, src, factor, unit);
            }
        }

        #[test]
        fn combines_factors() {
            let ureg = mks_registry();
            let cases = [
                ("1/12 m", 1.0 / 12.0, "m"),
                ("1 / 2 m", 0.5, "m"),
                ("2 * 3 m", 6.0, "m"),
                ("1000 m / 10 s", 100.0, "m*s"),
                ("1000 m / (10 s)", 100.0, "m/s"),
            ];
            for (src, factor, unit) in cases {
                assert_definition(&ureg, src, factor, unit);
            }
        }

        #[test]
        fn powers_raise_the_factor() {
            let ureg = mks_registry();
            let cases = [
                ("10^3 m", 1e3, "m"),
                ("10^-3 m", 1e-3, "m"),
                ("(2 m)^2", 4.0, "m^2"),
                ("(2 m)²", 4.0, "m^2"),
                ("(4 m^2)^(1/2)", 2.0, "m"),
            ];
            for (src, factor, unit) in cases {
                assert_definition(&ureg, src, factor, unit);
            }
        }

        #[test]
        fn rejects_point_with_number() {
            let ureg = mks_registry();
            let actual = ureg.parse_definition("2 degC").unwrap_err();
            let expected = UnitError::NotComposable {
                name: "degree_Celsius".into(),
            };
            assert!(errors_match(&actual, &expected), "{actual:?}");
        }
    }
}
