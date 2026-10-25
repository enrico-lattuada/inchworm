//! The core [`Dimension`] value type and dimension-to-dimension [`Compatibility`].
//!
//! Every dimension carries three related forms: `factors` (the stored,
//! display-facing composite the user built), `signature` (fully expanded to
//! base atoms, used for [`Compatibility::Partial`]), and `canonical`
//! (expanded, but keeping named-dimensionless atoms like `plane_angle`
//! irreducible, used for [`Compatibility::Full`] and `==`).

use std::fmt;
use std::sync::{Arc, LazyLock};

use crate::{
    DimRegistryId, DimensionError, Exp, Form, Signature,
    atom::{AtomKind, DimAtom},
};

/// How two [`Dimension`]s relate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// The three forms of a [`Dimension`], shared behind an [`Arc`] so that a
/// `Dimension` is one pointer wide and cloning it is a reference-count increment.
#[derive(Debug)]
struct DimensionData {
    /// Stored (display/identity) form: the named composite the user built,
    /// with like factors combined and zero powers dropped.
    /// `velocity * time` stores `{velocity¹, time¹}` — NOT reduced to `{length¹}`.
    factors: Form,
    /// Full expansion to base atoms (dimensionless kinds vanish).
    signature: Signature,
    /// Expansion to base atoms + irreducible dimensionless-kind atoms.
    canonical: Form,
}

impl DimensionData {
    /// All three forms empty: the bare dimensionless value.
    fn empty() -> Self {
        Self {
            factors: Form::empty(),
            signature: Signature(Form::empty()),
            canonical: Form::empty(),
        }
    }
}

/// An immutable dimension value.
///
/// Values are self-contained ([`Arc`](std::sync::Arc)-backed) and never need the
/// [`DimRegistry`](crate::DimRegistry) that produced them to stay alive or to
/// be used in arithmetic. `==` compares canonical forms: an expression like
/// `length / time` and a dimension named `velocity` compare equal even though
/// they carry different [`factors`](Dimension::factors) — the stored form
/// distinguishes "the named atom" from "the raw computation" while `==`
/// treats them as the same physical dimension.
///
/// # Examples
///
/// ```
/// use inchworm_dimensions::DimRegistry;
///
/// let mut registry = DimRegistry::new("mechanics");
/// let length = registry.add_base("length", Some("L")).unwrap();
/// let time = registry.add_base("time", Some("T")).unwrap();
/// let velocity = registry.add_derived_expr("velocity", "length / time").unwrap();
///
/// // Arithmetic on the base dimensions is canonically equal to the named one.
/// let length_over_time = length.try_div(&time).unwrap();
/// assert_eq!(length_over_time, velocity);
///
/// // But the named dimension keeps its own identity in its stored form.
/// assert_eq!(velocity.name(), Some("velocity"));
/// assert_ne!(velocity.factors(), length_over_time.factors());
/// ```
#[derive(Clone, Debug)]
pub struct Dimension(Arc<DimensionData>);

static DIMENSIONLESS: LazyLock<Dimension> =
    LazyLock::new(|| Dimension::new(DimensionData::empty()));

impl Dimension {
    /// The only place a `Dimension` is built.
    fn new(data: DimensionData) -> Self {
        Self(Arc::new(data))
    }

    /// Returns a dimensionless dimension.
    ///
    /// The universal bare-dimensionless value; belongs to no registry and is
    /// usable in arithmetic with dimensions of any registry.
    pub fn dimensionless() -> Self {
        DIMENSIONLESS.clone()
    }

    /// Returns a dimension from an [`Atom`].
    pub(crate) fn from_atom(atom: &DimAtom) -> Self {
        match &atom.kind {
            AtomKind::Base => {
                let factors = Form::single(atom, Exp::ONE);
                let signature = Signature(factors.clone());
                let canonical = factors.clone();
                Self::new(DimensionData {
                    factors,
                    signature,
                    canonical,
                })
            }
            AtomKind::Derived {
                definition,
                dimensionless_kind,
            } => {
                let factors = Form::single(atom, Exp::ONE);
                let signature = definition.signature().clone();
                let canonical = if *dimensionless_kind {
                    factors.clone()
                } else {
                    definition.canonical_form().clone()
                };
                Self::new(DimensionData {
                    factors,
                    signature,
                    canonical,
                })
            }
        }
    }

    /// The stored named-composite factors (what `Display` prints).
    pub fn factors(&self) -> &Form {
        &self.0.factors
    }

    /// The base signature of the dimension.
    pub fn signature(&self) -> &Signature {
        &self.0.signature
    }

    /// The canonical form of the dimension.
    pub fn canonical_form(&self) -> &Form {
        &self.0.canonical
    }

    /// Returns `true` iff canonically bare dimensionless (i.e., a pure number).
    pub fn is_dimensionless(&self) -> bool {
        self.canonical_form().is_empty()
    }

    /// Returns `true` iff the base signature is empty.
    pub fn has_dimensionless_signature(&self) -> bool {
        self.signature().0.is_empty()
    }

    /// Returns the [`DimRegistryId`] of the registry where `self` is defined.
    pub fn registry_id(&self) -> Option<DimRegistryId> {
        self.factors()
            .entries()
            .first()
            .map(|(atom, _)| atom.registry_id)
    }

    /// Returns an Atom if `factors` is a single `(atom, Exp::ONE)` entry.
    fn as_named_atom(&self) -> Option<&DimAtom> {
        self.factors().single_atom()
    }

    /// Returns `true` iff `factors` is a single `(atom, Exp::ONE)` entry.
    pub fn is_named(&self) -> bool {
        self.as_named_atom().is_some()
    }

    /// Returns the dimension name iff [`Self::is_named`] returns `true`.
    pub fn name(&self) -> Option<&str> {
        self.as_named_atom().map(|atom| atom.name.as_ref())
    }

    /// Returns the dimension symbol iff [`Self::is_named`] returns `true` and a symbol is stored.
    pub fn symbol(&self) -> Option<&str> {
        self.as_named_atom().and_then(|atom| atom.symbol.as_deref())
    }

    /// Returns `true` iff `self`'s canonical form reduces to a single base
    /// dimension or named dimensionless kind.
    pub fn is_irreducible(&self) -> bool {
        self.canonical_form().single_atom().is_some()
    }
}

// ---- algebra ----
impl Dimension {
    /// Multiplies two dimensions.
    ///
    /// # Errors
    ///
    /// Returns [`DimensionError::ExponentOverflow`] if combining a shared atom's exponents overflows.
    /// Returns [`DimensionError::CrossRegistry`] if combining atoms from different registries.
    pub fn try_mul(&self, rhs: &Self) -> Result<Self, DimensionError> {
        if let (Some(lhs_id), Some(rhs_id)) = (self.registry_id(), rhs.registry_id())
            && lhs_id != rhs_id
        {
            return Err(DimensionError::CrossRegistry {
                left: lhs_id,
                right: rhs_id,
            });
        }
        let factors = self.factors().mul(rhs.factors())?;
        let signature = Signature(self.signature().0.mul(&rhs.signature().0)?);
        let canonical = self.canonical_form().mul(rhs.canonical_form())?;
        Ok(Self::new(DimensionData {
            factors,
            signature,
            canonical,
        }))
    }

    /// Divides `self` by `rhs`.
    ///
    /// # Errors
    ///
    /// Returns [`DimensionError::ExponentOverflow`] if combining a shared atom's exponents overflows.
    /// Returns [`DimensionError::CrossRegistry`] if combining atoms from different registries.
    pub fn try_div(&self, rhs: &Self) -> Result<Self, DimensionError> {
        self.try_mul(&rhs.recip()?)
    }

    /// Raises `self` to the power of `e`.
    ///
    /// # Errors
    ///
    /// Returns [`DimensionError::ExponentOverflow`] if multiplying an atom's exponent by `e` overflows.
    pub fn pow(&self, e: Exp) -> Result<Self, DimensionError> {
        let factors = self.factors().pow(e)?;
        let signature = Signature(self.signature().0.pow(e)?);
        let canonical = self.canonical_form().pow(e)?;
        Ok(Self::new(DimensionData {
            factors,
            signature,
            canonical,
        }))
    }

    /// Computes the reciprocal of `self` by raising it to the power of `-1`.
    ///
    /// # Errors
    ///
    /// Returns [`DimensionError::ExponentOverflow`] if computing the reciprocal of an atom's exponents overflows.
    pub fn recip(&self) -> Result<Self, DimensionError> {
        let factors = self.factors().recip()?;
        let signature = Signature(self.signature().0.recip()?);
        let canonical = self.canonical_form().recip()?;
        Ok(Self::new(DimensionData {
            factors,
            signature,
            canonical,
        }))
    }
}

// ---- compatibility queries ----
impl Dimension {
    /// Returns the [`Compatibility`] between `self` and `rhs`.
    pub fn compatibility(&self, rhs: &Self) -> Compatibility {
        if self == rhs {
            Compatibility::Full
        } else if self.signature() == rhs.signature() {
            Compatibility::Partial
        } else {
            Compatibility::Incompatible
        }
    }
}

impl std::hash::Hash for Dimension {
    /// Two dimensions are equal iff they are canonically equivalent.
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.canonical_form().hash(state);
    }
}

impl PartialEq for Dimension {
    /// Tests for `self` and `other` values to be equal.
    ///
    /// Two dimensions are equal iff they are canonically equivalent.
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.canonical_form() == other.canonical_form()
    }
}

impl Eq for Dimension {}

/// Multiplies two dimensions, delegating to [`Dimension::try_mul`].
///
/// # Panics
///
/// Panics on cross-registry mixing or exponent overflow, the same two cases
/// [`Dimension::try_mul`] returns as errors.
impl std::ops::Mul for &Dimension {
    type Output = Dimension;
    fn mul(self, rhs: &Dimension) -> Self::Output {
        self.try_mul(rhs).expect("dimension multiplication failed")
    }
}

/// Delegates to the `&Dimension * &Dimension` impl; panics under the same conditions.
impl std::ops::Mul for Dimension {
    type Output = Dimension;
    fn mul(self, rhs: Dimension) -> Self::Output {
        &self * &rhs
    }
}

/// Delegates to the `&Dimension * &Dimension` impl; panics under the same conditions.
impl std::ops::Mul<&Dimension> for Dimension {
    type Output = Dimension;
    fn mul(self, rhs: &Dimension) -> Self::Output {
        &self * rhs
    }
}

/// Delegates to the `&Dimension * &Dimension` impl; panics under the same conditions.
impl std::ops::Mul<Dimension> for &Dimension {
    type Output = Dimension;
    fn mul(self, rhs: Dimension) -> Self::Output {
        self * &rhs
    }
}

/// Divides two dimensions, delegating to [`Dimension::try_div`].
///
/// # Panics
///
/// Panics on cross-registry mixing or exponent overflow, the same two cases
/// [`Dimension::try_div`] returns as errors.
impl std::ops::Div for &Dimension {
    type Output = Dimension;
    fn div(self, rhs: &Dimension) -> Self::Output {
        self.try_div(rhs).expect("dimension division failed")
    }
}

/// Delegates to the `&Dimension / &Dimension` impl; panics under the same conditions.
impl std::ops::Div for Dimension {
    type Output = Dimension;
    fn div(self, rhs: Dimension) -> Self::Output {
        &self / &rhs
    }
}

/// Delegates to the `&Dimension / &Dimension` impl; panics under the same conditions.
impl std::ops::Div<&Dimension> for Dimension {
    type Output = Dimension;
    fn div(self, rhs: &Dimension) -> Self::Output {
        &self / rhs
    }
}

/// Delegates to the `&Dimension / &Dimension` impl; panics under the same conditions.
impl std::ops::Div<Dimension> for &Dimension {
    type Output = Dimension;
    fn div(self, rhs: Dimension) -> Self::Output {
        self / &rhs
    }
}

impl fmt::Display for Dimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.factors().fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{
        assert_exactly_eq, errors_match, make_form_entry, make_form_entry_in_registry,
    };

    #[test]
    fn is_one_pointer_wide() {
        assert_eq!(size_of::<Dimension>(), size_of::<usize>());
    }

    mod dimensionless {
        use super::*;

        #[test]
        fn has_empty_canonical_and_signature() {
            let dimensionless = Dimension::dimensionless();
            assert_eq!(dimensionless.canonical_form(), &Form::empty());
            assert_eq!(dimensionless.signature(), &Signature(Form::empty()));
        }

        #[test]
        fn equals_itself() {
            assert_eq!(Dimension::dimensionless(), Dimension::dimensionless());
        }
    }

    mod registry_id {
        use super::*;

        #[test]
        fn basic() {
            let factors = Form::raw(vec![make_form_entry(0, (1, 2))]);
            let dimension = Dimension::new(DimensionData {
                factors: factors.clone(),
                ..DimensionData::empty()
            });
            assert!(dimension.registry_id().is_some());
            let dim_registry_id = dimension.registry_id().unwrap();
            let fac_registry_id = factors.entries()[0].0.registry_id;
            assert_eq!(dim_registry_id, fac_registry_id);
        }

        #[test]
        fn dimensionless_registry_id_is_none() {
            assert_eq!(Dimension::dimensionless().registry_id(), None);
        }
    }

    mod is_named {
        use super::*;
        use crate::DimRegistry;

        #[test]
        fn base_dimension_with_symbol_reports_name_and_symbol() {
            let mut registry = DimRegistry::new("test-reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            assert!(length.is_named());
            assert_eq!(length.name(), Some("length"));
            assert_eq!(length.symbol(), Some("L"));
        }

        #[test]
        fn base_dimension_without_symbol_reports_name_but_no_symbol() {
            let mut registry = DimRegistry::new("test-reg");
            let length = registry.add_base("length", None).unwrap();
            assert!(length.is_named());
            assert_eq!(length.name(), Some("length"));
            assert_eq!(length.symbol(), None);
        }

        #[test]
        fn derived_dimension_reports_name_but_no_symbol() {
            let mut registry = DimRegistry::new("test-reg");
            registry.add_base("length", Some("L")).unwrap();
            let distance = registry.add_derived_expr("distance", "length").unwrap();
            assert!(distance.is_named());
            assert_eq!(distance.name(), Some("distance"));
            assert_eq!(distance.symbol(), None);
        }

        #[test]
        fn unnamed_composite_dimension_is_not_named() {
            let mut registry = DimRegistry::new("test-reg");
            registry.add_base("length", Some("L")).unwrap();
            registry.add_base("time", Some("T")).unwrap();
            let velocity = registry.parse("length / time").unwrap();
            assert!(!velocity.is_named());
            assert!(velocity.name().is_none());
            assert!(velocity.symbol().is_none());
        }

        #[test]
        fn bare_dimensionless_is_not_named() {
            let dimensionless = Dimension::dimensionless();
            assert!(!dimensionless.is_named());
            assert!(dimensionless.name().is_none());
            assert!(dimensionless.symbol().is_none());
        }

        #[test]
        fn single_atom_with_non_unit_exponent_is_not_named() {
            let mut registry = DimRegistry::new("test-reg");
            let length = registry.add_base("length", Some("L")).unwrap();
            let area = length.pow(Exp::int(2)).unwrap();
            assert!(!area.is_named());
            assert!(area.name().is_none());
            assert!(area.symbol().is_none());
        }
    }

    mod compatibility {
        use super::*;

        #[test]
        fn compatibility_full_for_empty_canonical_forms() {
            let canonical1 = Form::empty();
            let canonical2 = Form::empty();
            let dimension1 = Dimension::new(DimensionData {
                canonical: canonical1,
                ..DimensionData::empty()
            });
            let dimension2 = Dimension::new(DimensionData {
                canonical: canonical2,
                ..DimensionData::empty()
            });
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
            let lhs = Dimension::new(DimensionData {
                canonical: canonical1,
                ..DimensionData::empty()
            });
            let rhs = Dimension::new(DimensionData {
                canonical: canonical2,
                ..DimensionData::empty()
            });
            assert!(matches!(lhs.compatibility(&rhs), Compatibility::Full));
        }

        #[test]
        fn compatibility_incompatible_for_different_signatures() {
            let canonical1 = Form::raw(vec![make_form_entry(0, (1, 2))]);
            let signature1 = Signature(Form::empty());
            let signature2 = Signature(Form::raw(vec![make_form_entry(0, (1, 2))]));
            let dimension1 = Dimension::new(DimensionData {
                canonical: canonical1,
                signature: signature1,
                ..DimensionData::empty()
            });
            let dimension2 = Dimension::new(DimensionData {
                signature: signature2,
                ..DimensionData::empty()
            });
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
            let lhs = Dimension::new(DimensionData {
                signature: signature1,
                canonical: canonical1,
                ..DimensionData::empty()
            });
            let rhs = Dimension::new(DimensionData {
                signature: signature2,
                canonical: canonical2,
                ..DimensionData::empty()
            });
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
            let torque = Dimension::new(DimensionData {
                canonical: torque_canonical,
                signature: torque_signature,
                ..DimensionData::empty()
            });
            let energy = Dimension::new(DimensionData {
                canonical: energy_canonical,
                signature: energy_signature,
                ..DimensionData::empty()
            });
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
            let plane_angle = Dimension::new(DimensionData {
                canonical: plane_angle_canonical,
                ..DimensionData::empty()
            });
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
            let angular_velocity_signature =
                Signature::raw(vec![make_form_entry(time_id, (-1, 1))]);
            let frequency_canonical = Form::raw(vec![make_form_entry(time_id, (-1, 1))]);
            let frequency_signature = Signature::raw(vec![make_form_entry(time_id, (-1, 1))]);
            let angular_velocity = Dimension::new(DimensionData {
                canonical: angular_velocity_canonical,
                signature: angular_velocity_signature,
                ..DimensionData::empty()
            });
            let frequency = Dimension::new(DimensionData {
                canonical: frequency_canonical,
                signature: frequency_signature,
                ..DimensionData::empty()
            });
            assert!(matches!(
                angular_velocity.compatibility(&frequency),
                Compatibility::Partial
            ));
        }
    }

    mod try_mul {
        use super::*;

        #[test]
        fn combines_all_three_forms() {
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
            let dimension1 = Dimension::new(DimensionData {
                factors: factors1,
                signature: signature1,
                canonical: canonical1,
            });
            let dimension2 = Dimension::new(DimensionData {
                factors: factors2,
                signature: signature2,
                canonical: canonical2,
            });
            let dimension12 = Dimension::new(DimensionData {
                factors: factors12,
                signature: signature12,
                canonical: canonical12,
            });
            assert_exactly_eq(&dimension1.try_mul(&dimension2).unwrap(), &dimension12);
        }

        #[test]
        fn err_on_exponent_overflow() {
            let lhs_factors = Form::raw(vec![make_form_entry(0, (i64::MAX, 1))]);
            let lhs = Dimension::new(DimensionData {
                factors: lhs_factors,
                ..DimensionData::empty()
            });
            let rhs_factors = Form::raw(vec![make_form_entry(0, (2, 1))]);
            let rhs = Dimension::new(DimensionData {
                factors: rhs_factors,
                ..DimensionData::empty()
            });
            assert!(errors_match(
                &lhs.try_mul(&rhs).unwrap_err(),
                &DimensionError::ExponentOverflow
            ));
        }
    }

    mod mul_operator {
        use super::*;

        #[test]
        fn all_reference_combinations_agree_with_try_mul() {
            let factors1 = Form::raw(vec![
                make_form_entry(0, (-1, 2)),
                make_form_entry(2, (4, 3)),
            ]);
            let signature1 = Signature::raw(vec![make_form_entry(1, (-3, 2))]);
            let canonical1 = Form::raw(vec![
                make_form_entry(3, (10, 7)),
                make_form_entry(5, (-6, 7)),
            ]);
            let factors2 = Form::raw(vec![make_form_entry(0, (1, 2)), make_form_entry(1, (2, 3))]);
            let signature2 = Signature::raw(vec![make_form_entry(1, (1, 2))]);
            let canonical2 = Form::raw(vec![
                make_form_entry(3, (-10, 7)),
                make_form_entry(5, (6, 7)),
            ]);
            let dimension1 = Dimension::new(DimensionData {
                factors: factors1,
                signature: signature1,
                canonical: canonical1,
            });
            let dimension2 = Dimension::new(DimensionData {
                factors: factors2,
                signature: signature2,
                canonical: canonical2,
            });
            let dim1_times_dim2 = dimension1.try_mul(&dimension2).unwrap();
            assert_exactly_eq(&(&dimension1 * &dimension2), &dim1_times_dim2);
            assert_exactly_eq(&(&dimension1 * dimension2.clone()), &dim1_times_dim2);
            assert_exactly_eq(&(dimension1.clone() * &dimension2), &dim1_times_dim2);
            assert_exactly_eq(&(dimension1 * dimension2), &dim1_times_dim2);
        }

        #[test]
        #[should_panic]
        fn panics_on_cross_registry_mixing() {
            let a = Dimension::new(DimensionData {
                factors: Form::raw(vec![make_form_entry_in_registry(0, (1, 1), 0)]),
                ..DimensionData::empty()
            });
            let b = Dimension::new(DimensionData {
                factors: Form::raw(vec![make_form_entry_in_registry(1, (1, 1), 1)]),
                ..DimensionData::empty()
            });
            let _ = &a * &b;
        }

        #[test]
        #[should_panic]
        fn panics_on_exponent_overflow() {
            let lhs_factors = Form::raw(vec![make_form_entry(0, (i64::MAX, 1))]);
            let lhs = Dimension::new(DimensionData {
                factors: lhs_factors,
                ..DimensionData::empty()
            });
            let rhs_factors = Form::raw(vec![make_form_entry(0, (2, 1))]);
            let rhs = Dimension::new(DimensionData {
                factors: rhs_factors,
                ..DimensionData::empty()
            });
            let _ = lhs * rhs;
        }
    }

    mod try_div {
        use super::*;

        #[test]
        fn div_of_self_gives_dimensionless() {
            let factors = Form::raw(vec![
                make_form_entry(0, (-1, 2)),
                make_form_entry(2, (4, 3)),
            ]);
            let signature = Signature::raw(vec![make_form_entry(1, (-3, 2))]);
            let canonical = Form::raw(vec![
                make_form_entry(3, (10, 7)),
                make_form_entry(5, (6, 7)),
            ]);
            let dimension = Dimension::new(DimensionData {
                factors,
                signature,
                canonical,
            });
            assert!(dimension.try_div(&dimension).unwrap().is_dimensionless());
        }
    }

    mod div_operator {
        use super::*;

        #[test]
        fn all_reference_combinations_agree_with_try_div() {
            let factors = Form::raw(vec![
                make_form_entry(0, (-1, 2)),
                make_form_entry(2, (4, 3)),
            ]);
            let signature = Signature::raw(vec![make_form_entry(1, (-3, 2))]);
            let canonical = Form::raw(vec![
                make_form_entry(3, (10, 7)),
                make_form_entry(5, (6, 7)),
            ]);
            let dimension = Dimension::new(DimensionData {
                factors,
                signature,
                canonical,
            });
            let dim_div_dim = dimension.try_div(&dimension).unwrap();
            assert_exactly_eq(&(&dimension / &dimension), &dim_div_dim);
            assert_exactly_eq(&(&dimension / dimension.clone()), &dim_div_dim);
            assert_exactly_eq(&(dimension.clone() / &dimension), &dim_div_dim);
            assert_exactly_eq(&(dimension.clone() / dimension), &dim_div_dim);
        }

        #[test]
        #[should_panic]
        fn panics_on_cross_registry_mixing() {
            let a = Dimension::new(DimensionData {
                factors: Form::raw(vec![make_form_entry_in_registry(0, (1, 1), 0)]),
                ..DimensionData::empty()
            });
            let b = Dimension::new(DimensionData {
                factors: Form::raw(vec![make_form_entry_in_registry(1, (1, 1), 1)]),
                ..DimensionData::empty()
            });
            let _ = &a / &b;
        }

        #[test]
        #[should_panic]
        fn panics_on_exponent_overflow() {
            let lhs_factors = Form::raw(vec![make_form_entry(0, (i64::MAX, 1))]);
            let lhs = Dimension::new(DimensionData {
                factors: lhs_factors,
                ..DimensionData::empty()
            });
            let rhs_factors = Form::raw(vec![make_form_entry(0, (-2, 1))]);
            let rhs = Dimension::new(DimensionData {
                factors: rhs_factors,
                ..DimensionData::empty()
            });
            let _ = lhs / rhs;
        }
    }

    mod pow {
        use super::*;

        #[test]
        fn zero_gives_dimensionless() {
            let factors = Form::raw(vec![
                make_form_entry(0, (-1, 2)),
                make_form_entry(2, (4, 3)),
            ]);
            let signature = Signature::raw(vec![make_form_entry(1, (-3, 2))]);
            let canonical = Form::raw(vec![
                make_form_entry(3, (10, 7)),
                make_form_entry(5, (6, 7)),
            ]);
            let dimension = Dimension::new(DimensionData {
                factors,
                signature,
                canonical,
            });
            assert!(dimension.pow(Exp::ZERO).unwrap().is_dimensionless());
        }

        #[test]
        fn matches_recip_at_exponent_negative_one() {
            let factors = Form::raw(vec![
                make_form_entry(0, (-1, 2)),
                make_form_entry(2, (4, 3)),
            ]);
            let signature = Signature::raw(vec![make_form_entry(1, (-3, 2))]);
            let canonical = Form::raw(vec![
                make_form_entry(3, (10, 7)),
                make_form_entry(5, (6, 7)),
            ]);
            let dimension = Dimension::new(DimensionData {
                factors,
                signature,
                canonical,
            });
            assert_exactly_eq(
                &dimension.pow(Exp::int(-1)).unwrap(),
                &dimension.recip().unwrap(),
            );
        }
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
        let dimension = Dimension::new(DimensionData {
            factors,
            signature,
            canonical,
        });
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
        let dimension = Dimension::new(DimensionData {
            factors,
            canonical: canonical.clone(),
            ..DimensionData::empty()
        });
        let other_dimension = Dimension::new(DimensionData {
            canonical: canonical.clone(),
            ..DimensionData::empty()
        });
        assert!(dimension == other_dimension);
    }

    mod display {
        use super::*;
        use crate::DimRegistry;

        #[test]
        fn displays_named_dimension() {
            let mut registry = DimRegistry::new("test-reg");
            registry.add_base("length", Some("L")).unwrap();
            registry.add_base("time", Some("T")).unwrap();
            let velocity = registry
                .add_derived_expr("velocity", "length/time")
                .unwrap();
            assert_eq!(velocity.to_string(), "velocity");
        }

        #[test]
        fn displays_composite_dimension() {
            let mut registry = DimRegistry::new("test-reg");
            registry.add_base("length", Some("L")).unwrap();
            registry.add_base("time", Some("T")).unwrap();
            assert_eq!(
                registry.parse("length/time").unwrap().to_string(),
                "length·time^-1"
            );
        }

        #[test]
        fn displays_dimensionless_as_one() {
            assert_eq!(Dimension::dimensionless().to_string(), "1");
        }
    }
}
