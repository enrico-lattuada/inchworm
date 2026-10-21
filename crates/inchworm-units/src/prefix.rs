use crate::scale::Scale;

/// A named multiplicative prefix (e.g., SI's "kilo") that combines with a
/// prefixable unit atom to lazily derive a new, interned atom.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Prefix {
    /// Prefix name, e.g. "kilo".
    pub name: Box<str>,
    /// Prefix symbol, e.g. "k".
    pub symbol: Box<str>,
    /// Multiplicative factor, e.g. `1e3`.
    pub factor: Scale,
}
