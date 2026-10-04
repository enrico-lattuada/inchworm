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

use crate::UnitError;

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
