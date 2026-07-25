use crate::{DimensionError, Exp, Form, Signature};

/// How two [`Dimension`]s relate.
pub enum Compatibility {
    /// Equal canonical forms: the same physical kind by definition.
    /// e.g. `length/time` vs `velocity`; `force·length` vs `energy`.
    Full,
    /// Equal base signatures but different canonical forms.
    /// e.g. `plane_angle×length` vs `length`; `torque` vs `energy`;
    /// `angular_velocity` vs `frequency`; `strain` vs bare dimensionless.
    Partial,
    /// Different base signatures.
    Incompatible,
}

// TODO: Do not derive Hash, impl Hash for Dimension to be compatible with PartialEq

/// An immutable dimension value.
#[derive(Clone, Debug)]
pub struct Dimension {
    factors: Form,
    signature: Signature,
    canonical: Form,
}

impl Dimension {
    /// Returns a dimensionless dimension.
    ///
    /// The universal bare-dimensionless value; belongs to no registry and is
    /// usable in arithmetic with dimensions of any registry.
    pub fn dimensionless() -> Self {
        Self {
            factors: Form::empty(),
            signature: Signature(Form::empty()),
            canonical: Form::empty(),
        }
    }

    /// The stored named-composite factors (what `Display` prints).
    pub fn factors(&self) -> &Form {
        &self.factors
    }

    /// The base signature of the dimension.
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    /// The canonical form of the dimension.
    pub fn canonical_form(&self) -> &Form {
        &self.canonical
    }

    /// Returns `true` iff canonically bare dimensionless (i.e., a pure number).
    pub fn is_dimensionless(&self) -> bool {
        self.canonical.is_empty()
    }

    /// Returns `true` iff the base signature is empty.
    pub fn has_dimensionless_signature(&self) -> bool {
        self.signature.0.is_empty()
    }
}


// ---- algebra ----
impl Dimension {
    /// Multiplies two dimensions.
    ///
    /// # Errors
    /// Returns [`DimensionError::ExponentOverflow`] if combining a shared atom's exponents overflows.
    pub fn try_mul(&self, rhs: &Self) -> Result<Self, DimensionError> {
        let factors = self.factors.mul(&rhs.factors)?;
        let signature = Signature(self.signature.0.mul(&rhs.signature.0)?);
        let canonical = self.canonical.mul(&rhs.canonical)?;
        Ok(Self {
            factors,
            signature,
            canonical,
        })
    }

    /// Divides `self` by `rhs`.
    ///
    /// # Errors
    /// Returns [`DimensionError::ExponentOverflow`] if combining a shared atom's exponents overflows.
    pub fn try_div(&self, rhs: &Self) -> Result<Self, DimensionError> {
        self.try_mul(&rhs.recip()?)
    }

    /// Raises `self` to the power of `e`.
    ///
    /// # Errors
    /// Returns [`DimensionError::ExponentOverflow`] if multiplying an atom's exponent by `e` overflows.
    pub fn pow(&self, e: Exp) -> Result<Self, DimensionError> {
        let factors = self.factors.pow(e)?;
        let signature = Signature(self.signature.0.pow(e)?);
        let canonical = self.canonical.pow(e)?;
        Ok(Self {
            factors,
            signature,
            canonical,
        })
    }

    /// Computes the reciprocal of `self` by raising it to the power of `-1`.
    ///
    /// # Errors
    /// Returns [`DimensionError::ExponentOverflow`] if computing the reciprocal of an atom's exponents overflows.
    pub fn recip(&self) -> Result<Self, DimensionError> {
        let factors = self.factors.recip()?;
        let signature = Signature(self.signature.0.recip()?);
        let canonical = self.canonical.recip()?;
        Ok(Self {
            factors,
            signature,
            canonical,
        })
    }
}


// ---- compatibility queries ----
impl Dimension {
    /// Returns the [`Compatibility`] between `self` and `rhs`.
    pub fn compatibility(&self, rhs: &Self) -> Compatibility {
        if self == rhs {
            Compatibility::Full
        } else if self.signature == rhs.signature {
            Compatibility::Partial
        } else {
            Compatibility::Incompatible
        }
    }
}

impl PartialEq for Dimension {
    /// Tests for `self` and `other` values to be equal.
    ///
    /// Two dimensions are equal iff they are canonically equivalent.
    fn eq(&self, other: &Self) -> bool {
        self.canonical == other.canonical
    }
}

impl Eq for Dimension {}

#[cfg(test)]
mod tests {
    use crate::test_utils::make_form_entry;

    use super::*;

    /// Assert factors, signature, and canonical of two dimensions are equal.
    ///
    /// Caller must pass entries already sorted/reduced.
    fn assert_exactly_eq(dimension: &Dimension, other_dimension: &Dimension) -> () {
        assert_eq!(
            dimension.factors, other_dimension.factors,
            "factors must match"
        );
        assert_eq!(
            dimension.signature, other_dimension.signature,
            "signature must match"
        );
        assert_eq!(
            dimension.canonical, other_dimension.canonical,
            "canonical must match"
        )
    }

    #[test]
    fn dimensionless_has_empty_canonical_and_signature() {
        let dimensionless = Dimension::dimensionless();
        assert_eq!(dimensionless.canonical, Form::empty());
        assert_eq!(dimensionless.signature, Signature(Form::empty()));
    }

    #[test]
    fn dimensionless_equals_itself() {
        assert_eq!(Dimension::dimensionless(), Dimension::dimensionless());
    }

    #[test]
    fn compatibility_full_for_empty_canonical_forms() {
        let canonical1 = Form::empty();
        let canonical2 = Form::empty();
        let dimension1 = Dimension {
            canonical: canonical1,
            ..Dimension::dimensionless()
        };
        let dimension2 = Dimension {
            canonical: canonical2,
            ..Dimension::dimensionless()
        };
        assert!(matches!(
            dimension1.compatibility(&dimension2),
            Compatibility::Full
        ));
    }

    #[test]
    fn compatibility_full_for_equal_canonical_forms() {
        let canonical1 = Form::raw(vec![
            make_form_entry(0, (1, 2)),
            make_form_entry(2, (-4, 3)),
        ]);
        let canonical2 = canonical1.clone();
        let lhs = Dimension {
            canonical: canonical1,
            ..Dimension::dimensionless()
        };
        let rhs = Dimension {
            canonical: canonical2,
            ..Dimension::dimensionless()
        };
        assert!(matches!(lhs.compatibility(&rhs), Compatibility::Full));
    }

    #[test]
    fn compatibility_incompatible_for_different_signatures() {
        let canonical1 = Form::raw(vec![make_form_entry(0, (1, 2))]);
        let signature1 = Signature(Form::empty());
        let signature2 = Signature(Form::raw(vec![make_form_entry(0, (1, 2))]));
        let dimension1 = Dimension {
            canonical: canonical1,
            signature: signature1,
            ..Dimension::dimensionless()
        };
        let dimension2 = Dimension {
            signature: signature2,
            ..Dimension::dimensionless()
        };
        assert!(matches!(
            dimension1.compatibility(&dimension2),
            Compatibility::Incompatible
        ));
    }

    #[test]
    fn compatibility_partial_for_same_signature_different_canonical_forms() {
        let canonical1 = Form::empty();
        let canonical2 = Form::raw(vec![make_form_entry(0, (1, 2))]);
        let signature1 = Signature(Form::raw(vec![make_form_entry(1, (-3, 4))]));
        let signature2 = Signature(Form::raw(vec![make_form_entry(1, (-3, 4))]));
        let lhs = Dimension {
            signature: signature1,
            canonical: canonical1,
            ..Dimension::dimensionless()
        };
        let rhs = Dimension {
            signature: signature2,
            canonical: canonical2,
            ..Dimension::dimensionless()
        };
        assert!(matches!(lhs.compatibility(&rhs), Compatibility::Partial));
    }

    #[test]
    fn compatibility_partial_for_torque_vs_energy() {
        // Torque is r x F, therefore
        // canonical = [(L, 1), (force_id, 1)] ; signature = [(T, -2), (L, 2), (M, 1)]
        // Energy (potential) is m.g.h, therefore
        // canonical = [(L, 1), (M, 1), (acceleration_id, 1)] ; signature = [(T, -2), (L, 2), (M, 1)]
        let (time_id, length_id, mass_id) = (0, 1, 2);
        let acceleration_id = 3;
        let force_id = 4;
        let torque_canonical = Form::raw(vec![
            make_form_entry(length_id, (1, 1)),
            make_form_entry(force_id, (1, 1)),
        ]);
        let torque_signature = Signature::raw(vec![
            make_form_entry(time_id, (-2, 1)),
            make_form_entry(length_id, (2, 1)),
            make_form_entry(mass_id, (1, 1)),
        ]);
        let energy_canonical = Form::raw(vec![
            make_form_entry(length_id, (1, 1)),
            make_form_entry(mass_id, (1, 1)),
            make_form_entry(acceleration_id, (1, 1)),
        ]);
        let energy_signature = Signature::raw(vec![
            make_form_entry(time_id, (-2, 1)),
            make_form_entry(length_id, (2, 1)),
            make_form_entry(mass_id, (1, 1)),
        ]);
        let torque = Dimension {
            canonical: torque_canonical,
            signature: torque_signature,
            ..Dimension::dimensionless()
        };
        let energy = Dimension {
            canonical: energy_canonical,
            signature: energy_signature,
            ..Dimension::dimensionless()
        };
        assert!(matches!(
            torque.compatibility(&energy),
            Compatibility::Partial
        ));
    }

    #[test]
    fn compatibility_partial_for_plane_angle_vs_bare_number() {
        // Plane angle has
        // canonical = [(plane_angle_id, 1)] ; signature = []
        // Bare number has
        // canonical = [] ; signature = []
        let plane_angle_id = 0;
        let plane_angle_canonical = Form::raw(vec![make_form_entry(plane_angle_id, (1, 1))]);
        let plane_angle = Dimension {
            canonical: plane_angle_canonical,
            ..Dimension::dimensionless()
        };
        let bare_number = Dimension::dimensionless();
        assert!(matches!(
            plane_angle.compatibility(&bare_number),
            Compatibility::Partial
        ));
    }

    #[test]
    fn compatibility_partial_for_angular_velocity_vs_frequency() {
        // Angular velocity has
        // canonical = [(T, -1), (plane_angle_id, 1)] ; signature = [(T, -1)]
        // Bare number has
        // canonical = [(T, -1)] ; signature = [(T, -1)]
        let (time_id, plane_angle_id) = (0, 1);
        let angular_velocity_canonical = Form::raw(vec![
            make_form_entry(time_id, (-1, 1)),
            make_form_entry(plane_angle_id, (1, 1)),
        ]);
        let angular_velocity_signature = Signature::raw(vec![make_form_entry(time_id, (-1, 1))]);
        let frequency_canonical = Form::raw(vec![make_form_entry(time_id, (-1, 1))]);
        let frequency_signature = Signature::raw(vec![make_form_entry(time_id, (-1, 1))]);
        let angular_velocity = Dimension {
            canonical: angular_velocity_canonical,
            signature: angular_velocity_signature,
            ..Dimension::dimensionless()
        };
        let frequency = Dimension {
            canonical: frequency_canonical,
            signature: frequency_signature,
            ..Dimension::dimensionless()
        };
        assert!(matches!(
            angular_velocity.compatibility(&frequency),
            Compatibility::Partial
        ));
    }

    #[test]
    fn try_mul_combines_all_three_forms() {
        let factors1 = Form::raw(vec![
            make_form_entry(0, (-1, 2)),
            make_form_entry(2, (4, 3)),
        ]);
        let factors2 = Form::raw(vec![make_form_entry(0, (1, 2)), make_form_entry(1, (2, 3))]);
        let factors12 = Form::raw(vec![make_form_entry(1, (2, 3)), make_form_entry(2, (4, 3))]);
        let signature1 = Signature::raw(vec![make_form_entry(1, (-3, 2))]);
        let signature2 = Signature::raw(vec![make_form_entry(1, (1, 2))]);
        let signature12 = Signature::raw(vec![make_form_entry(1, (-1, 1))]);
        let canonical1 = Form::raw(vec![
            make_form_entry(3, (10, 7)),
            make_form_entry(5, (-6, 7)),
        ]);
        let canonical2 = Form::raw(vec![
            make_form_entry(3, (-10, 7)),
            make_form_entry(5, (6, 7)),
        ]);
        let canonical12 = Form::raw(vec![]);
        let dimension1 = Dimension {
            factors: factors1,
            signature: signature1,
            canonical: canonical1,
        };
        let dimension2 = Dimension {
            factors: factors2,
            signature: signature2,
            canonical: canonical2,
        };
        let dimension12 = Dimension {
            factors: factors12,
            signature: signature12,
            canonical: canonical12,
        };
        assert_exactly_eq(&dimension1.try_mul(&dimension2).unwrap(), &dimension12);
    }

    #[test]
    fn try_mul_propagates_exponent_overflow() {
        let lhs_factors = Form::raw(vec![make_form_entry(0, (i64::MAX, 1))]);
        let lhs = Dimension {
            factors: lhs_factors,
            ..Dimension::dimensionless()
        };
        let rhs_factors = Form::raw(vec![make_form_entry(0, (2, 1))]);
        let rhs = Dimension {
            factors: rhs_factors,
            ..Dimension::dimensionless()
        };
        assert!(matches!(
            lhs.try_mul(&rhs),
            Err(DimensionError::ExponentOverflow)
        ));
    }

    #[test]
    fn try_div_of_self_gives_dimensionless() {
        let factors = Form::raw(vec![
            make_form_entry(0, (-1, 2)),
            make_form_entry(2, (4, 3)),
        ]);
        let signature = Signature::raw(vec![make_form_entry(1, (-3, 2))]);
        let canonical = Form::raw(vec![
            make_form_entry(3, (10, 7)),
            make_form_entry(5, (6, 7)),
        ]);
        let dimension = Dimension {
            factors,
            signature,
            canonical,
        };
        assert!(dimension.try_div(&dimension).unwrap().is_dimensionless());
    }

    #[test]
    fn pow_zero_gives_dimensionless() {
        let factors = Form::raw(vec![
            make_form_entry(0, (-1, 2)),
            make_form_entry(2, (4, 3)),
        ]);
        let signature = Signature::raw(vec![make_form_entry(1, (-3, 2))]);
        let canonical = Form::raw(vec![
            make_form_entry(3, (10, 7)),
            make_form_entry(5, (6, 7)),
        ]);
        let dimension = Dimension {
            factors,
            signature,
            canonical,
        };
        assert!(dimension.pow(Exp::ZERO).unwrap().is_dimensionless());
    }

    #[test]
    fn pow_matches_recip_at_exponent_negative_one() {
        let factors = Form::raw(vec![
            make_form_entry(0, (-1, 2)),
            make_form_entry(2, (4, 3)),
        ]);
        let signature = Signature::raw(vec![make_form_entry(1, (-3, 2))]);
        let canonical = Form::raw(vec![
            make_form_entry(3, (10, 7)),
            make_form_entry(5, (6, 7)),
        ]);
        let dimension = Dimension {
            factors,
            signature,
            canonical,
        };
        assert_exactly_eq(
            &dimension.pow(Exp::int(-1).unwrap()).unwrap(),
            &dimension.recip().unwrap(),
        );
    }

    #[test]
    fn recip_of_recip_is_identity() {
        let factors = Form::raw(vec![
            make_form_entry(0, (-1, 2)),
            make_form_entry(2, (4, 3)),
        ]);
        let signature = Signature::raw(vec![make_form_entry(1, (-3, 2))]);
        let canonical = Form::raw(vec![
            make_form_entry(3, (10, 7)),
            make_form_entry(5, (6, 7)),
        ]);
        let dimension = Dimension {
            factors,
            signature,
            canonical,
        };
        assert_exactly_eq(&dimension.recip().unwrap().recip().unwrap(), &dimension);
    }

    #[test]
    fn eq_ignores_factors_when_canonical_matches() {
        let factors = Form::raw(vec![
            make_form_entry(0, (-1, 2)),
            make_form_entry(2, (4, 3)),
        ]);
        let canonical = Form::raw(vec![
            make_form_entry(3, (10, 7)),
            make_form_entry(5, (6, 7)),
        ]);
        let dimension = Dimension {
            factors,
            canonical: canonical.clone(),
            ..Dimension::dimensionless()
        };
        let other_dimension = Dimension {
            canonical: canonical.clone(),
            ..Dimension::dimensionless()
        };
        assert!(dimension == other_dimension);
    }
}
