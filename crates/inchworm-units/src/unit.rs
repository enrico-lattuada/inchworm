use std::cmp::Ordering;

use inchworm_dimensions::{Dimension, Exp};
use smallvec::{SmallVec, smallvec};

use crate::{
    UnitError, UnitRegistryId,
    atom::{ConversionKind, UnitAtom},
    parse::digit_to_superscript,
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
    factors: SmallVec<[(UnitAtom, Exp); MAX_INLINE_FACTORS]>,
    /// Cached product of factor dimensions.
    dimension: Dimension,
    /// Cached product of factor scale^exponent
    scale: f64,
}

impl Unit {
    /// Returns a scaled, dimensionless unit.
    pub fn scaled(scale: f64) -> Self {
        Self {
            factors: SmallVec::new(),
            dimension: Dimension::dimensionless(),
            scale,
        }
    }

    /// Returns an empty (dimensionless, scale `1.0`) unit.
    pub fn empty() -> Self {
        Self::scaled(1.0)
    }

    /// Returns a unit with a single factor at power `exp`, or an empty
    /// (dimensionless, scale `1.0`) unit if `exp` is zero.
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
        let scale = match atom.conversion {
            ConversionKind::Linear { scale } => scale.powf(exp.to_f64()),
            ConversionKind::Affine { scale, .. } => {
                debug_assert!(exp.is_one(), "should never get exp != 1 for affine unit");
                scale
            }
        };
        Ok(Self {
            factors,
            dimension,
            scale,
        })
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

    /// The factors of this unit.
    pub(crate) fn factors(&self) -> &[(UnitAtom, Exp)] {
        &self.factors
    }

    /// The scale of this unit.
    pub(crate) fn scale(&self) -> f64 {
        self.scale
    }
}

// ---- Algebra ----
impl Unit {
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
        let scale = self.scale * rhs.scale;
        Ok(Self {
            factors,
            dimension,
            scale,
        })
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
        let scale = self.scale.powf(e.to_f64());
        Ok(Self {
            factors,
            dimension,
            scale,
        })
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
            scale: 1.0 / self.scale,
        })
    }
}

// ---- Display helpers ----
impl Unit {
    fn grouped_factors(&self) -> (Vec<&(UnitAtom, Exp)>, Vec<&(UnitAtom, Exp)>) {
        let (mut positive, mut negative) = (Vec::new(), Vec::new());
        for factor in self.factors().iter() {
            if factor.1.num() > 0 {
                positive.push(factor);
            } else {
                negative.push(factor);
            }
        }
        // Sort by symbol
        positive.sort_by(|&a, &b| a.0.symbol.cmp(&b.0.symbol));
        negative.sort_by(|&a, &b| a.0.symbol.cmp(&b.0.symbol));
        (positive, negative)
    }
}

fn write_factor(
    f: &mut std::fmt::Formatter<'_>,
    symbol: &str,
    exp: Exp,
    pretty: bool,
) -> std::fmt::Result {
    write!(f, "{symbol}")?;
    let num = exp.num().unsigned_abs();
    if exp.is_int() {
        if num == 1 {
            return Ok(());
        } else if pretty {
            let as_superscript: String = num
                .to_string()
                .chars()
                .map(|c| digit_to_superscript(c).expect("should be a superscript here"))
                .collect();
            write!(f, "{as_superscript}")?;
        } else {
            write!(f, "^{num}")?;
        }
    } else {
        write!(f, "^({}/{})", num, exp.den())?;
    }
    Ok(())
}

impl std::fmt::Display for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (positive, negative) = self.grouped_factors();
        let pretty = f.alternate();
        let mul_separator = if pretty { '·' } else { '*' };
        let mut wrote_anything = false;
        if self.scale() != 1.0 {
            write!(f, "{}", self.scale())?;
            wrote_anything = true;
        }
        for (atom, exp) in positive {
            if wrote_anything {
                write!(f, "{mul_separator}")?;
            }
            // write factor
            write_factor(f, &atom.symbol, *exp, pretty)?;
            // flip flag
            wrote_anything = true;
        }
        if !wrote_anything {
            write!(f, "1")?;
        }
        for (atom, exp) in negative {
            write!(f, "/")?;
            write_factor(f, &atom.symbol, *exp, pretty)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{errors_match, make_unit_atom, units_match};
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
            let err = Unit::single(&atom, Exp::int(2)).unwrap_err();
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
            let err = Unit::single(&atom, Exp::ZERO).unwrap_err();
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
            let unit = Unit::single(&atom, Exp::ONE).unwrap();
            assert_eq!(unit.factors.len(), 1);
            assert_eq!(unit.factors.first().unwrap(), &(atom, Exp::ONE));
            assert_eq!(unit.dimension, dim);
            assert_eq!(unit.scale, 5.0);
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
            let expected_factors: SmallVec<[(UnitAtom, Exp); MAX_INLINE_FACTORS]> =
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
            let expected_factors: SmallVec<[(UnitAtom, Exp); MAX_INLINE_FACTORS]> =
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
            let affine_unit = Unit::single(&affine_atom, Exp::ONE).unwrap();
            let linear_atom = make_unit_atom(
                registry_id,
                "linear_unit",
                Dimension::dimensionless(),
                ConversionKind::Linear { scale: 1.0 },
            );
            let linear_unit = Unit::single(&linear_atom, Exp::ONE).unwrap();
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
            let affine_unit = Unit::single(&affine_atom, Exp::ONE).unwrap();
            let linear_atom = make_unit_atom(
                registry_id,
                "linear_unit",
                Dimension::dimensionless(),
                ConversionKind::Linear { scale: 1.0 },
            );
            let linear_unit = Unit::single(&linear_atom, Exp::ONE).unwrap();
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
            let affine_unit = Unit::single(&affine_atom, Exp::ONE).unwrap();
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
            let unit = Unit {
                factors: smallvec![(a_atom.clone(), Exp::ONE), (b_atom.clone(), Exp::int(3))],
                dimension: dimension.clone(),
                scale: 8.0,
            };
            let e = Exp::int(2);
            let unit_raised = unit.pow(e).unwrap();
            let expected_factors: SmallVec<[(UnitAtom, Exp); MAX_INLINE_FACTORS]> =
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
            let affine_unit = Unit::single(&affine_atom, Exp::ONE).unwrap();
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
            let affine_unit = Unit::single(&affine_atom, Exp::ONE).unwrap();
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
            let affine_unit = Unit::single(&affine_atom, Exp::ONE).unwrap();
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
            let affine_unit = Unit::single(&affine_atom, Exp::ONE).unwrap();
            let linear_atom = make_unit_atom(
                registry_id,
                "linear_unit",
                Dimension::dimensionless(),
                ConversionKind::Linear { scale: 1.0 },
            );
            let linear_unit = Unit::single(&linear_atom, Exp::ONE).unwrap();
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
            let affine_unit = Unit::single(&affine_atom, Exp::ONE).unwrap();
            let linear_atom = make_unit_atom(
                registry_id,
                "linear_unit",
                Dimension::dimensionless(),
                ConversionKind::Linear { scale: 1.0 },
            );
            let linear_unit = Unit::single(&linear_atom, Exp::ONE).unwrap();
            let err = affine_unit.try_div(&linear_unit).unwrap_err();
            let expected_err = UnitError::NotComposable {
                name: affine_atom.name.to_string(),
                registry_id: affine_atom.registry_id,
            };
            assert!(errors_match(&err, &expected_err));
        }
    }

    mod display {
        use super::*;
        use crate::UnitRegistry;

        #[test]
        fn displays_single_unit_without_exponent() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let meter = registry
                .add_unit("meter", "m", length.clone(), 1.0, true)
                .unwrap();
            assert_eq!(meter.to_string(), "m");
        }

        #[test]
        fn displays_compound_unit_with_slash_chaining() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let time = dims.add_base("time", None).unwrap();
            let mass = dims.add_base("mass", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry
                .add_unit("meter", "m", length.clone(), 1.0, true)
                .unwrap();
            registry
                .add_unit("second", "s", time.clone(), 1.0, true)
                .unwrap();
            registry
                .add_unit("kilogram", "kg", mass.clone(), 1.0, false)
                .unwrap();
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
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let meter = registry
                .add_unit("meter", "m", length.clone(), 1.0, true)
                .unwrap();
            assert_eq!(meter.pow(Exp::int(-1)).unwrap().to_string(), "1/m");
        }

        #[test]
        fn displays_dimensionless_empty_unit_as_one() {
            assert_eq!(Unit::empty().to_string(), "1");
        }

        #[test]
        fn displays_scaled_unit() {
            assert_eq!(Unit::scaled(42.0).to_string(), "42");
            assert_eq!(Unit::scaled(1.23).to_string(), "1.23");
        }

        #[test]
        fn displays_fractional_exponent() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let meter = registry
                .add_unit("meter", "m", length.clone(), 1.0, true)
                .unwrap();
            assert_eq!(
                meter.pow(Exp::new(-1, 2).unwrap()).unwrap().to_string(),
                "1/m^(1/2)"
            );
        }

        #[test]
        fn displays_pretty_form() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let mass = dims.add_base("mass", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            let meter = registry
                .add_unit("meter", "m", length.clone(), 1.0, true)
                .unwrap();
            let kilogram = registry
                .add_unit("kilogram", "kg", mass.clone(), 1.0, false)
                .unwrap();
            let kilogram_meter_squared =
                kilogram.try_mul(&meter.pow(Exp::int(2)).unwrap()).unwrap();
            assert_eq!(kilogram_meter_squared.to_string(), "kg*m^2");
            assert_eq!(format!("{kilogram_meter_squared:#}"), "kg·m²");
        }
    }

    mod roundtrips {
        use super::*;
        use crate::{UnitRegistry, test_utils::units_match};

        #[test]
        fn formats_and_reparses_to_equivalent_unit() {
            let mut dims = DimRegistry::new("test-reg");
            let length = dims.add_base("length", None).unwrap();
            let time = dims.add_base("time", None).unwrap();
            let mass = dims.add_base("mass", None).unwrap();
            let mut registry = UnitRegistry::new("test-ureg", dims);
            registry
                .add_unit("meter", "m", length.clone(), 1.0, true)
                .unwrap();
            registry
                .add_unit("second", "s", time.clone(), 1.0, true)
                .unwrap();
            registry
                .add_unit("kilogram", "kg", mass.clone(), 1.0, false)
                .unwrap();
            let pascal = registry.parse("kilogram / meter / second^2").unwrap();
            assert!(units_match(
                &registry.parse(&pascal.to_string()).unwrap(),
                &pascal
            ));
        }
    }
}
