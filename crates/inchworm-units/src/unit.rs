use std::{cmp::Ordering, sync::Arc};

use inchworm_dimensions::{Dimension, Exp};
use smallvec::{SmallVec, smallvec};

use crate::{
    UnitError, UnitRegistryId,
    atom::{ConversionKind, UnitData},
};

const MAX_INLINE_FACTORS: usize = 4;

/// A unit expression: a reduced product of powers over named unit atoms.
///
/// A free value: once built, a `Unit` never needs its originating unit
/// registry again. Caches the product of its factors' dimensions and the
/// product of their scales, so dimension compatibility and coherent-unit
/// conversion are O(1) lookups rather than re-derived on every use.
///
/// Invariants:
/// - sorted by [`UnitId`](crate::UnitId) ascending
/// - no zero exponents
/// - no duplicates.
#[derive(Debug, Clone)]
pub struct Unit {
    factors: SmallVec<[(Arc<UnitData>, Exp); MAX_INLINE_FACTORS]>,
    /// Cached product of factor dimensions.
    dimension: Dimension,
    /// Cached product of factor scale^exponent
    scale: f64,
}

impl Unit {
    /// Returns an empty (dimensionless, scale `1.0`) unit.
    pub fn empty() -> Self {
        Self {
            factors: SmallVec::new(),
            dimension: Dimension::dimensionless(),
            scale: 1.0,
        }
    }

    /// Returns a unit with a single factor at power `exp`, or an empty
    /// (dimensionless, scale `1.0`) unit if `exp` is zero.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`DimensionError::ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if raising `atom`'s dimension to `exp` overflows.
    pub(crate) fn single(atom: &Arc<UnitData>, exp: Exp) -> Result<Self, UnitError> {
        if !exp.is_zero() {
            let factors = smallvec![(atom.clone(), exp)];
            let dimension = atom.dimension.pow(exp)?;
            let scale = match atom.conversion {
                ConversionKind::Linear { scale } => scale.powf(exp.to_f64()),
                _ => todo!(),
            };
            Ok(Self {
                factors,
                dimension,
                scale,
            })
        } else {
            Ok(Self::empty())
        }
    }

    /// This unit's dimension: the cached product of its factors' dimensions.
    pub fn dimension(&self) -> &Dimension {
        &self.dimension
    }

    /// The identity of the unit registry that minted this value's atoms, or
    /// `None` if it has no factors (a bare dimensionless `Unit`).
    pub(crate) fn registry_id(&self) -> Option<UnitRegistryId> {
        self.factors.first().map(|(atom, _)| atom.registry_id)
    }
}

// ---- Algebra ----
impl Unit {
    /// Merges two units, combining exponents of shared atoms, pruning any that cancel to zero.
    ///
    /// # Errors
    /// Returns [`UnitError::CrossRegistry`] if `self` and `rhs` were minted by
    /// different unit registries.
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
        let scale = self.scale * rhs.scale;
        Ok(Self {
            factors,
            dimension,
            scale,
        })
    }

    /// Raises `self` to the power of `e`, pruning any that cancels to zero.
    ///
    /// # Errors
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`DimensionError::ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if multiplying an atom's exponent by `e` overflows.
    pub fn pow(&self, e: Exp) -> Result<Self, UnitError> {
        let mut factors = SmallVec::new();
        if !e.is_zero() {
            for (atom_data, exp) in self.factors.iter() {
                factors.push((atom_data.clone(), exp.checked_mul(e)?))
            }
        }
        let dimension = self.dimension.pow(e)?;
        let scale = self.scale.powf(e.to_f64());
        Ok(Self {
            factors,
            dimension,
            scale,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{errors_match, make_unit_atom};
    use inchworm_dimensions::{DimRegistry, DimensionError};

    #[test]
    fn empty() {
        let empty = Unit::empty();
        assert!(empty.factors.is_empty());
        assert!(empty.dimension.is_dimensionless());
        assert_eq!(empty.scale, 1.0);
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
            let unit = Unit::single(&atom, Exp::ONE).unwrap();
            assert_eq!(unit.factors.len(), 1);
            assert_eq!(unit.factors.first().unwrap(), &(atom, Exp::ONE));
            assert_eq!(unit.dimension, length);
            assert_eq!(unit.scale, 2.0);
        }

        #[test]
        fn zero_exponent_yields_empty() {
            let registry_id = UnitRegistryId::next();
            let name = "meter";
            let mut dim_registry = DimRegistry::new("test-dim-reg");
            let length = dim_registry.add_base("length", None).unwrap();
            let conversion = ConversionKind::Linear { scale: 2.0 };
            let atom = make_unit_atom(registry_id, name, length.clone(), conversion);
            let unit = Unit::single(&atom, Exp::ZERO).unwrap();
            assert!(unit.factors.is_empty());
            assert!(unit.dimension().is_dimensionless());
            assert_eq!(unit.scale, 1.0);
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
            let err = Unit::single(&atom, Exp::int(i64::MAX)).unwrap_err();
            let expected_err = UnitError::Dimension(DimensionError::ExponentOverflow);
            assert!(errors_match(&err, &expected_err));
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
            let a_unit = Unit::single(&a_atom, Exp::ONE).unwrap();
            let b_unit = Unit::single(&b_atom, Exp::ONE).unwrap();
            let ab_unit = a_unit.try_mul(&b_unit).unwrap();
            let expected_factors: SmallVec<[(Arc<UnitData>, Exp); MAX_INLINE_FACTORS]> =
                smallvec![(a_atom, Exp::ONE), (b_atom, Exp::ONE)];
            assert_eq!(ab_unit.factors, expected_factors);
            assert_eq!(ab_unit.dimension, a_dim.try_mul(&b_dim).unwrap());
            assert_eq!(ab_unit.scale, 5.0);
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
            let a_unit_1 = Unit::single(&a_atom, Exp::ONE).unwrap();
            let a_unit_2 = Unit::single(&a_atom, Exp::ONE).unwrap();
            let ab_unit = a_unit_1.try_mul(&a_unit_2).unwrap();
            let expected_factors: SmallVec<[(Arc<UnitData>, Exp); MAX_INLINE_FACTORS]> =
                smallvec![(a_atom, Exp::int(2))];
            assert_eq!(ab_unit.factors, expected_factors);
            assert_eq!(ab_unit.dimension, a_dim.pow(Exp::int(2)).unwrap());
            assert_eq!(ab_unit.scale, 4.0);
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
            let a_unit_1 = Unit::single(&a_atom, Exp::ONE).unwrap();
            let a_unit_2 = Unit::single(&a_atom, Exp::int(-1)).unwrap();
            let ab_unit = a_unit_1.try_mul(&a_unit_2).unwrap();
            assert!(ab_unit.factors.is_empty());
            assert!(ab_unit.dimension.is_dimensionless());
            assert_eq!(ab_unit.scale, 1.0);
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
            let a_unit_1 = Unit::single(&a_atom, Exp::int(2)).unwrap();
            let a_unit_2 = Unit::single(&a_atom, Exp::int(i64::MAX)).unwrap();
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
            let a_unit = Unit::single(&a_atom, Exp::ONE).unwrap();
            let b_unit = Unit::single(&b_atom, Exp::ONE).unwrap();
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
            let a_unit = Unit::single(&a_atom, Exp::ONE).unwrap();
            let b_atom = make_unit_atom(
                UnitRegistryId::next(),
                "b_unit",
                Dimension::dimensionless(),
                ConversionKind::Linear { scale: 1.0 },
            );
            let b_unit = Unit::single(&b_atom, Exp::ONE).unwrap();
            let err = a_unit.try_mul(&b_unit).unwrap_err();
            let expected_err = UnitError::CrossRegistry {
                left: a_atom.registry_id,
                right: b_atom.registry_id,
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
            let unit = Unit {
                factors: smallvec![(a_atom.clone(), Exp::ONE), (b_atom.clone(), Exp::int(3))],
                dimension: dimension.clone(),
                scale: 8.0,
            };
            let e = Exp::int(2);
            let unit_raised = unit.pow(e).unwrap();
            let expected_factors: SmallVec<[(Arc<UnitData>, Exp); MAX_INLINE_FACTORS]> =
                smallvec![(a_atom, Exp::int(2)), (b_atom, Exp::int(6))];
            assert_eq!(unit_raised.factors, expected_factors);
            assert_eq!(unit_raised.dimension, dimension.pow(e).unwrap());
            assert_eq!(unit_raised.scale, 64.0);
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
            let a_unit = Unit::single(&a_atom, Exp::ONE).unwrap();
            let raised_to_zero = a_unit.pow(Exp::ZERO).unwrap();
            assert!(raised_to_zero.factors.is_empty());
            assert!(raised_to_zero.dimension.is_dimensionless());
            assert_eq!(raised_to_zero.scale, 1.0);
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
            let a_unit = Unit::single(&a_atom, Exp::int(i64::MAX)).unwrap();
            let err = a_unit.pow(Exp::int(2)).unwrap_err();
            let expected_err = UnitError::Dimension(DimensionError::ExponentOverflow);
            assert!(errors_match(&err, &expected_err));
        }
    }
}
