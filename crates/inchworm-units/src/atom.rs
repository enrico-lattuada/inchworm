use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

use inchworm_dimensions::Dimension;

/// Process-unique identity, assigned from a global counter at registration.
///
/// Never reused: removing and re-adding a name yields a *new* atom, so
/// dimensions built before the removal are distinct from ones built after.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnitId(u64);

/// Process-unique registry identity, used to detect cross-registry mixing.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct UnitRegistryId(u64);

static NEXT_ATOM_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_REGISTRY_ID: AtomicU64 = AtomicU64::new(1);

impl UnitId {
    pub(crate) fn next() -> Self {
        let id = NEXT_ATOM_ID.fetch_add(1, Ordering::Relaxed);
        assert_ne!(
            id, 0,
            "AtomId space exhausted: counter wrapped past u64::MAX"
        );
        Self(id)
    }
}

impl UnitRegistryId {
    pub(crate) fn next() -> Self {
        let id = NEXT_REGISTRY_ID.fetch_add(1, Ordering::Relaxed);
        assert_ne!(
            id, 0,
            "RegistryId space exhausted: counter wrapped past u64::MAX"
        );
        Self(id)
    }
}

/// How a unit atom's raw numeric value relates to the coherent unit of its dimension.
///
/// `Linear` and `LogRatio` are "delta"-like: freely composable in compound
/// `Unit` expressions. `Affine` and `LogLevel` are "point"-like: anchored to
/// an absolute reference, so two quantities of that kind can never be added
/// to each other: restricted to standalone, power-1 use in compounds. See
/// [`ConversionKind::is_point`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ConversionKind {
    /// `coherent = raw * scale`. The default case (meter, second, radian, a
    /// gain expressed as a bare ratio...).
    Linear {
        /// Factor to the coherent unit of this atom's dimension.
        scale: f64,
    },
    /// `coherent = raw * scale + offset`. Anchored ("point"-like); restricted
    /// to standalone, power-1 use. Fahrenheit needs both fields; Celsius
    /// happens to have `scale == 1.0`.
    Affine {
        /// Factor to the coherent unit of this atom's dimension.
        scale: f64,
        /// Additive shift, in the coherent unit, at `raw == 0.0`.
        offset: f64,
    },
    /// `level = multiplier * log_base.log(ratio)`, where `ratio` is a bare,
    /// reference-free ratio between two same-dimensioned quantities (an
    /// amplifier gain in dB, a frequency ratio in decades). Unanchored
    /// ("delta"-like); always a dimensionless kind.
    LogRatio {
        /// Multiplies the chosen logarithm (e.g. 10 for a power-ratio dB).
        multiplier: f64,
        /// Logarithm base (10 for dB/decade, 2 for octave, e for neper).
        log_base: f64,
    },
    /// `level = multiplier * log_base.log(raw / reference)`, anchored to an
    /// absolute quantity of the unit's own dimension (dBm: `reference` = 1 mW
    /// in coherent units; pH: `reference` = 1 mol/L, `multiplier` = -1).
    /// Anchored ("point"-like); restricted to standalone, power-1 use.
    LogLevel {
        /// Multiplies the chosen logarithm.
        multiplier: f64,
        /// Logarithm base.
        log_base: f64,
        /// The absolute reference quantity, in the coherent unit of this
        /// atom's dimension.
        reference: f64,
    },
}

impl ConversionKind {
    /// Returns `true` if this conversion is anchored to an absolute
    /// reference ([`Affine`](Self::Affine) or [`LogLevel`](Self::LogLevel)) —
    /// meaning two quantities of this kind can never be added to each other.
    pub(crate) fn is_point(&self) -> bool {
        matches!(self, Self::Affine { .. } | Self::LogLevel { .. })
    }

    /// The complement of [`is_point`](Self::is_point).
    pub(crate) fn is_delta(&self) -> bool {
        !self.is_point()
    }
}

#[derive(Debug)]
pub(crate) struct UnitData {
    /// This atom's process-unique identity.
    pub id: UnitId,
    /// The registry that created this atom.
    pub registry_id: UnitRegistryId,
    /// Unit name (e.g. "meter").
    pub name: Box<str>,
    /// Unit symbol.
    pub symbol: Box<str>,
    /// Unit dimension.
    pub dimension: Dimension,
    /// How this atom's raw value relates to the coherent unit of `dimension`.
    pub conversion: ConversionKind,
}

impl UnitData {
    /// See [`ConversionKind::is_point`].
    pub(crate) fn is_point(&self) -> bool {
        self.conversion.is_point()
    }
}

impl PartialEq for UnitData {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for UnitData {}

impl PartialOrd for UnitData {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for UnitData {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl Hash for UnitData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod unit_id {
        use super::*;

        #[test]
        fn next() {
            assert_ne!(UnitId::next(), UnitId::next());
        }
    }

    mod unit_registry_id {
        use super::*;

        #[test]
        fn next() {
            assert_ne!(UnitRegistryId::next(), UnitRegistryId::next());
        }
    }

    mod conversion_kind {
        use super::*;

        #[test]
        fn is_point() {
            let cases = [
                (ConversionKind::Linear { scale: 1.0 }, false),
                (
                    ConversionKind::Affine {
                        scale: 1.0,
                        offset: 0.0,
                    },
                    true,
                ),
                (
                    ConversionKind::LogRatio {
                        multiplier: 10.0,
                        log_base: 10.0,
                    },
                    false,
                ),
                (
                    ConversionKind::LogLevel {
                        multiplier: 10.0,
                        log_base: 10.0,
                        reference: 1.0,
                    },
                    true,
                ),
            ];
            for (conversion_kind, expected_point) in cases {
                assert_eq!(conversion_kind.is_point(), expected_point);
                assert_eq!(conversion_kind.is_delta(), !expected_point);
            }
        }
    }
}
