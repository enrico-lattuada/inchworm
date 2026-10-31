//! Folding of look-alike code points in registry keys.

use std::borrow::Cow;

/// Look-alike code points folded to one form in registry keys, each mapped to
/// what Unicode normalization gives it (NFC for the canonical equivalents,
/// NFKC for the compatibility ones).
const FOLDS: [(char, &str); 6] = [
    ('\u{00B5}', "\u{03BC}"),  // micro sign → Greek small mu (NFKC)
    ('\u{2126}', "\u{03A9}"),  // ohm sign → Greek capital omega (NFC)
    ('\u{212B}', "\u{00C5}"),  // angstrom sign → Latin capital A with ring (NFC)
    ('\u{212A}', "K"),         // kelvin sign → Latin capital K (NFC)
    ('\u{2103}', "\u{00B0}C"), // degree Celsius sign → degree sign + C (NFKC)
    ('\u{2109}', "\u{00B0}F"), // degree Fahrenheit sign → degree sign + F (NFKC)
];

/// The replacement for `c` in a registry key, if it is a look-alike.
fn fold_char(c: char) -> Option<&'static str> {
    FOLDS
        .iter()
        .find(|&(from, _)| *from == c)
        .map(|&(_, to)| to)
}

/// `name` as a registry key: look-alike code points folded to one form, so
/// `µm` (micro sign) and `μm` (Greek mu) find the same unit. Borrows `name`
/// unless something needs folding.
pub(crate) fn lookup_key(name: &str) -> Cow<'_, str> {
    if !name.chars().any(|c| fold_char(c).is_some()) {
        Cow::Borrowed(name)
    } else {
        let mut key = String::with_capacity(name.len());
        for c in name.chars() {
            match fold_char(c) {
                None => key.push(c),
                Some(fold) => key.push_str(fold),
            }
        }
        Cow::Owned(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::assert_matches;

    #[test]
    fn borrows_when_nothing_to_fold() {
        assert_matches!(lookup_key("km"), Cow::Borrowed(_));
    }

    #[test]
    fn folds_look_alikes() {
        let cases = [
            ("\u{B5}m", "\u{3BC}m"),
            ("m\u{2126}", "m\u{03A9}"),
            ("m\u{212B}", "m\u{00C5}"),
            ("\u{212A}", "K"),
            ("\u{2103}", "\u{00B0}C"),
            ("\u{2109}", "\u{00B0}F"),
            ("a\u{2103}b", "a\u{B0}Cb"),
        ];
        for (from, to) in cases {
            let dbg_from = from.escape_unicode();
            let dbg_to = to.escape_unicode();
            assert_eq!(lookup_key(from), to, "{dbg_from} -> {dbg_to}");
        }
    }

    #[test]
    fn keeps_other_chars() {
        assert_matches!(lookup_key("Θ%"), Cow::Borrowed("Θ%"));
    }
}
