use std::{
    cmp::Ordering,
    fmt::{self, Write},
};

use inchworm_dimensions::{Dimension, Exp};
use smallvec::{SmallVec, smallvec};

use crate::{
    UnitError, UnitRegistryId,
    atom::UnitAtom,
    parse::{
        CARET_CHAR, LPAREN_CHAR, MUL_CHAR, PRETTY_MUL_CHAR, RPAREN_CHAR, SLASH_CHAR,
        UNITARY_IDENT_CHAR, digit_to_superscript,
    },
};

const MAX_INLINE_FACTORS: usize = 4;

/// A unit expression: a reduced product of powers over named unit atoms.
///
/// A free value: once built, a `DeltaUnit` never needs its originating unit
/// registry again. Caches the product of its factors' dimensions,
/// so dimension compatibility is O(1) lookup.
///
/// Invariants:
/// - sorted by [`UnitId`](crate::UnitId) ascending
/// - no zero exponents
/// - no duplicates.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeltaUnit {
    factors: SmallVec<[(UnitAtom, Exp); MAX_INLINE_FACTORS]>,
    /// Cached product of factor dimensions.
    dimension: Dimension,
}

impl DeltaUnit {
    /// Returns an empty (dimensionless) unit.
    pub fn empty() -> Self {
        Self {
            factors: SmallVec::new(),
            dimension: Dimension::dimensionless(),
        }
    }

    /// Returns a unit with a single factor at power `exp`, or an empty
    /// (dimensionless) unit if `exp` is zero.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::NotExponentiable`] if `atom`'s conversion is anchored and
    /// `exp` is not `1`.
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`DimensionError::ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if raising `atom`'s dimension to `exp` overflows.
    pub(crate) fn single(atom: &UnitAtom, exp: Exp) -> Result<Self, UnitError> {
        atom.check_exponentiable(exp)?;
        if exp.is_zero() {
            return Ok(Self::empty());
        }
        let factors = smallvec![(atom.clone(), exp)];
        let dimension = atom.dimension.pow(exp)?;
        Ok(Self { factors, dimension })
    }

    /// This unit's dimension: the cached product of its factors' dimensions.
    pub fn dimension(&self) -> &Dimension {
        &self.dimension
    }

    /// The identity of the unit registry that minted this value's atoms, or
    /// `None` if it has no factors (a bare dimensionless `DeltaUnit`).
    pub(crate) fn registry_id(&self) -> Option<UnitRegistryId> {
        self.factors.first().map(|(atom, _)| atom.registry_id)
    }

    /// The factors of this unit.
    pub(crate) fn factors(&self) -> &[(UnitAtom, Exp)] {
        &self.factors
    }
}

// ---- Algebra ----
impl DeltaUnit {
    /// Merges two units, combining exponents of shared atoms, pruning any that cancel to zero.
    ///
    /// # Errors
    /// Returns [`UnitError::CrossRegistry`] if `self` and `rhs` were minted by
    /// different unit registries.
    /// Returns [`UnitError::NotComposable`] if `self` or `rhs` are anchored.
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`DimensionError::ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if combining a shared atom's exponents overflows, or
    /// [`DimensionError::CrossRegistry`](inchworm_dimensions::DimensionError::CrossRegistry)
    /// if `self` and `rhs`'s dimensions come from different dimension registries.
    pub fn try_mul(&self, rhs: &Self) -> Result<Self, UnitError> {
        if let (Some(lhs_id), Some(rhs_id)) = (self.registry_id(), rhs.registry_id())
            && lhs_id != rhs_id
        {
            return Err(UnitError::CrossRegistry {
                left: lhs_id,
                right: rhs_id,
            });
        }
        if let Some((atom, _)) = self
            .factors()
            .iter()
            .chain(rhs.factors())
            .find(|(atom, _)| atom.is_point())
        {
            return Err(UnitError::NotComposable {
                name: atom.name.to_string(),
                registry_id: atom.registry_id,
            });
        }
        let mut factors = SmallVec::new();
        let (mut i, mut j) = (0, 0);
        while i < self.factors.len() && j < rhs.factors.len() {
            let (id_a, exp_a) = &self.factors[i];
            let (id_b, exp_b) = &rhs.factors[j];
            match id_a.cmp(id_b) {
                Ordering::Less => {
                    factors.push((id_a.clone(), *exp_a));
                    i += 1;
                }
                Ordering::Greater => {
                    factors.push((id_b.clone(), *exp_b));
                    j += 1;
                }
                Ordering::Equal => {
                    let exp = exp_a.checked_add(*exp_b)?;
                    if !exp.is_zero() {
                        factors.push((id_a.clone(), exp));
                    }
                    (i, j) = (i + 1, j + 1);
                }
            }
        }
        factors.extend(self.factors[i..].iter().cloned());
        factors.extend(rhs.factors[j..].iter().cloned());
        let dimension = self.dimension().try_mul(rhs.dimension())?;
        Ok(Self { factors, dimension })
    }

    /// Divides `self` by `rhs`.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::CrossRegistry`] if `self` and `rhs` were minted by
    /// different unit registries.
    /// Returns [`UnitError::NotExponentiable`] if `rhs` is anchored.
    /// Returns [`UnitError::NotComposable`] if `self` is anchored.
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`DimensionError::ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if combining a shared atom's exponents overflows.
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`DimensionError::CrossRegistry`](inchworm_dimensions::DimensionError::CrossRegistry)
    /// if `self` and `rhs`'s dimensions come from different dimension registries.
    pub fn try_div(&self, rhs: &Self) -> Result<Self, UnitError> {
        self.try_mul(&rhs.recip()?)
    }

    /// Raises `self` to the power of `e`, pruning any that cancels to zero.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::NotExponentiable`] if the unit contains any
    /// anchored atom and `exp != 1`.
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`DimensionError::ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if multiplying an atom's exponent by `e` overflows.
    pub fn pow(&self, e: Exp) -> Result<Self, UnitError> {
        let mut factors = SmallVec::new();
        for (atom_data, exp) in self.factors.iter() {
            atom_data.check_exponentiable(e)?;
            factors.push((atom_data.clone(), exp.checked_mul(e)?))
        }
        if e.is_zero() {
            return Ok(Self::empty());
        }
        let dimension = self.dimension.pow(e)?;
        Ok(Self { factors, dimension })
    }

    /// Computes the reciprocal of `self` by raising it to the power of `-1`.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::NotExponentiable`] if the unit contains any
    /// anchored atoms.
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`DimensionError::ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if computing the reciprocal of an atom's exponents overflows.
    pub fn recip(&self) -> Result<Self, UnitError> {
        let mut factors = SmallVec::new();
        for (atom_data, exp) in self.factors.iter() {
            atom_data.check_exponentiable(Exp::int(-1))?;
            factors.push((atom_data.clone(), exp.checked_neg()?));
        }
        Ok(Self {
            factors,
            dimension: self.dimension.recip()?,
        })
    }
}

// ---- Display helpers ----
impl DeltaUnit {
    fn sorted_factors(&self) -> Vec<&(UnitAtom, Exp)> {
        let mut factors: Vec<_> = self.factors.iter().collect();
        factors.sort_by(|a, b| a.0.symbol.cmp(&b.0.symbol));
        factors
    }
}

fn write_factor(f: &mut fmt::Formatter<'_>, symbol: &str, exp: Exp, pretty: bool) -> fmt::Result {
    f.write_str(symbol)?;
    let num = exp.num().unsigned_abs();
    match (exp.is_int(), num, pretty) {
        (true, 1, _) => Ok(()),
        (true, _, true) => num
            .to_string()
            .chars()
            .filter_map(digit_to_superscript)
            .try_for_each(|c| f.write_char(c)),
        (true, _, false) => write!(f, "{CARET_CHAR}{num}"),
        (false, ..) => write!(
            f,
            "{CARET_CHAR}{LPAREN_CHAR}{num}{SLASH_CHAR}{}{RPAREN_CHAR}",
            exp.den()
        ),
    }
}

impl fmt::Display for DeltaUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pretty = f.alternate();
        let sep = if pretty { PRETTY_MUL_CHAR } else { MUL_CHAR };
        let factors = self.sorted_factors();
        let mut first = true;
        for (atom, exp) in factors.iter().filter(|(_, e)| e.num() > 0) {
            if !first {
                f.write_char(sep)?;
            }
            write_factor(f, &atom.symbol, *exp, pretty)?;
            first = false;
        }
        if first {
            f.write_char(UNITARY_IDENT_CHAR)?;
        }
        for (atom, exp) in factors.iter().filter(|(_, e)| e.num() < 0) {
            f.write_char(SLASH_CHAR)?;
            write_factor(f, &atom.symbol, *exp, pretty)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atom::ConversionKind;
    use crate::test_utils::{errors_match, make_unit_atom};
    use inchworm_dimensions::{DimRegistry, DimensionError};

    /// `DeltaUnit` can be moved to and shared between threads (checked at compile time).
    #[test]
    fn unit_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DeltaUnit>();
    }

    #[test]
    fn empty() {
        let empty = DeltaUnit::empty();
        assert!(empty.factors.is_empty());
        assert!(empty.dimension.is_dimensionless());
    }

    mod single {
        use super::*;

        #[test]
        fn basic() {
            let registry_id = UnitRegistryId::next();
            let name = "meter";
            let mut dim_registry = DimRegistry::new("test-dim-reg");
            let length = dim_registry.add_base("length", None).unwrap();
            let conversion = ConversionKind::Linear { scale: 2.0 };
            let atom = make_unit_atom(registry_id, name, length.clone(), conversion);
            let unit = DeltaUnit::single(&atom, Exp::ONE).unwrap();
            assert_eq!(unit.factors.len(), 1);
            assert_eq!(unit.factors.first().unwrap(), &(atom, Exp::ONE));
            assert_eq!(unit.dimension, length);
        }

        #[test]
        fn zero_exponent_yields_empty() {
            let registry_id = UnitRegistryId::next();
            let name = "meter";
            let mut dim_registry = DimRegistry::new("test-dim-reg");
            let length = dim_registry.add_base("length", None).unwrap();
            let conversion = ConversionKind::Linear { scale: 2.0 };
            let atom = make_unit_atom(registry_id, name, length.clone(), conversion);
            let unit = DeltaUnit::single(&atom, Exp::ZERO).unwrap();
            assert!(unit.factors.is_empty());
            assert!(unit.dimension().is_dimensionless());
        }

        #[test]
        fn propagates_exponent_overflow() {
            let registry_id = UnitRegistryId::next();
            let name = "meter";
            let mut dim_registry = DimRegistry::new("test-dim-reg");
            let length = dim_registry.add_base("length", None).unwrap();
            let conversion = ConversionKind::Linear { scale: 2.0 };
            let atom = make_unit_atom(
                registry_id,
                name,
                length.pow(Exp::int(2)).unwrap(),
                conversion,
            );
            let err = DeltaUnit::single(&atom, Exp::int(i64::MAX)).unwrap_err();
            let expected_err = UnitError::Dimension(DimensionError::ExponentOverflow);
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_exponent_other_than_one_for_affine() {
            let registry_id = UnitRegistryId::next();
            let mut dim_registry = DimRegistry::new("test-dim-reg");
            let temperature = dim_registry.add_base("temperature", None).unwrap();
            let conversion = ConversionKind::Affine {
                scale: 1.0,
                offset: 273.15,
            };
            let atom = make_unit_atom(registry_id, "celsius", temperature, conversion);
            let err = DeltaUnit::single(&atom, Exp::int(2)).unwrap_err();
            let expected_err = UnitError::NotExponentiable {
                name: "celsius".into(),
                registry_id,
                exp: Exp::int(2),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_zero_exponent_for_affine() {
            let registry_id = UnitRegistryId::next();
            let mut dim_registry = DimRegistry::new("test-dim-reg");
            let temperature = dim_registry.add_base("temperature", None).unwrap();
            let conversion = ConversionKind::Affine {
                scale: 1.0,
                offset: 273.15,
            };
            let atom = make_unit_atom(registry_id, "celsius", temperature, conversion);
            let err = DeltaUnit::single(&atom, Exp::ZERO).unwrap_err();
            let expected_err = UnitError::NotExponentiable {
                name: "celsius".into(),
                registry_id,
                exp: Exp::ZERO,
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn allows_exponent_one_for_affine() {
            let registry_id = UnitRegistryId::next();
            let mut dim_registry = DimRegistry::new("test-dim-reg");
            let dim = dim_registry.add_base("dim", None).unwrap();
            let conversion = ConversionKind::Affine {
                scale: 5.0,
                offset: 2.0,
            };
            let atom = make_unit_atom(registry_id, "unit", dim.clone(), conversion);
            let unit = DeltaUnit::single(&atom, Exp::ONE).unwrap();
            assert_eq!(unit.factors.len(), 1);
            assert_eq!(unit.factors.first().unwrap(), &(atom, Exp::ONE));
            assert_eq!(unit.dimension, dim);
        }
    }

    mod try_mul {
        use super::*;

        #[test]
        fn merges_disjoint_atoms() {
            let registry_id = UnitRegistryId::next();
            let mut dim_registry = DimRegistry::new("test-dim-reg");
            let a_dim = dim_registry.add_base("a", None).unwrap();
            let b_dim = dim_registry.add_base("b", None).unwrap();
            let a_atom = make_unit_atom(
                registry_id,
                "a_unit",
                a_dim.clone(),
                ConversionKind::Linear { scale: 2.0 },
            );
            let b_atom = make_unit_atom(
                registry_id,
                "b_unit",
                b_dim.clone(),
                ConversionKind::Linear { scale: 2.5 },
            );
            let a_unit = DeltaUnit::single(&a_atom, Exp::ONE).unwrap();
            let b_unit = DeltaUnit::single(&b_atom, Exp::ONE).unwrap();
            let ab_unit = a_unit.try_mul(&b_unit).unwrap();
            let expected_factors: SmallVec<[(UnitAtom, Exp); MAX_INLINE_FACTORS]> =
                smallvec![(a_atom, Exp::ONE), (b_atom, Exp::ONE)];
            assert_eq!(ab_unit.factors, expected_factors);
            assert_eq!(ab_unit.dimension, a_dim.try_mul(&b_dim).unwrap());
        }

        #[test]
        fn combines_shared_atom_exponents() {
            let registry_id = UnitRegistryId::next();
            let mut dim_registry = DimRegistry::new("test-dim-reg");
            let a_dim = dim_registry.add_base("a", None).unwrap();
            let a_atom = make_unit_atom(
                registry_id,
                "a_unit",
                a_dim.clone(),
                ConversionKind::Linear { scale: 2.0 },
            );
            let a_unit_1 = DeltaUnit::single(&a_atom, Exp::ONE).unwrap();
            let a_unit_2 = DeltaUnit::single(&a_atom, Exp::ONE).unwrap();
            let ab_unit = a_unit_1.try_mul(&a_unit_2).unwrap();
            let expected_factors: SmallVec<[(UnitAtom, Exp); MAX_INLINE_FACTORS]> =
                smallvec![(a_atom, Exp::int(2))];
            assert_eq!(ab_unit.factors, expected_factors);
            assert_eq!(ab_unit.dimension, a_dim.pow(Exp::int(2)).unwrap());
        }

        #[test]
        fn cancels_shared_atom_to_zero() {
            let registry_id = UnitRegistryId::next();
            let mut dim_registry = DimRegistry::new("test-dim-reg");
            let a_dim = dim_registry.add_base("a", None).unwrap();
            let a_atom = make_unit_atom(
                registry_id,
                "a_unit",
                a_dim.clone(),
                ConversionKind::Linear { scale: 2.0 },
            );
            let a_unit_1 = DeltaUnit::single(&a_atom, Exp::ONE).unwrap();
            let a_unit_2 = DeltaUnit::single(&a_atom, Exp::int(-1)).unwrap();
            let ab_unit = a_unit_1.try_mul(&a_unit_2).unwrap();
            assert!(ab_unit.factors.is_empty());
            assert!(ab_unit.dimension.is_dimensionless());
        }

        #[test]
        fn propagates_exponent_overflow() {
            let registry_id = UnitRegistryId::next();
            let mut dim_registry = DimRegistry::new("test-dim-reg");
            let a_dim = dim_registry.add_base("a", None).unwrap();
            let a_atom = make_unit_atom(
                registry_id,
                "a_unit",
                a_dim.clone(),
                ConversionKind::Linear { scale: 2.0 },
            );
            let a_unit_1 = DeltaUnit::single(&a_atom, Exp::int(2)).unwrap();
            let a_unit_2 = DeltaUnit::single(&a_atom, Exp::int(i64::MAX)).unwrap();
            let err = a_unit_1.try_mul(&a_unit_2).unwrap_err();
            let expected_err = UnitError::Dimension(DimensionError::ExponentOverflow);
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_cross_registry_dimensions() {
            let mut a_dim_registry = DimRegistry::new("test-dim-reg-a");
            let mut b_dim_registry = DimRegistry::new("test-dim-reg-b");
            let a_dim = a_dim_registry.add_base("a", None).unwrap();
            let b_dim = b_dim_registry.add_base("b", None).unwrap();
            let registry_id = UnitRegistryId::next();
            let a_atom = make_unit_atom(
                registry_id,
                "a_unit",
                a_dim.clone(),
                ConversionKind::Linear { scale: 2.0 },
            );
            let b_atom = make_unit_atom(
                registry_id,
                "b_unit",
                b_dim.clone(),
                ConversionKind::Linear { scale: 1.0 },
            );
            let a_unit = DeltaUnit::single(&a_atom, Exp::ONE).unwrap();
            let b_unit = DeltaUnit::single(&b_atom, Exp::ONE).unwrap();
            let err = a_unit.try_mul(&b_unit).unwrap_err();
            let expected_err = UnitError::Dimension(DimensionError::CrossRegistry {
                left: a_dim_registry.id(),
                right: b_dim_registry.id(),
            });
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_cross_registry_dimensionless_atoms() {
            let a_atom = make_unit_atom(
                UnitRegistryId::next(),
                "a_unit",
                Dimension::dimensionless(),
                ConversionKind::Linear { scale: 1.0 },
            );
            let a_unit = DeltaUnit::single(&a_atom, Exp::ONE).unwrap();
            let b_atom = make_unit_atom(
                UnitRegistryId::next(),
                "b_unit",
                Dimension::dimensionless(),
                ConversionKind::Linear { scale: 1.0 },
            );
            let b_unit = DeltaUnit::single(&b_atom, Exp::ONE).unwrap();
            let err = a_unit.try_mul(&b_unit).unwrap_err();
            let expected_err = UnitError::CrossRegistry {
                left: a_atom.registry_id,
                right: b_atom.registry_id,
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_composing_point_like_left_operand() {
            let registry_id = UnitRegistryId::next();
            let affine_atom = make_unit_atom(
                registry_id,
                "affine_unit",
                Dimension::dimensionless(),
                ConversionKind::Affine {
                    scale: 1.0,
                    offset: 1.0,
                },
            );
            let affine_unit = DeltaUnit::single(&affine_atom, Exp::ONE).unwrap();
            let linear_atom = make_unit_atom(
                registry_id,
                "linear_unit",
                Dimension::dimensionless(),
                ConversionKind::Linear { scale: 1.0 },
            );
            let linear_unit = DeltaUnit::single(&linear_atom, Exp::ONE).unwrap();
            let err = affine_unit.try_mul(&linear_unit).unwrap_err();
            let expected_err = UnitError::NotComposable {
                name: affine_atom.name.to_string(),
                registry_id: affine_atom.registry_id,
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_composing_point_like_right_operand() {
            let registry_id = UnitRegistryId::next();
            let affine_atom = make_unit_atom(
                registry_id,
                "affine_unit",
                Dimension::dimensionless(),
                ConversionKind::Affine {
                    scale: 1.0,
                    offset: 1.0,
                },
            );
            let affine_unit = DeltaUnit::single(&affine_atom, Exp::ONE).unwrap();
            let linear_atom = make_unit_atom(
                registry_id,
                "linear_unit",
                Dimension::dimensionless(),
                ConversionKind::Linear { scale: 1.0 },
            );
            let linear_unit = DeltaUnit::single(&linear_atom, Exp::ONE).unwrap();
            let err = linear_unit.try_mul(&affine_unit).unwrap_err();
            let expected_err = UnitError::NotComposable {
                name: affine_atom.name.to_string(),
                registry_id: affine_atom.registry_id,
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_composing_point_like_with_itself() {
            let registry_id = UnitRegistryId::next();
            let affine_atom = make_unit_atom(
                registry_id,
                "affine_unit",
                Dimension::dimensionless(),
                ConversionKind::Affine {
                    scale: 1.0,
                    offset: 1.0,
                },
            );
            let affine_unit = DeltaUnit::single(&affine_atom, Exp::ONE).unwrap();
            let err = affine_unit.try_mul(&affine_unit).unwrap_err();
            let expected_err = UnitError::NotComposable {
                name: affine_atom.name.to_string(),
                registry_id: affine_atom.registry_id,
            };
            assert!(errors_match(&err, &expected_err));
        }
    }

    mod pow {
        use super::*;

        #[test]
        fn basic() {
            let mut dim_registry = DimRegistry::new("test-dim-reg");
            let a_dim = dim_registry.add_base("a", None).unwrap();
            let b_dim = dim_registry.add_base("b", None).unwrap();
            let registry_id = UnitRegistryId::next();
            let a_atom = make_unit_atom(
                registry_id,
                "a_unit",
                a_dim.clone(),
                ConversionKind::Linear { scale: 1.0 },
            );
            let b_atom = make_unit_atom(
                registry_id,
                "b_unit",
                b_dim.clone(),
                ConversionKind::Linear { scale: 2.0 },
            );
            let dimension = a_atom
                .dimension
                .try_mul(&b_atom.dimension.pow(Exp::int(3)).unwrap())
                .unwrap();
            let unit = DeltaUnit {
                factors: smallvec![(a_atom.clone(), Exp::ONE), (b_atom.clone(), Exp::int(3))],
                dimension: dimension.clone(),
            };
            let e = Exp::int(2);
            let unit_raised = unit.pow(e).unwrap();
            let expected_factors: SmallVec<[(UnitAtom, Exp); MAX_INLINE_FACTORS]> =
                smallvec![(a_atom, Exp::int(2)), (b_atom, Exp::int(6))];
            assert_eq!(unit_raised.factors, expected_factors);
            assert_eq!(unit_raised.dimension, dimension.pow(e).unwrap());
        }

        #[test]
        fn zero_exponent_yields_empty() {
            let registry_id = UnitRegistryId::next();
            let mut dim_registry = DimRegistry::new("test-dim-reg");
            let a_dim = dim_registry.add_base("a", None).unwrap();
            let a_atom = make_unit_atom(
                registry_id,
                "a_unit",
                a_dim.clone(),
                ConversionKind::Linear { scale: 2.0 },
            );
            let a_unit = DeltaUnit::single(&a_atom, Exp::ONE).unwrap();
            let raised_to_zero = a_unit.pow(Exp::ZERO).unwrap();
            assert!(raised_to_zero.factors.is_empty());
            assert!(raised_to_zero.dimension.is_dimensionless());
        }

        #[test]
        fn propagates_exponent_overflow() {
            let registry_id = UnitRegistryId::next();
            let mut dim_registry = DimRegistry::new("test-dim-reg");
            let a_dim = dim_registry.add_base("a", None).unwrap();
            let a_atom = make_unit_atom(
                registry_id,
                "a_unit",
                a_dim.clone(),
                ConversionKind::Linear { scale: 2.0 },
            );
            let a_unit = DeltaUnit::single(&a_atom, Exp::int(i64::MAX)).unwrap();
            let err = a_unit.pow(Exp::int(2)).unwrap_err();
            let expected_err = UnitError::Dimension(DimensionError::ExponentOverflow);
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_non_unit_exponent_on_point_like_atom() {
            let registry_id = UnitRegistryId::next();
            let affine_atom = make_unit_atom(
                registry_id,
                "affine_unit",
                Dimension::dimensionless(),
                ConversionKind::Affine {
                    scale: 1.0,
                    offset: 1.0,
                },
            );
            let affine_unit = DeltaUnit::single(&affine_atom, Exp::ONE).unwrap();
            let err = affine_unit.pow(Exp::int(2)).unwrap_err();
            let expected_err = UnitError::NotExponentiable {
                name: affine_atom.name.to_string(),
                registry_id: affine_atom.registry_id,
                exp: Exp::int(2),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_zero_exponent_on_point_like_atom() {
            let registry_id = UnitRegistryId::next();
            let affine_atom = make_unit_atom(
                registry_id,
                "affine_unit",
                Dimension::dimensionless(),
                ConversionKind::Affine {
                    scale: 1.0,
                    offset: 1.0,
                },
            );
            let affine_unit = DeltaUnit::single(&affine_atom, Exp::ONE).unwrap();
            let err = affine_unit.pow(Exp::ZERO).unwrap_err();
            let expected_err = UnitError::NotExponentiable {
                name: affine_atom.name.to_string(),
                registry_id: affine_atom.registry_id,
                exp: Exp::ZERO,
            };
            assert!(errors_match(&err, &expected_err));
        }
    }

    mod recip {
        use super::*;

        #[test]
        fn rejects_reciprocal_of_point_like_atom() {
            let registry_id = UnitRegistryId::next();
            let affine_atom = make_unit_atom(
                registry_id,
                "affine_unit",
                Dimension::dimensionless(),
                ConversionKind::Affine {
                    scale: 1.0,
                    offset: 1.0,
                },
            );
            let affine_unit = DeltaUnit::single(&affine_atom, Exp::ONE).unwrap();
            let err = affine_unit.recip().unwrap_err();
            let expected_err = UnitError::NotExponentiable {
                name: affine_atom.name.to_string(),
                registry_id: affine_atom.registry_id,
                exp: Exp::int(-1),
            };
            assert!(errors_match(&err, &expected_err));
        }
    }

    mod try_div {
        use super::*;

        #[test]
        fn rejects_dividing_by_point_like_unit() {
            let registry_id = UnitRegistryId::next();
            let affine_atom = make_unit_atom(
                registry_id,
                "affine_unit",
                Dimension::dimensionless(),
                ConversionKind::Affine {
                    scale: 1.0,
                    offset: 1.0,
                },
            );
            let affine_unit = DeltaUnit::single(&affine_atom, Exp::ONE).unwrap();
            let linear_atom = make_unit_atom(
                registry_id,
                "linear_unit",
                Dimension::dimensionless(),
                ConversionKind::Linear { scale: 1.0 },
            );
            let linear_unit = DeltaUnit::single(&linear_atom, Exp::ONE).unwrap();
            let err = linear_unit.try_div(&affine_unit).unwrap_err();
            let expected_err = UnitError::NotExponentiable {
                name: affine_atom.name.to_string(),
                registry_id: affine_atom.registry_id,
                exp: Exp::int(-1),
            };
            assert!(errors_match(&err, &expected_err));
        }

        #[test]
        fn rejects_point_like_dividend() {
            let registry_id = UnitRegistryId::next();
            let affine_atom = make_unit_atom(
                registry_id,
                "affine_unit",
                Dimension::dimensionless(),
                ConversionKind::Affine {
                    scale: 1.0,
                    offset: 1.0,
                },
            );
            let affine_unit = DeltaUnit::single(&affine_atom, Exp::ONE).unwrap();
            let linear_atom = make_unit_atom(
                registry_id,
                "linear_unit",
                Dimension::dimensionless(),
                ConversionKind::Linear { scale: 1.0 },
            );
            let linear_unit = DeltaUnit::single(&linear_atom, Exp::ONE).unwrap();
            let err = affine_unit.try_div(&linear_unit).unwrap_err();
            let expected_err = UnitError::NotComposable {
                name: affine_atom.name.to_string(),
                registry_id: affine_atom.registry_id,
            };
            assert!(errors_match(&err, &expected_err));
        }
    }

    mod eq {
        use super::*;
        use std::collections::HashSet;

        /// Equality is structural: `(m*s)/s` equals `m` regardless of how it was built.
        #[test]
        fn equal_when_built_by_different_paths() {
            let registry_id = UnitRegistryId::next();
            let mut dims = DimRegistry::new("test-reg");
            let a_dim = dims.add_base("a_dim", None).unwrap();
            let b_dim = dims.add_base("b_dim", None).unwrap();
            let a_atom = make_unit_atom(
                registry_id,
                "a",
                a_dim,
                ConversionKind::Linear { scale: 1.0 },
            );
            let a = DeltaUnit::single(&a_atom, Exp::ONE).unwrap();
            let b_atom = make_unit_atom(
                registry_id,
                "b",
                b_dim,
                ConversionKind::Linear { scale: 1.0 },
            );
            let b = DeltaUnit::single(&b_atom, Exp::ONE).unwrap();
            assert_eq!(&a.try_mul(&b).unwrap().try_div(&b).unwrap(), &a)
        }

        /// The order of the operands does not matter: `a*b == b*a`.
        #[test]
        fn multiplication_is_commutative() {
            let registry_id = UnitRegistryId::next();
            let mut dims = DimRegistry::new("test-reg");
            let a_dim = dims.add_base("a_dim", None).unwrap();
            let b_dim = dims.add_base("b_dim", None).unwrap();
            let a_atom = make_unit_atom(
                registry_id,
                "a",
                a_dim,
                ConversionKind::Linear { scale: 1.0 },
            );
            let a = DeltaUnit::single(&a_atom, Exp::ONE).unwrap();
            let b_atom = make_unit_atom(
                registry_id,
                "b",
                b_dim,
                ConversionKind::Linear { scale: 1.0 },
            );
            let b = DeltaUnit::single(&b_atom, Exp::ONE).unwrap();
            assert_eq!(&a.try_mul(&b).unwrap(), &b.try_mul(&a).unwrap())
        }

        /// A unit whose factors all cancel equals `DeltaUnit::empty()`.
        #[test]
        fn cancelled_unit_equals_empty() {
            let registry_id = UnitRegistryId::next();
            let mut dims = DimRegistry::new("test-reg");
            let a_dim = dims.add_base("a_dim", None).unwrap();
            let a_atom = make_unit_atom(
                registry_id,
                "a",
                a_dim,
                ConversionKind::Linear { scale: 1.0 },
            );
            let a = DeltaUnit::single(&a_atom, Exp::ONE).unwrap();
            assert_eq!(&a.try_div(&a).unwrap(), &DeltaUnit::empty())
        }

        /// The same atom at different exponents is not equal: `m != m^2`.
        #[test]
        fn differing_exponents_are_not_equal() {
            let registry_id = UnitRegistryId::next();
            let mut dims = DimRegistry::new("test-reg");
            let a_dim = dims.add_base("a_dim", None).unwrap();
            let a_atom = make_unit_atom(
                registry_id,
                "a",
                a_dim,
                ConversionKind::Linear { scale: 1.0 },
            );
            let a = DeltaUnit::single(&a_atom, Exp::ONE).unwrap();
            let a2 = DeltaUnit::single(&a_atom, Exp::int(2)).unwrap();
            assert_ne!(&a, &a2)
        }

        /// Atoms compare by identity, not name: `meter` from two registries differ.
        #[test]
        fn same_name_from_different_registries_is_not_equal() {
            let mut dims = DimRegistry::new("test-reg");
            const NAME: &str = "a";
            let dim = dims.add_base("a_dim", None).unwrap();
            let reg_id_1 = UnitRegistryId::next();
            let atom_1 = make_unit_atom(
                reg_id_1,
                NAME,
                dim.clone(),
                ConversionKind::Linear { scale: 1.0 },
            );
            let unit_1 = DeltaUnit::single(&atom_1, Exp::ONE).unwrap();
            let reg_id_2 = UnitRegistryId::next();
            let atom_2 = make_unit_atom(
                reg_id_2,
                NAME,
                dim.clone(),
                ConversionKind::Linear { scale: 1.0 },
            );
            let unit_2 = DeltaUnit::single(&atom_2, Exp::ONE).unwrap();
            assert_ne!(&unit_1, &unit_2)
        }

        /// Equal units hash equally, so an equal unit built differently finds the stored one in a `HashSet`.
        #[test]
        fn hash_agrees_with_eq() {
            let registry_id = UnitRegistryId::next();
            let mut dim_registry = DimRegistry::new("test-dim-reg");
            let a_dim = dim_registry.add_base("a", None).unwrap();
            let b_dim = dim_registry.add_base("b", None).unwrap();
            let a_atom = make_unit_atom(
                registry_id,
                "a_unit",
                a_dim,
                ConversionKind::Linear { scale: 1.0 },
            );
            let b_atom = make_unit_atom(
                registry_id,
                "b_unit",
                b_dim,
                ConversionKind::Linear { scale: 1.0 },
            );
            let a_unit = DeltaUnit::single(&a_atom, Exp::ONE).unwrap();
            let b_unit = DeltaUnit::single(&b_atom, Exp::ONE).unwrap();
            let mut set = HashSet::new();
            set.insert(a_unit.try_mul(&b_unit).unwrap());
            assert!(set.contains(&b_unit.try_mul(&a_unit).unwrap()));
            set.insert(a_unit.clone());
            assert!(set.contains(&a_unit.try_mul(&b_unit).unwrap().try_div(&b_unit).unwrap()));
        }
    }

    mod display {
        use super::*;
        use crate::test_utils::mks_registry;

        #[test]
        fn displays_single_unit_without_exponent() {
            let registry = mks_registry();
            let meter = registry.get("meter").unwrap();
            assert_eq!(meter.to_string(), "m");
        }

        #[test]
        fn displays_compound_unit_with_slash_chaining() {
            let registry = mks_registry();
            assert_eq!(
                registry
                    .parse("kilogram / meter / second^2")
                    .unwrap()
                    .to_string(),
                "kg/m/s^2"
            );
        }

        #[test]
        fn displays_reciprocal_only_unit_with_leading_one() {
            let registry = mks_registry();
            let meter = registry.get("meter").unwrap();
            assert_eq!(meter.pow(Exp::int(-1)).unwrap().to_string(), "1/m");
        }

        #[test]
        fn displays_dimensionless_empty_unit_as_one() {
            assert_eq!(DeltaUnit::empty().to_string(), "1");
        }

        #[test]
        fn displays_fractional_exponent() {
            let registry = mks_registry();
            let meter = registry.get("meter").unwrap();
            assert_eq!(
                meter.pow(Exp::new(-1, 2).unwrap()).unwrap().to_string(),
                "1/m^(1/2)"
            );
        }

        #[test]
        fn displays_pretty_form() {
            let registry = mks_registry();
            let meter = registry.get("meter").unwrap();
            let kilogram = registry.get("kilogram").unwrap();
            let kilogram_meter_squared =
                kilogram.try_mul(&meter.pow(Exp::int(2)).unwrap()).unwrap();
            assert_eq!(kilogram_meter_squared.to_string(), "kg*m^2");
            assert_eq!(format!("{kilogram_meter_squared:#}"), "kg·m²");
        }
    }

    mod roundtrips {
        use super::*;
        use crate::test_utils::mks_registry;

        /// A compound unit survives Display -> parse unchanged.
        #[test]
        fn roundtrips_compound_unit() {
            let registry = mks_registry();
            let pascal = registry.parse("kilogram / meter / second^2").unwrap();
            assert_eq!(&registry.parse(&pascal.to_string()).unwrap(), &pascal);
        }

        /// The empty unit displays as `1` and parses back to `DeltaUnit::empty()`.
        #[test]
        fn roundtrips_empty_unit() {
            let registry = mks_registry();
            let empty = DeltaUnit::empty();
            assert_eq!(empty.to_string(), "1");
            assert_eq!(registry.parse(&empty.to_string()).unwrap(), empty);
        }

        /// A reciprocal-only unit displays with a leading `1/` and parses back unchanged.
        #[test]
        fn roundtrips_reciprocal_unit() {
            let registry = mks_registry();
            let second = registry.get("second").unwrap();
            let unit = second.recip().unwrap();
            let repr = unit.to_string();
            assert_eq!(repr, "1/s");
            assert_eq!(registry.parse(&repr).unwrap(), unit)
        }

        /// A fractional exponent displays as `^(n/d)` and parses back unchanged.
        #[test]
        fn roundtrips_fractional_exponent() {
            let registry = mks_registry();
            let second = registry.get("second").unwrap();
            let unit = second.pow(Exp::new(2, 3).unwrap()).unwrap();
            let repr = unit.to_string();
            assert_eq!(repr, "s^(2/3)");
            assert_eq!(registry.parse(&repr).unwrap(), unit)
        }

        /// The alternate (`{:#}`) form with `·` and superscripts parses back unchanged.
        #[test]
        fn roundtrips_pretty_form() {
            let registry = mks_registry();
            let meter = registry.get("meter").unwrap();
            let kilogram = registry.get("kilogram").unwrap();
            let unit = kilogram.try_mul(&meter.pow(Exp::int(2)).unwrap()).unwrap();
            let pretty = format!("{unit:#}");
            // Guard the premise: without this, a Display that ignored `#` would
            // still round-trip and the test would pass without exercising `·` or `²`.
            assert_eq!(pretty, "kg·m²");
            assert_eq!(registry.parse(&pretty).unwrap(), unit);
        }
    }
}
