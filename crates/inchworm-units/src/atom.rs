//! Atom identity and registration records.
//!
//! An [`UnitAtom`](crate::atom::UnitAtom) (`Arc<AtomData>`) is the crate's unit of
//! registered identity: every base or derived unit added to a
//! [`UnitRegistry`](crate::UnitRegistry) becomes one, tagged with a
//! process-unique [`UnitId`] and the [`UnitRegistryId`] of the registry that
//! created it. [`Unit`](crate::Unit) values hold [`Arc`](std::sync::Arc)
//! clones of the atoms in their factors, so they stay valid independent of
//! the registry's own lifetime.

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use inchworm_dimensions::Dimension;

use crate::{DeltaUnit, UnitError, UnitId, UnitRegistryId, scale::Scale};

/// A derived atom's definition expanded to base units: `1 atom = scale × base`.
///
/// Built only by [`Self::new`], so it always matches its definition.
#[derive(Debug)]
pub(crate) struct Expansion {
    scale: Scale,
    base: DeltaUnit,
}

impl Expansion {
    /// Expands `1 name = scale × definition` to base units.
    ///
    /// # Errors
    /// Returns [`UnitError::Dimension`] wrapping
    /// [`ExponentOverflow`](inchworm_dimensions::DimensionError::ExponentOverflow)
    /// if combining exponents overflows.
    /// Returns [`UnitError::ScaleOutOfRange`] if the scale relative to the
    /// base units overflows or underflows.
    pub(crate) fn new(name: &str, definition: &DeltaUnit, scale: Scale) -> Result<Self, UnitError> {
        let base = definition.base()?;
        let total = definition
            .scale()?
            .checked_mul(scale)
            .ok_or_else(|| UnitError::ScaleOutOfRange { unit: name.into() })?;
        Ok(Self { scale: total, base })
    }

    /// The scale of the expansion to base units.
    pub(crate) fn scale(&self) -> Scale {
        self.scale
    }

    /// The base units of the expansion.
    pub(crate) fn base(&self) -> &DeltaUnit {
        &self.base
    }
}

#[derive(Debug)]
pub(crate) enum AtomKind {
    /// The unit of one irreducible dimension.
    /// The frame every other unit of that dimension is measured in.
    Base,
    /// `1 self = scale × base`.
    Derived(Box<Expansion>),
}

#[derive(Debug)]
pub(crate) struct AtomData {
    /// This atom's process-unique identity.
    pub id: UnitId,
    /// The registry that created this atom.
    pub registry_id: UnitRegistryId,
    /// Unit name.
    pub name: Box<str>,
    /// Unit symbol.
    pub symbol: Box<str>,
    /// Unit dimension.
    pub dimension: Dimension,
    /// Whether this unit is prefixable.
    pub prefixable: bool,
    /// Whether this is a base unit or a derived one.
    pub kind: AtomKind,
}

impl PartialEq for AtomData {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for AtomData {}

impl PartialOrd for AtomData {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for AtomData {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl Hash for AtomData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

pub(crate) type UnitAtom = Arc<AtomData>;

#[derive(Debug)]
pub(crate) struct PointData {
    /// This atom's process-unique identity.
    pub id: UnitId,
    /// The registry that created this atom.
    pub registry_id: UnitRegistryId,
    /// Unit name.
    pub name: Box<str>,
    /// Unit symbol.
    pub symbol: Box<str>,
    /// The unit of a difference between two readings.
    pub delta: DeltaUnit,
    /// The `delta` value at this unit's zero.
    pub offset: f64,
}

impl PartialEq for PointData {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for PointData {}

impl Hash for PointData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

pub(crate) type PointAtom = Arc<PointData>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{assert_scale_close, errors_match, make_atom, make_derived_atom};
    use inchworm_dimensions::DimRegistry;

    fn meter() -> (UnitRegistryId, DeltaUnit) {
        let mut dims = DimRegistry::new("test-reg");
        let length = dims.add_base("length", None).unwrap();
        let id = UnitRegistryId::next();
        let meter = make_atom(id, "m", length);
        (id, DeltaUnit::from_atom(&meter))
    }

    mod expansion_new {
        use super::*;

        #[test]
        fn base_definition_expands_to_scale_and_base() {
            let (_, m) = meter();
            let expansion = Expansion::new("km", &m, Scale::new(1e3).unwrap()).unwrap();
            assert_scale_close(expansion.scale().value(), 1e3);
            assert_eq!(expansion.base(), &m);
        }

        #[test]
        fn composes_through_derived_definition() {
            let (id, m) = meter();
            let km =
                DeltaUnit::from_atom(&make_derived_atom(id, "km", &m, Scale::new(1e3).unwrap()));
            let expansion = Expansion::new("Mm", &km, Scale::new(1e3).unwrap()).unwrap();
            assert_scale_close(expansion.scale().value(), 1e6);
            assert_eq!(expansion.base(), &m);
        }

        #[test]
        fn rejects_out_of_range_total() {
            let (id, m) = meter();
            let huge = DeltaUnit::from_atom(&make_derived_atom(
                id,
                "huge",
                &m,
                Scale::new(1e200).unwrap(),
            ));
            let actual = Expansion::new("huger", &huge, Scale::new(1e200).unwrap()).unwrap_err();
            let expected = UnitError::ScaleOutOfRange {
                unit: "huger".into(),
            };
            assert!(errors_match(&actual, &expected));
        }
    }
}
