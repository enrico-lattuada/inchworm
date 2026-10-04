//! The [`DeltaUnit`] value type: a composable (linear) unit.
//!
//! A delta unit is a reduced product of unit atoms (`m`, `kg*m^2*s^-2`) plus
//! its cached [`Dimension`]. A value in it is an amount or a difference, so it
//! can be multiplied, divided and raised to powers.

use inchworm_dimensions::{Dimension, Exp};
use std::fmt;

use crate::{UnitError, UnitForm, UnitRegistryId, atom::UnitAtom};

/// A composable unit: a reduced product of unit-atom powers with its cached
/// [`Dimension`].
///
/// Values are self-contained ([`Arc`](std::sync::Arc)-backed) and never need
/// the [`UnitRegistry`](crate::UnitRegistry) that produced them. `==` is
/// structural: two units are equal iff they have the same atoms at the same
/// powers, so `J` and `kg*m^2*s^-2` are different units even though their
/// dimensions are equal.
///
/// # Examples
///
/// ```
/// use inchworm_dimensions::DimRegistry;
/// use inchworm_units::{DeltaUnit, UnitRegistry};
///
/// let mut dims = DimRegistry::new("mechanics");
/// let length = dims.add_base("length", Some("L")).unwrap();
/// let time = dims.add_base("time", Some("T")).unwrap();
/// let mut units = UnitRegistry::new("si", dims);
/// let m = units.add_base("meter", "m", &length, true).unwrap();
/// let s = units.add_base("second", "s", &time, true).unwrap();
///
/// let speed = m.try_div(&s).unwrap();
/// assert_eq!(speed.to_string(), "m/s");
/// assert_eq!(speed.dimension(), &length.try_div(&time).unwrap());
/// assert_eq!(m.try_div(&m).unwrap(), DeltaUnit::dimensionless());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeltaUnit {
    factors: UnitForm,
    dimension: Dimension,
}

// ---- instantiation ----
impl DeltaUnit {
    /// Returns the dimensionless unit `1`: no factors and a dimensionless dimension.
    /// It belongs to no registry, so it combines with units of any registry.
    pub fn dimensionless() -> Self {
        Self {
            factors: UnitForm::empty(),
            dimension: Dimension::dimensionless(),
        }
    }

    /// Returns the unit made of `atom` alone, at power 1.
    /// The dimension is read from the atom, so this is the same for every atom kind.
    pub(crate) fn from_atom(atom: &UnitAtom) -> Self {
        Self {
            factors: UnitForm::single(atom, Exp::ONE),
            dimension: atom.dimension.clone(),
        }
    }
}

// ---- accessors ----
impl DeltaUnit {
    /// The stored factors (what `Display` prints).
    pub fn factors(&self) -> &UnitForm {
        &self.factors
    }

    /// This unit's dimension: the product of its factors' dimensions.
    pub fn dimension(&self) -> &Dimension {
        &self.dimension
    }

    /// Returns the [`UnitRegistryId`] of the registry whose atoms make up `self`,
    /// or `None` for the dimensionless unit.
    pub fn registry_id(&self) -> Option<UnitRegistryId> {
        self.factors
            .entries()
            .first()
            .map(|(unit_atom, _)| unit_atom.registry_id)
    }
}

// ---- algebra ----
impl DeltaUnit {
    /// Multiplies `self` by `rhs`, adding the exponents of shared atoms
    /// and dropping any that cancel to zero.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if combining exponents overflows.
    /// Returns [`UnitError::CrossRegistry`] if `self` and `rhs` were built by different unit registries.
    pub fn try_mul(&self, rhs: &Self) -> Result<Self, UnitError> {
        self.validate_cross_unit_registry_id(rhs)?;
        Ok(Self {
            factors: self.factors.mul(&rhs.factors)?,
            dimension: self.dimension().try_mul(rhs.dimension())?,
        })
    }

    /// Returns the reciprocal of `self`, with every exponent negated (`s` → `s^-1`).
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if computing the reciprocal of an atom's exponents overflows.
    pub fn recip(&self) -> Result<Self, UnitError> {
        Ok(Self {
            factors: self.factors.recip()?,
            dimension: self.dimension().recip()?,
        })
    }

    /// Divides `self` by `rhs`.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if combining exponents overflows.
    /// Returns [`UnitError::CrossRegistry`] if `self` and `rhs` were built by different unit registries.
    pub fn try_div(&self, rhs: &Self) -> Result<Self, UnitError> {
        self.try_mul(&rhs.recip()?)
    }

    /// Raises `self` to the power of `e`;
    /// `e = 0` gives the dimensionless unit.
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if multiplying an atom's exponent by `e` overflows.
    pub fn pow(&self, e: Exp) -> Result<Self, UnitError> {
        Ok(Self {
            factors: self.factors.pow(e)?,
            dimension: self.dimension().pow(e)?,
        })
    }

    fn validate_cross_unit_registry_id(&self, rhs: &Self) -> Result<(), UnitError> {
        if let (Some(lhs_id), Some(rhs_id)) = (self.registry_id(), rhs.registry_id())
            && lhs_id != rhs_id
        {
            return Err(UnitError::CrossRegistry {
                left: lhs_id,
                right: rhs_id,
            });
        }
        Ok(())
    }
}

impl fmt::Display for DeltaUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.factors.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        UnitRegistryId,
        test_utils::{errors_match, make_atom},
    };
    use inchworm_dimensions::DimRegistry;

    #[test]
    fn delta_unit_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DeltaUnit>();
    }

    mod dimensionless {
        use super::*;

        #[test]
        fn has_no_factors_and_dimensionless_dimension() {
            let unit = DeltaUnit::dimensionless();
            assert!(unit.factors.is_empty());
            assert!(unit.dimension.is_dimensionless());
        }

        #[test]
        fn has_no_registry_id() {
            assert!(DeltaUnit::dimensionless().registry_id().is_none());
        }
    }

    mod from_atom {
        use super::*;

        #[test]
        fn has_single_factor_and_atom_dimension() {
            let mut dim_reg = DimRegistry::new("test-reg");
            let dim = dim_reg.add_base("X", None).unwrap();
            let atom = make_atom(UnitRegistryId::next(), "x", dim.clone());
            let unit = DeltaUnit::from_atom(&atom);
            assert_eq!(unit.factors().entries(), [(atom, Exp::ONE)]);
            assert_eq!(*unit.dimension(), dim);
        }

        #[test]
        fn registry_id_is_the_atom_registry() {
            let atom = make_atom(UnitRegistryId::next(), "x", Dimension::dimensionless());
            let unit = DeltaUnit::from_atom(&atom);
            assert_eq!(unit.registry_id(), Some(atom.registry_id));
        }
    }

    struct MAndS {
        m: DeltaUnit,
        m_atom: UnitAtom,
        length: Dimension,
        s: DeltaUnit,
        s_atom: UnitAtom,
        time: Dimension,
    }

    fn m_and_s() -> MAndS {
        let mut dim_reg = DimRegistry::new("test-reg");
        let length = dim_reg.add_base("length", None).unwrap();
        let time = dim_reg.add_base("time", None).unwrap();
        let ureg_id = UnitRegistryId::next();
        let m_atom = make_atom(ureg_id, "m", length.clone());
        let s_atom = make_atom(ureg_id, "s", time.clone());
        let m = DeltaUnit::from_atom(&m_atom);
        let s = DeltaUnit::from_atom(&s_atom);
        MAndS {
            m,
            m_atom,
            length,
            s,
            s_atom,
            time,
        }
    }

    mod try_mul {
        use super::*;

        #[test]
        fn combines_factors_and_dimension() {
            let MAndS {
                m, m_atom, length, ..
            } = m_and_s();
            let m2 = m.try_mul(&m).unwrap();
            assert_eq!(m2.factors().entries(), [(m_atom, Exp::int(2))]);
            assert_eq!(*m2.dimension(), length.pow(Exp::int(2)).unwrap());
        }

        #[test]
        fn dimensionless_combines_with_any_registry() {
            let m = m_and_s().m;
            assert_eq!(m.try_mul(&DeltaUnit::dimensionless()).unwrap(), m);
            assert_eq!(DeltaUnit::dimensionless().try_mul(&m).unwrap(), m);
        }

        #[test]
        fn err_on_cross_registry() {
            let mut dim_reg_a = DimRegistry::new("test-reg-a");
            let length = dim_reg_a.add_base("length", None).unwrap();
            let mut dim_reg_b = DimRegistry::new("test-reg-b");
            let time = dim_reg_b.add_base("time", None).unwrap();
            let m_reg_id = UnitRegistryId::next();
            let m_atom = make_atom(m_reg_id, "m", length.clone());
            let s_reg_id = UnitRegistryId::next();
            let s_atom = make_atom(s_reg_id, "s", time.clone());
            let m = DeltaUnit::from_atom(&m_atom);
            let s = DeltaUnit::from_atom(&s_atom);
            let actual = m.try_mul(&s).unwrap_err();
            let expected = UnitError::CrossRegistry {
                left: m_reg_id,
                right: s_reg_id,
            };
            assert!(errors_match(&actual, &expected));
        }
    }

    mod try_div {
        use super::*;

        #[test]
        fn self_division_gives_dimensionless() {
            let m = m_and_s().m;
            assert_eq!(m.try_div(&m).unwrap(), DeltaUnit::dimensionless());
        }

        #[test]
        fn divides_factors_and_dimension() {
            let MAndS {
                m,
                m_atom,
                length,
                s,
                s_atom,
                time,
            } = m_and_s();
            let m_over_s = m.try_div(&s).unwrap();
            assert_eq!(
                m_over_s.factors().entries(),
                [(m_atom, Exp::int(1)), (s_atom, Exp::int(-1))]
            );
            assert_eq!(*m_over_s.dimension(), length.try_div(&time).unwrap());
        }
    }

    mod pow {
        use super::*;

        #[test]
        fn zero_gives_dimensionless() {
            let m = m_and_s().m;
            assert_eq!(m.pow(Exp::ZERO).unwrap(), DeltaUnit::dimensionless());
        }

        #[test]
        fn raises_factors_and_dimension() {
            let MAndS {
                m,
                m_atom,
                length,
                s,
                s_atom,
                time,
            } = m_and_s();
            let m_over_s = m.try_div(&s).unwrap();
            let squared_m_over_s = m_over_s.pow(Exp::int(2)).unwrap();
            assert_eq!(
                squared_m_over_s.factors().entries(),
                [(m_atom, Exp::int(2)), (s_atom, Exp::int(-2))]
            );
            assert_eq!(
                *squared_m_over_s.dimension(),
                length.try_div(&time).unwrap().pow(Exp::int(2)).unwrap()
            );
        }
    }

    mod recip {
        use super::*;

        #[test]
        fn negates_factors_and_inverts_dimension() {
            let MAndS {
                m, m_atom, length, ..
            } = m_and_s();
            let one_over_m = m.recip().unwrap();
            assert_eq!(one_over_m.factors().entries(), [(m_atom, Exp::int(-1))]);
            assert_eq!(*one_over_m.dimension(), length.recip().unwrap());
        }
    }

    mod display {
        use super::*;

        #[test]
        fn delegates_to_factors() {
            let MAndS { m, s, .. } = m_and_s();
            let m_over_s = m.try_div(&s).unwrap();
            let squared_m_over_s = m_over_s.pow(Exp::int(2)).unwrap();
            let factors = squared_m_over_s.factors();
            assert_eq!(format!("{squared_m_over_s}"), format!("{factors}"));
            assert_eq!(format!("{squared_m_over_s:#}"), format!("{factors:#}"));
        }
    }
}
